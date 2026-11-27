//! ppdb-api: REST API pendaftaran PPDB (microservice 2 dari 2), pola MVC.
//!
//!   Model      src/models/       pendaftar (validasi, verifikasi, penyimpanan), sekolah
//!   View       src/views/        bentuk respons JSON
//!   Controller src/controllers/  handler REST

mod controllers;
mod models;
mod views;

use axum::routing::{get, post};
use axum::Router;
use models::pendaftar::PendaftarStore;
use models::rekap::Rekap;
use std::env;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

pub struct Config {
    pub versi: String,
    pub environment: String,
    pub pod: String,
    pub tahun_ajaran: String,
    /// Username database dari Secret `db-credentials` (Vault); password tidak pernah dipakai di respons.
    pub db_user: String,
    pub iterasi: u32,
    /// Batas pendaftaran yang diproses bersamaan per pod; lebih dari itu dijawab 503 "server sibuk".
    pub max_antrean: usize,
    /// Zona waktu tampilan dashboard (jam dari UTC), default WIB.
    pub tz_jam: i32,
    pub port: u16,
}

fn var(k: &str, d: &str) -> String {
    env::var(k).ok().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| d.to_string())
}

impl Config {
    fn from_env() -> Self {
        Self {
            versi: var("APP_VERSION", "v1"),
            environment: var("APP_ENV", "lokal"),
            pod: var("POD_NAME", "api-lokal"),
            tahun_ajaran: var("TAHUN_AJARAN", "2027/2028"),
            db_user: var("DB_USER", ""),
            iterasi: var("VERIFIKASI_ITERASI", "20000").parse().unwrap_or(20000),
            max_antrean: var("MAX_ANTREAN", "32").parse().unwrap_or(32),
            tz_jam: var("TZ_JAM", "7").parse().unwrap_or(7),
            port: var("PORT", "8081").parse().unwrap_or(8081),
        }
    }
}

pub struct AppState {
    pub cfg: Config,
    pub store: PendaftarStore,
    pub rekap: Arc<Rekap>,
    pub antrean: AtomicUsize,
    pub ready: AtomicBool,
}

/// URL Valkey dari VALKEY_HOST (+ VALKEY_PASSWORD dari Secret Vault). Kosong = rekap nonaktif.
fn valkey_url() -> Option<String> {
    let host = var("VALKEY_HOST", "");
    if host.is_empty() {
        return None;
    }
    let pw = var("VALKEY_PASSWORD", "");
    let port = var("VALKEY_PORT", "6379");
    Some(if pw.is_empty() { format!("redis://{host}:{port}/") } else { format!("redis://:{pw}@{host}:{port}/") })
}

pub fn router(st: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/referensi", get(controllers::referensi))
        .route("/api/pendaftaran", post(controllers::daftar))
        .route("/api/statistik", get(controllers::statistik))
        .route("/api/dashboard", get(controllers::dashboard))
        .route("/healthz", get(controllers::healthz))
        .route("/readyz", get(controllers::readyz))
        .route("/info", get(controllers::info))
        .with_state(st)
}

#[tokio::main]
async fn main() {
    let cfg = Config::from_env();
    let addr = format!("0.0.0.0:{}", cfg.port);
    let rekap = Arc::new(Rekap::new(cfg.pod.clone()));
    rekap.mulai(valkey_url());
    let st = Arc::new(AppState { cfg, store: PendaftarStore::default(), rekap, antrean: AtomicUsize::new(0), ready: AtomicBool::new(true) });
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("gagal bind port");
    println!("ppdb-api {} ({}) di {addr}, pod {}, antrean maks {}", st.cfg.versi, st.cfg.environment, st.cfg.pod, st.cfg.max_antrean);
    let s2 = st.clone();
    axum::serve(listener, router(st))
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
