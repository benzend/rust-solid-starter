use std::{env, error::Error, net::SocketAddr, str::FromStr, time::Duration};

use server::{AppState, app};
use sqlx::postgres::PgPoolOptions;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Loads .env from this directory or any parent (the repo root in dev).
    // In Docker/production, real environment variables are used instead.
    dotenvy::dotenv().ok();

    let port: u16 = env_or("PORT", 3000)?;

    // `server --health-check` is what Docker's HEALTHCHECK runs.
    if env::args().nth(1).as_deref() == Some("--health-check") {
        return health_check(port).await;
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "server=info,tower_http=info".into()),
        )
        .init();

    let database_url = env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL is not set. Copy .env.example to .env (or run `make setup`).")?;
    let static_dir = env::var("STATIC_DIR").unwrap_or_else(|_| "../web/dist".into());
    let max_connections: u32 = env_or("DB_MAX_CONNECTIONS", 10)?;
    let request_timeout = Duration::from_secs(env_or("REQUEST_TIMEOUT_SECS", 10)?);

    let db = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await?;

    // Migrations in ./migrations are compiled into the binary and applied on start.
    sqlx::migrate!().run(&db).await?;

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on http://localhost:{port}");

    axum::serve(listener, app(AppState { db }, &static_dir, request_timeout))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Reads an optional env var, falling back to `default` when it's unset.
/// A value that is set but doesn't parse is an error rather than silently ignored.
fn env_or<T: FromStr>(name: &str, default: T) -> Result<T, String> {
    match env::var(name) {
        Ok(raw) => raw
            .parse()
            .map_err(|_| format!("{name} has an invalid value: {raw:?}")),
        Err(_) => Ok(default),
    }
}

/// Exits non-zero unless GET /api/health on this machine returns 200. The
/// distroless image has no curl, so the binary checks itself.
async fn health_check(port: u16) -> Result<(), Box<dyn Error>> {
    let probe = async {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await?;
        stream
            .write_all(b"GET /api/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await?;
        Ok::<_, std::io::Error>(response.starts_with(b"HTTP/1.1 200 "))
    };
    match tokio::time::timeout(Duration::from_secs(3), probe).await {
        Ok(Ok(true)) => Ok(()),
        Ok(Ok(false)) => Err("health check: /api/health did not return 200".into()),
        Ok(Err(err)) => Err(format!("health check: {err}").into()),
        Err(_) => Err("health check: timed out".into()),
    }
}

/// Finish in-flight requests on Ctrl+C or SIGTERM (what Docker sends on stop).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutting down");
}
