//! Environment configuration of the public serving binary.
//!
//! - `DATABASE_URL` — the Postgres system of record (required; narrow public
//!   reads plus privacy-reduced analytics and contact-form writes).
//! - `SITES_DOMAIN` — the apex the service resolves subdomain hosts under,
//!   e.g. `alosites.example` makes `acme.alosites.example` serve the site
//!   with subdomain `acme` (required; the name is the contract used across
//!   `docs/design/sites.md`).
//! - `ALO_BLOB_DIR` — the on-disk blob backend published images are read
//!   from (required; the same directory the authenticated services write —
//!   the name matches `alo-jmap`'s).
//! - `ALO_SITES_ADDR` — internal bind address (default `0.0.0.0:8081`; TLS
//!   is terminated by the front proxy).
//! - `ALO_SITES_ANALYTICS_SECRET` — at least 32 bytes of deployment secret,
//!   used only for daily-separated visitor HMACs (required).
//! - `ALO_SITES_PAYMENTS` — the ticket shop's hosted-payment provider.
//!   Absent or empty means none: the shop stays visible and says online
//!   sales are not set up. `fixture` wires the deterministic in-memory
//!   provider (local development and tests only — it moves no money and
//!   forgets everything on restart). A live provider (Mollie/Adyen, per
//!   ADR 0041) is wired by a human with its own ADR and its own value here.

use std::net::SocketAddr;
use std::path::PathBuf;

use thiserror::Error;

/// Default internal bind (the front proxy terminates TLS and forwards here).
pub const DEFAULT_ADDR: &str = "0.0.0.0:8081";

/// Per-object byte ceiling on blob reads — the same ceiling `alo-jmap`
/// enforces on upload, re-checked here as defence against a tampered object.
pub const BLOB_MAX_BYTES: usize = 50 * 1024 * 1024;

/// Why configuration could not be read — printable to an operator as-is.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// A required variable is absent or empty.
    #[error("{0} is required")]
    Missing(&'static str),
    /// A variable is present but unusable.
    #[error("{name} is invalid: {reason}")]
    Invalid {
        /// The environment variable.
        name: &'static str,
        /// What is wrong with it.
        reason: String,
    },
}

/// Which hosted-payment provider the ticket shop runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentsChoice {
    /// No provider: the shop answers honestly that online sales are off.
    None,
    /// The deterministic in-memory provider — local development and tests.
    Fixture,
}

/// Everything the service needs from the environment.
#[derive(Debug, Clone)]
pub struct ServeConfig {
    /// Postgres connection string.
    pub database_url: String,
    /// The apex domain published sites are served under (lowercased).
    pub sites_domain: String,
    /// The on-disk blob backend published images are read from.
    pub blob_dir: PathBuf,
    /// The bind address.
    pub addr: SocketAddr,
    /// Secret for daily visitor HMACs; never written to analytics storage.
    pub analytics_secret: String,
    /// The ticket shop's hosted-payment provider.
    pub payments: PaymentsChoice,
}

impl ServeConfig {
    /// Reads and validates the configuration from the process environment.
    ///
    /// # Errors
    /// [`ConfigError`] naming the offending variable.
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = require("DATABASE_URL")?;
        let sites_domain = require("SITES_DOMAIN")?.to_ascii_lowercase();
        if !sites_domain
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
        {
            return Err(ConfigError::Invalid {
                name: "SITES_DOMAIN",
                reason: "must be a bare DNS name (letters, digits, dots, hyphens)".to_owned(),
            });
        }
        let blob_dir = PathBuf::from(require("ALO_BLOB_DIR")?);
        let addr = std::env::var("ALO_SITES_ADDR")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_ADDR.to_owned());
        let addr: SocketAddr = addr.parse().map_err(|_| ConfigError::Invalid {
            name: "ALO_SITES_ADDR",
            reason: format!("`{addr}` is not a host:port address"),
        })?;
        let analytics_secret = require("ALO_SITES_ANALYTICS_SECRET")?;
        if analytics_secret.len() < 32 {
            return Err(ConfigError::Invalid {
                name: "ALO_SITES_ANALYTICS_SECRET",
                reason: "must be at least 32 bytes".to_owned(),
            });
        }
        let payments = match std::env::var("ALO_SITES_PAYMENTS")
            .ok()
            .filter(|v| !v.is_empty())
            .as_deref()
        {
            None => PaymentsChoice::None,
            Some("fixture") => PaymentsChoice::Fixture,
            Some(other) => {
                return Err(ConfigError::Invalid {
                    name: "ALO_SITES_PAYMENTS",
                    reason: format!(
                        "`{other}` is not a provider this build knows; \
                         leave it unset, or use `fixture` for local development"
                    ),
                });
            }
        };
        Ok(Self {
            database_url,
            sites_domain,
            blob_dir,
            addr,
            analytics_secret,
            payments,
        })
    }
}

fn require(name: &'static str) -> Result<String, ConfigError> {
    std::env::var(name)
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or(ConfigError::Missing(name))
}
