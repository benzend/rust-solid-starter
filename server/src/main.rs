use std::{env, net::SocketAddr, time::Duration};

use server::{AppState, app};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Loads .env from this directory or any parent (the repo root in dev).
    // In Docker/production, real environment variables are used instead.
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "server=info,tower_http=info".into()),
        )
        .init();

    let database_url = env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL is not set. Copy .env.example to .env (or run `make setup`).")?;
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let static_dir = env::var("STATIC_DIR").unwrap_or_else(|_| "../web/dist".into());

    let db = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await?;

    // Migrations in ./migrations are compiled into the binary and applied on start.
    sqlx::migrate!().run(&db).await?;

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on http://localhost:{port}");

    axum::serve(listener, app(AppState { db }, &static_dir))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
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
