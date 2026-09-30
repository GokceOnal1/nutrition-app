//! Environment-based configuration, loaded once at startup.
//!
//! All values are required. Missing or malformed values are treated as a
//! startup error (`expect`/`panic` are acceptable here per `CLAUDE.md`); we
//! never want the server to come up half-configured.

use std::{net::SocketAddr, str::FromStr};

use tower_sessions::cookie::Key;

/// The owner's preferred unit for displaying body mass. Storage stays in kg
/// regardless (see `CLAUDE.md` invariants); this only affects presentation,
/// added later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassUnit {
    Kg,
    Lb,
}

impl FromStr for MassUnit {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "kg" => Ok(Self::Kg),
            "lb" => Ok(Self::Lb),
            other => Err(format!(
                "DISPLAY_MASS_UNIT must be `kg` or `lb`, got `{other}`"
            )),
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub user_tz: chrono_tz::Tz,
    pub display_mass_unit: MassUnit,
    pub app_password_hash: String,
    pub session_key: Key,
    /// Bearer token for the future `/ingest/*` endpoints (M5). Validated at
    /// startup so a missing value fails fast rather than at first use; not
    /// read anywhere yet, and never logged since it's a secret.
    #[allow(dead_code)]
    pub ingest_token: String,
}

impl Config {
    /// Reads and validates every configuration value from the process
    /// environment. Call `dotenvy::dotenv()` before this if a `.env` file
    /// should be loaded first.
    pub fn from_env() -> Self {
        let bind_addr = required("BIND_ADDR")
            .parse::<SocketAddr>()
            .expect("BIND_ADDR must be a valid socket address, e.g. 127.0.0.1:3000");

        let database_url = required("DATABASE_URL");

        let user_tz = required("USER_TZ")
            .parse::<chrono_tz::Tz>()
            .expect("USER_TZ must be a valid IANA time zone name, e.g. America/New_York");

        let display_mass_unit = required("DISPLAY_MASS_UNIT")
            .parse::<MassUnit>()
            .expect("invalid DISPLAY_MASS_UNIT");

        let app_password_hash = required("APP_PASSWORD_HASH");

        let session_secret_hex = required("SESSION_SECRET");
        let session_secret = decode_hex(&session_secret_hex)
            .expect("SESSION_SECRET must be a hex string (see .env.example)");
        let session_key = Key::try_from(session_secret.as_slice()).expect(
            "SESSION_SECRET must decode to at least 64 bytes; generate one with `openssl rand -hex 64`",
        );

        let ingest_token = required("INGEST_TOKEN");

        Self {
            bind_addr,
            database_url,
            user_tz,
            display_mass_unit,
            app_password_hash,
            session_key,
            ingest_token,
        }
    }
}

fn required(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("missing required environment variable {key}"))
}

fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        return Err("hex string must have an even number of characters".to_string());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}
