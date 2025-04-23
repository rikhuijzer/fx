//! Encryption and authentication.
//!
//! This was moved in a separate crate to speed up incremental compilation.
use aes_gcm_siv::AeadCore;
use aes_gcm_siv::Aes256GcmSiv;
use aes_gcm_siv::KeyInit;
use aes_gcm_siv::aead::Aead;
use aes_gcm_siv::aead::Nonce;
use aes_gcm_siv::aead::rand_core::OsRng;
use argon2::Argon2;
use argon2::password_hash::SaltString;
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::Cookie;
use chrono::NaiveDate;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use subtle::ConstantTimeEq;

fn constant_time_str_eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Login {
    pub username: Option<String>,
    pub password: Option<String>,
}

fn verify_login(actual: &Login, received: &Login) -> bool {
    let username_eq = match (&actual.username, &received.username) {
        (Some(a), Some(e)) => constant_time_str_eq(a, e),
        _ => false,
    };
    let password_eq = match (&actual.password, &received.password) {
        (Some(a), Some(e)) => constant_time_str_eq(a, e),
        _ => false,
    };
    username_eq && password_eq
}

struct Key {
    key: Aes256GcmSiv,
}

pub type Salt = [u8; 22];

impl Key {
    fn new(salt: &Salt, password: &str) -> Self {
        // Salt can be public because it does not help the attacker.
        // It is only used to defend against rainbow tables.
        let argon2 = Argon2::default();
        let mut key = [0u8; 32];
        argon2
            .hash_password_into(password.as_bytes(), salt, &mut key)
            .unwrap();
        Key {
            key: Aes256GcmSiv::new_from_slice(&key).unwrap(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Ciphertext {
    nonce: [u8; 12],
    ciphertext: Vec<u8>,
}

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

fn encrypt_login(salt: &Salt, password: &str) -> Ciphertext {
    let plaintext = today();
    let key = Key::new(salt, password);
    // Nonce should be unique per message.
    let nonce = Aes256GcmSiv::generate_nonce(&mut OsRng);
    let plaintext = plaintext.to_string();
    let ciphertext = key.key.encrypt(&nonce, plaintext.as_bytes()).unwrap();
    let nonce = nonce.as_slice();
    Ciphertext {
        nonce: nonce.try_into().unwrap(),
        ciphertext,
    }
}

fn decrypt_login(salt: &Salt, password: &str, auth: &Ciphertext) -> Option<String> {
    let key = Key::new(salt, password);
    let nonce = Nonce::<Aes256GcmSiv>::from_slice(&auth.nonce);
    let ciphertext = auth.ciphertext.as_slice();
    let plaintext = match key.key.decrypt(nonce, ciphertext) {
        Ok(plaintext) => plaintext,
        Err(e) => {
            // This can occur when the salt is incorrect. Should not happen in
            // production I think. Time will tell.
            println!("failed to decrypt login: {}", e);
            return None;
        }
    };
    String::from_utf8(plaintext).ok()
}

#[test]
fn encryption_roundtrip() {
    let salt = b"nblVMlxYtvt0rxo3BML3zw";
    let password = "password";
    let auth = encrypt_login(salt, password);
    let plaintext = decrypt_login(salt, password, &auth).unwrap();
    let today = today().to_string();
    assert_eq!(plaintext, today);
}

pub fn handle_logout(jar: CookieJar) -> CookieJar {
    jar.remove(Cookie::from("auth"))
}

const MAX_AGE_SEC: i64 = 2 * 60 * 60 * 24 * 7; // 2 weeks.

pub fn is_logged_in(salt: &Salt, login: &Login, jar: &CookieJar) -> bool {
    let cookie = jar.get("auth");
    match cookie {
        Some(cookie) => {
            let ciphertext = match serde_json::from_str(cookie.value()) {
                Ok(ciphertext) => ciphertext,
                Err(_) => {
                    return false;
                }
            };
            let key = match &login.password {
                Some(key) => key,
                None => {
                    tracing::warn!("admin password not set");
                    return false;
                }
            };
            let plaintext = match decrypt_login(salt, key, &ciphertext) {
                Some(plaintext) => plaintext,
                None => {
                    tracing::warn!(
                        "failed to decrypt login; probably a cookie that belongs to another salt"
                    );
                    return false;
                }
            };
            let date = NaiveDate::parse_from_str(&plaintext, "%Y-%m-%d").unwrap();
            today() <= date + chrono::Duration::days(MAX_AGE_SEC)
        }
        None => false,
    }
}

pub fn generate_salt() -> Salt {
    SaltString::generate(OsRng)
        .as_str()
        .as_bytes()
        .try_into()
        .unwrap()
}

pub fn handle_login(
    salt: &Salt,
    actual: &Login,
    received: &Login,
    jar: CookieJar,
) -> Option<CookieJar> {
    if verify_login(actual, received) {
        let password = match &received.password {
            Some(password) => password,
            None => {
                tracing::warn!("admin password not set");
                return None;
            }
        };
        let ciphertext = encrypt_login(salt, password);
        let ciphertext = serde_json::to_string(&ciphertext).unwrap();
        // Secure ensures only HTTPS scheme (except on localhost).
        // Without secure, a man-in-the-middle could steal the cookie.
        let cookie = format!("auth={ciphertext}; Max-Age={MAX_AGE_SEC}; Secure;");
        let cookie = Cookie::parse(cookie).unwrap();
        let updated_jar = jar.add(cookie);
        Some(updated_jar)
    } else {
        None
    }
}
