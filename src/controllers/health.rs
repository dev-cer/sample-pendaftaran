//! Controller kesehatan untuk Kubernetes: liveness, readiness, dan info versi.

use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Liveness: proses hidup.
pub async fn healthz() -> &'static str {
    "ok"
}

/// Readiness: siap menerima trafik. Menjadi 503 saat pod sedang berhenti (SIGTERM),
/// sehingga load balancer berhenti mengirim request sebelum proses keluar.
pub async fn readyz(State(st): State<Arc<AppState>>) -> Response {
    if st.ready.load(Ordering::Relaxed) {
        (StatusCode::OK, "siap").into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "berhenti").into_response()
    }
}

/// Info rilis dalam JSON: dipakai skrip demo dan pengecekan cepat.
pub async fn info(State(st): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!({
        "aplikasi": "Portal PPDB Online",
        "versi": st.app.version,
        "environment": st.app.env,
        "pod": st.app.pod,
        "database": if st.app.db_user.is_empty() { None } else { Some(&st.app.db_user) },
        "pendaftar_dilayani_pod_ini": st.store.total(),
    }))
}
