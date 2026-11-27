//! Portal PPDB Online (sampel) - Rust full-stack dengan pola MVC.
//!
//!   Model      src/models/       data pendaftar, sekolah, validasi, verifikasi
//!   View       src/views/        struct template Askama + templates/*.html (Tailwind, HTMX)
//!   Controller src/controllers/  handler Axum: beranda, pendaftaran, kesehatan

mod config;
mod controllers;
mod models;
mod views;

use axum::routing::get;
use axum::Router;
use config::Config;
use models::pendaftar::PendaftarStore;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tower_http::services::ServeDir;
use views::AppInfo;

pub struct AppState {
    pub cfg: Config,
    pub app: AppInfo,
    pub store: PendaftarStore,
    pub ready: AtomicBool,
}

pub fn router(state: Arc<AppState>) -> Router {
    let static_dir = state.cfg.static_dir.clone();
    Router::new()
        .route("/", get(controllers::home::index))
        .route("/daftar", get(controllers::pendaftaran::form).post(controllers::pendaftaran::kirim))
        .route("/healthz", get(controllers::health::healthz))
        .route("/readyz", get(controllers::health::readyz))
        .route("/info", get(controllers::health::info))
        .nest_service("/static", ServeDir::new(static_dir))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    let cfg = Config::from_env();
    let addr = format!("0.0.0.0:{}", cfg.port);
    let state = Arc::new(AppState { app: AppInfo::from(&cfg), cfg, store: PendaftarStore::new(), ready: AtomicBool::new(true) });

    let listener = tokio::net::TcpListener::bind(&addr).await.expect("gagal bind port");
    println!(
        "Portal PPDB Online {} ({}) di {addr}, pod {}",
        state.cfg.app_version, state.cfg.app_env, state.cfg.pod_name
    );

    let st = state.clone();
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            // Tandai tidak siap, beri waktu load balancer melepas pod, lalu berhenti.
            st.ready.store(false, Ordering::Relaxed);
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            println!("berhenti dengan rapi");
        })
        .await
        .expect("server berhenti dengan galat");
}

async fn shutdown_signal() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.ok(); };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
}
