use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, Request},
    routing::{delete, get, post},
    Router, ServiceExt,
};
use dotenv::dotenv;
use lock::SharedState;
use std::net::SocketAddr;
use tokio::sync::Mutex;
use tower::Layer;
use tower_http::{
    compression::CompressionLayer,
    normalize_path::NormalizePathLayer,
    services::{ServeDir, ServeFile},
};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};

#[macro_use]
extern crate lazy_static;

mod config;
mod csp;
mod health;
mod lock;
mod note;
mod status;
mod store;

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Initialise structured logging. Level is controlled by the VERBOSITY env var
    // (defaults to "warn"). Set VERBOSITY=info to see audit events.
    let filter = EnvFilter::new(format!("cryptgeon={},warn", *config::VERBOSITY));
    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();

    let shared_state = SharedState {
        locks: Arc::new(Mutex::new(HashMap::new())),
    };

    if !store::can_reach_redis() {
        tracing::error!("cannot reach redis");
        panic!("cannot reach redis");
    }

    let notes_routes = Router::new()
        .route("/", post(note::create))
        .route("/:id", delete(note::delete))
        .route("/:id", get(note::preview));
    let health_routes = Router::new().route("/live", get(health::report_health));
    let status_routes = Router::new().route("/status", get(status::get_status));
    let api_routes = Router::new()
        .nest("/notes", notes_routes)
        .nest("/", health_routes)
        .nest("/", status_routes);

    let index = format!("{}{}", config::FRONTEND_PATH.to_string(), "/index.html");
    let serve_dir =
        ServeDir::new(config::FRONTEND_PATH.to_string()).not_found_service(ServeFile::new(index));
    let app = Router::new()
        .nest("/api", api_routes)
        .fallback_service(serve_dir)
        // Disabled for now, as svelte inlines scripts
        // .layer(middleware::from_fn(csp::add_csp_header))
        .layer(DefaultBodyLimit::max(*config::LIMIT))
        .layer(
            CompressionLayer::new()
                .br(true)
                .deflate(true)
                .gzip(true)
                .zstd(true),
        )
        .with_state(shared_state);

    let app = NormalizePathLayer::trim_trailing_slash().layer(app);

    let listener = tokio::net::TcpListener::bind(config::LISTEN_ADDR.to_string())
        .await
        .unwrap();
    info!(addr = %listener.local_addr().unwrap(), "server listening");
    axum::serve(
        listener,
        ServiceExt::<Request>::into_make_service_with_connect_info::<SocketAddr>(app),
    )
    .await
    .unwrap();
}
