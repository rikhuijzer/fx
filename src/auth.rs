use crate::serve::ServerContext;
use base64::prelude::*;
use axum_extra::extract::CookieJar;
use base64::prelude::BASE64_STANDARD;
use crate::serve::LoginForm;
use aes_gcm_siv::AeadCore;
use aes_gcm_siv::aead::Aead;
use aes_gcm_siv::aead::Nonce;
use axum_extra::extract::cookie::Cookie;
use aes_gcm_siv::Aes256GcmSiv;
use aes_gcm_siv::KeyInit;
use aes_gcm_siv::aead::rand_core::OsRng;
use subtle::ConstantTimeEq;
use chrono::Utc;

fn constant_time_str_eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

fn verify_login(ctx: &ServerContext, form: &LoginForm) -> bool {
    let admin_password = match &ctx.args.admin_password {
        Some(admin_password) => admin_password,
        None => {
            tracing::warn!("admin password not set");
            return false
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

fn encrypt_login(password: &str) -> String {
    let plaintext = Utc::now().to_rfc3339();
    let key = Aes256GcmSiv::new_from_slice(password.as_bytes()).unwrap();
    // Nonce should be unique per message.
    let nonce = Aes256GcmSiv::generate_nonce(&mut OsRng);
    let ciphertext = key.encrypt(&nonce, plaintext.as_bytes()).unwrap();
    let ciphertext = base64_encode(&ciphertext);
    let nonce = base64_encode(&nonce);
    format!("{nonce}{ciphertext}")
}

fn decrypt_login(password: &str, auth: &str) -> Option<String> {
    let key = Aes256GcmSiv::new_from_slice(password.as_bytes()).unwrap();
    let nonce = base64_decode(&auth[..24]);
    let nonce = nonce.as_bytes();
    let nonce = Nonce::<Aes256GcmSiv>::from_slice(nonce);
    let ciphertext = base64_decode(&auth[24..]);
    let plaintext = key.decrypt(&nonce, ciphertext.as_bytes()).unwrap();
    String::from_utf8(plaintext).ok()
}

#[test]
fn encryption_roundtrip() {
    let password = "password";
    let auth = encrypt_login(password);
    let plaintext = decrypt_login(password, &auth).unwrap();
    assert_eq!(plaintext, Utc::now().to_rfc3339());
}

pub fn handle_login(ctx: &ServerContext, form: &LoginForm, jar: CookieJar) -> Option<CookieJar> {
    if verify_login(ctx, form) {
        let ciphertext = encrypt_login(&form.password);
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
