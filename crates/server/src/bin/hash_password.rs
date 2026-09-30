//! Dev tool: prints an argon2 PHC hash for a password, for pasting into
//! `.env` as `APP_PASSWORD_HASH`. Not part of the running server.
//!
//! Usage: `cargo run -p server --bin hash-password -- <password>`

use argon2::{password_hash::PasswordHasher, Argon2};

fn main() {
    let password = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: hash-password <password>");
        std::process::exit(1);
    });

    let hash = Argon2::default()
        .hash_password(password.as_bytes())
        .expect("failed to hash password")
        .to_string();

    println!("{hash}");
}
