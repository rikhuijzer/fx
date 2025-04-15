use crate::serve::LoginForm;
use crate::serve::ServerContext;
use aes_gcm_siv::AeadCore;
use aes_gcm_siv::Aes256GcmSiv;
use aes_gcm_siv::KeyInit;
use aes_gcm_siv::aead::Aead;
use aes_gcm_siv::aead::Nonce;
use aes_gcm_siv::aead::rand_core::OsRng;
use argon2::Argon2;
use argon2::password_hash::PasswordHasher;
use argon2::password_hash::SaltString;
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::Cookie;
use base64::prelude::BASE64_STANDARD;
use base64::prelude::*;
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
    let password_eq = constant_time_str_eq(&form.password, &admin_password);
    username_eq && password_eq
}

fn base64_encode(data: &[u8]) -> String {
    BASE64_STANDARD.encode(data)
}

fn base64_decode(data: &str) -> String {
    let decoded = BASE64_STANDARD.decode(data).expect("invalid base64");
    String::from_utf8(decoded).expect("invalid utf-8")
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

fn encrypt_login(password: &str) -> Ciphertext {
    let plaintext = Utc::now().to_rfc3339();
    let key = Key::new(password);
    // Nonce should be unique per message.
    let nonce = Aes256GcmSiv::generate_nonce(&mut OsRng);
    let ciphertext = key.key.encrypt(&nonce, plaintext.as_bytes()).unwrap();
    let nonce = nonce.as_slice();
    Ciphertext {
        nonce: nonce.try_into().unwrap(),
        ciphertext: ciphertext,
    }
}

fn decrypt_login(password: &str, auth: &Ciphertext) -> Option<String> {
    let key = Key::new(password);
    let nonce = Nonce::<Aes256GcmSiv>::from_slice(&auth.nonce);
    let ciphertext = auth.ciphertext.as_slice();
    println!("nonce len: {}", nonce.len());
    println!("ciphertext len: {}", ciphertext.len());
    let plaintext = match key.key.decrypt(&nonce, ciphertext) {
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
    let today = Utc::now().date_naive();
    assert!(plaintext.contains(&today.to_string()));
}

pub fn handle_login(ctx: &ServerContext, form: &LoginForm, jar: CookieJar) -> Option<CookieJar> {
    if verify_login(ctx, form) {
        let ciphertext = encrypt_login(&form.password);
        let ciphertext = serde_json::to_string(&ciphertext).unwrap();
        // Secure ensures only HTTPS scheme (except on localhost).
        // Without this, a man-in-the-middle could steal the cookie.
        let age_sec = 2 * 60 * 60 * 24 * 7; // 2 weeks.
        let cookie = format!("auth={ciphertext}; Max-Age={age_sec}; Secure;");
        let cookie = Cookie::parse(cookie).unwrap();
        let updated_jar = jar.add(cookie);
        Some(updated_jar)
    } else {
        None
    }
}
