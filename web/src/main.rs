//! ppdb-web: halaman Portal PPDB Online (microservice 1 dari 2), pola MVC.
//!
//!   Model      src/models/api.rs   klien ppdb-api (sumber data)
//!   View       src/views/ + templates/   Askama + HTMX + Tailwind
//!   Controller src/controllers/    handler halaman

mod controllers;
mod models;
mod views;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use models::api::ApiClient;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tower_http::services::ServeDir;
use views::AppInfo;

pub struct AppState {
    pub app: AppInfo,
    pub api: ApiClient,
    pub ready: AtomicBool,
}

fn var(k: &str, d: &str) -> String {
    env::var(k).ok().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| d.to_string())
}

pub fn router(st: Arc<AppState>, static_dir: &str) -> Router {
    Router::new()
        .route("/", get(controllers::home))
        .route("/daftar", get(controllers::form).post(controllers::kirim))
        .route("/healthz", get(controllers::healthz))
        .route("/readyz", get(controllers::readyz))
        .route("/info", get(|State(st): State<Arc<AppState>>| async move { Json(controllers::info(&st.app)) }))
        .nest_service("/static", ServeDir::new(static_dir))
        .with_state(st)
}

#[tokio::main]
async fn main() {
    let app = AppInfo {
        version: var("APP_VERSION", "v1"),
        env: var("APP_ENV", "lokal"),
        instansi: var("INSTANSI", "Dinas Pendidikan Kota Nusantara"),
        tahun_ajaran: var("TAHUN_AJARAN", "2027/2028"),
        pengumuman: var("PENGUMUMAN", ""),
        pod: var("POD_NAME", "web-lokal"),
        ..Default::default()
    };
    let api_url = var("API_URL", "http://localhost:8081");
    let addr = format!("0.0.0.0:{}", var("PORT", "8080"));
    let static_dir = var("STATIC_DIR", "static");
    let st = Arc::new(AppState { app, api: ApiClient::new(&api_url), ready: AtomicBool::new(true) });
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("gagal bind port");
    println!("ppdb-web {} ({}) di {addr}, API {api_url}, pod {}", st.app.version, st.app.env, st.app.pod);
    let s2 = st.clone();
    axum::serve(listener, router(st, &static_dir))
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            s2.ready.store(false, Ordering::Relaxed);
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
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
