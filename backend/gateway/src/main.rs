mod routes;
mod state;

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use stock_storage::ArrowStore;
use tower_http::cors::{Any, CorsLayer};

use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "stock_gateway=info".parse().unwrap()),
        )
        .init();

    let base_dir = PathBuf::from(
        std::env::var("STOCK_DATA_DIR").unwrap_or_else(|_| "data".into())
    );

    let ashare_store = ArrowStore::new(base_dir.join("a_stock")).expect("ArrowStore a_stock init");
    let us_store     = ArrowStore::new(base_dir.join("us_stock")).expect("ArrowStore us_stock init");
    tracing::info!(
        "ArrowStore ready: a_stock={} keys, us_stock={} keys",
        ashare_store.cached_key_count(),
        us_store.cached_key_count(),
    );

    let state = Arc::new(AppState {
        ashare_store: Arc::new(ashare_store),
        store:        Arc::new(us_store),
        client:       reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("reqwest client"),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .nest("/api", routes::router())
        .layer(cors)
        .with_state(state);

    let addr = std::env::var("STOCK_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into());
    tracing::info!("stock-gateway listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
