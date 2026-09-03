//! Thin binary entry point of the public alo-sites service: read config
//! from the environment, open the narrow public store door, serve. All
//! logic lives in the library (`alo_sites::serve`). This process is the
//! anonymous, internet-facing side of alo Sites — it runs no migrations
//! (the authenticated `alo-jmap` service owns the schema) and can be
//! stopped at any time without touching tenant data.
//!
//! Environment: see [`alo_sites::serve::config`].

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::ExitCode;

use alo_sites::serve::config::{BLOB_MAX_BYTES, DEFAULT_ADDR};
use alo_sites::serve::{AppState, ServeConfig, app};
use alo_store::{BlobStore, SitePublicStore};

#[tokio::main]
async fn main() -> ExitCode {
    // `--healthcheck` TCP-probes the bind address over loopback and exits, so
    // a container runtime can ask whether this process is listening without a
    // shell, a client or a certificate in the image.
    //
    // It reads the address and nothing else. A probe that also parsed the
    // database URL and the analytics secret would report the process unhealthy
    // for reasons that have nothing to do with whether it is serving — and a
    // healthcheck that fails for the wrong reason gets a working container
    // restarted in a loop.
    if std::env::args().nth(1).as_deref() == Some("--healthcheck") {
        return match healthcheck().await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("alo-sites: {error}");
                ExitCode::FAILURE
            }
        };
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "fatal");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServeConfig::from_env()?;
    let blobs = BlobStore::local(&config.blob_dir, BLOB_MAX_BYTES)
        .map_err(|_| "cannot open the blob directory")?;
    let store = SitePublicStore::connect(&config.database_url, blobs)
        .await
        .map_err(|_| "cannot connect to the database")?;
    let payments: Option<std::sync::Arc<dyn alo_store::SitePaymentProvider>> = match config.payments
    {
        alo_sites::serve::config::PaymentsChoice::None => None,
        alo_sites::serve::config::PaymentsChoice::Fixture => {
            tracing::warn!(
                "ticket-shop payments run on the in-memory fixture provider: \
                     no money moves, and payments are forgotten on restart"
            );
            Some(std::sync::Arc::new(alo_store::FixtureSitePayments::new()))
        }
    };
    let state = AppState::with_payments(
        store,
        config.sites_domain.clone(),
        config.analytics_secret.as_bytes(),
        payments,
    );
    let listener = tokio::net::TcpListener::bind(config.addr).await?;
    tracing::info!(addr = %config.addr, sites_domain = %config.sites_domain, "alo-sites listening");
    // ConnectInfo gives the form rate limiter a per-peer fallback key when
    // the service is reached without a proxy's X-Forwarded-For in front.
    axum::serve(
        listener,
        app(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

/// Probe the address this service binds, over loopback, and say whether
/// something is listening there.
///
/// `ALO_SITES_ADDR` is read the same way [`ServeConfig::from_env`] reads it, so
/// the probe follows the address the service was actually told to use rather
/// than assuming the default.
async fn healthcheck() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::var("ALO_SITES_ADDR")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_ADDR.to_owned());
    let addr: SocketAddr = addr
        .parse()
        .map_err(|_| format!("healthcheck: `{addr}` is not a host:port address"))?;
    // A service bound to 0.0.0.0 is not reachable at that address from inside
    // its own container; probe the loopback interface on the same port.
    let probe = if addr.ip().is_unspecified() {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), addr.port())
    } else {
        addr
    };
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::net::TcpStream::connect(probe),
    )
    .await
    .map_err(|_| "healthcheck: connection timed out")?
    .map_err(|error| format!("healthcheck: {error}"))?;
    Ok(())
}
