use crate::serve::LoginForm;
use crate::serve::ServerContext;
use aes_gcm_siv::AeadCore;
use aes_gcm_siv::Aes256GcmSiv;
use aes_gcm_siv::KeyInit;
use aes_gcm_siv::aead::Aead;
use aes_gcm_siv::aead::Nonce;
use aes_gcm_siv::aead::rand_core::OsRng;
use argon2::Argon2;
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

fn verify_login(ctx: &ServerContext, form: &LoginForm) -> bool {
    let admin_password = match &ctx.args.admin_password {
        Some(admin_password) => admin_password,
        None => {
            tracing::warn!("admin password not set");
            return false;
        }
    };
    let username_eq = constant_time_str_eq(&form.username, &ctx.args.admin_username);
    let password_eq = constant_time_str_eq(&form.password, admin_password);
    username_eq && password_eq
}

struct Key {
    key: Aes256GcmSiv,
}

impl Key {
    fn new(password: &str) -> Self {
        // Salt can be public because it does not help the attacker.
        // It is only used to defend against rainbow tables.
        let salt = b"nblVMlxYtvt0rxo3BML3zw";
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

fn encrypt_login(password: &str) -> Ciphertext {
    let plaintext = today();
    let key = Key::new(password);
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

fn decrypt_login(password: &str, auth: &Ciphertext) -> Option<String> {
    let key = Key::new(password);
    let nonce = Nonce::<Aes256GcmSiv>::from_slice(&auth.nonce);
    let ciphertext = auth.ciphertext.as_slice();
    let plaintext = match key.key.decrypt(nonce, ciphertext) {
        Ok(plaintext) => plaintext,
        Err(e) => {
            println!("failed to decrypt login: {}", e);
            return None;
        }
    };
    String::from_utf8(plaintext).ok()
}

#[test]
fn encryption_roundtrip() {
    let password = "password";
    let auth = encrypt_login(password);
    let plaintext = decrypt_login(password, &auth).unwrap();
    let today = today().to_string();
    assert_eq!(plaintext, today);
}

pub fn handle_logout(jar: CookieJar) -> CookieJar {
    let cookie = Cookie::parse("auth=; Max-Age=0").unwrap();
    // Somehow, `jar.remove` throws an error, so we just override the cookie.
    jar.add(cookie)
}

const MAX_AGE_SEC: i64 = 2 * 60 * 60 * 24 * 7; // 2 weeks.

pub fn is_logged_in(ctx: &ServerContext, jar: &CookieJar) -> bool {
    let cookie = jar.get("auth");
    match cookie {
        Some(cookie) => {
            let ciphertext = match serde_json::from_str(cookie.value()) {
                Ok(ciphertext) => ciphertext,
                Err(_) => {
                    return false;
                }
            };
            let key = match &ctx.args.admin_password {
                Some(key) => key,
                None => {
                    tracing::warn!("admin password not set");
                    return false;
                }
            };
            let plaintext = decrypt_login(key, &ciphertext).unwrap();
            let date = NaiveDate::parse_from_str(&plaintext, "%Y-%m-%d").unwrap();
            today() <= date + chrono::Duration::days(MAX_AGE_SEC)
        }
        None => false,
    }
}

pub fn handle_login(ctx: &ServerContext, form: &LoginForm, jar: CookieJar) -> Option<CookieJar> {
    if verify_login(ctx, form) {
        let ciphertext = encrypt_login(&form.password);
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
