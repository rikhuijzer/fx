# fx

Federated (Micro)blogging.

## Secret

Token can store the date assuming a good KDF function is used.
HKDF has good legacy support and Argon2id is best in class.

encrypt(info, password) -> secret

```rust
Hkdf::<Sha256>::new(None, ikm)
let mut secret = [0u8; 42];
hkdf.expand(info, &mut secret).expect("length 42");
```

decrypt(secret, password) -> info
