//! Controller ppdb-web: ambil data dari ppdb-api (Model), pilih View.
//! Permintaan HTMX (header `HX-Request`) dibalas potongan HTML saja.

use crate::models::api::{FormPendaftaran, HasilDaftar};
use crate::views::{render, AppInfo, DaftarView, FormPartial, FormView, HasilPartial, HomeView};
use crate::AppState;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Form;
use std::sync::atomic::Ordering;
use std::sync::Arc;

fn htmx(h: &HeaderMap) -> bool {
    h.contains_key("hx-request")
}

/// GET /: beranda dengan kuota sekolah dan statistik pod API.
pub async fn home(State(st): State<Arc<AppState>>) -> Response {
    let mut app = st.app.clone();
    let (refr, stat) = tokio::join!(st.api.referensi(), st.api.statistik());
    let gangguan = refr.as_ref().err().or(stat.as_ref().err()).cloned();
    let (total, terakhir) = match &stat {
        Ok(s) => {
            app.api_pod = s.meta.pod.clone();
            app.db = s.database.clone();
            (s.total_pod_ini, s.terakhir.clone())
        }
        Err(_) => (0, vec![]),
    };
    let sekolah = refr.map(|r| r.sekolah).unwrap_or_default();
    render(HomeView { app, sekolah, total, terakhir, gangguan })
}

/// GET /daftar: formulir kosong.
pub async fn form(State(st): State<Arc<AppState>>) -> Response {
    let mut app = st.app.clone();
    let f = match st.api.referensi().await {
        Ok(r) => {
            app.api_pod = r.meta.pod.clone();
            FormView::new(&FormPendaftaran::default(), r.sekolah, r.jalur)
        }
        Err(e) => {
            let mut f = FormView::new(&FormPendaftaran::default(), vec![], vec![]);
            f.gangguan = Some(e);
            f
        }
    };
    render(DaftarView { app, f, hasil: None })
}

/// POST /daftar: teruskan ke ppdb-api, tampilkan bukti atau kesalahan.
pub async fn kirim(State(st): State<Arc<AppState>>, headers: HeaderMap, Form(input): Form<FormPendaftaran>) -> Response {
    let mut app = st.app.clone();
    match st.api.daftar(&input).await {
        HasilDaftar::Diterima(p, meta) => {
            app.api_pod = meta.pod;
            if htmx(&headers) {
                render(HasilPartial { app, p })
            } else {
                let f = FormView::new(&FormPendaftaran::default(), vec![], vec![]);
                render(DaftarView { app, f, hasil: Some(p) })
            }
        }
        hasil => {
            let refr = st.api.referensi().await.ok();
            let (sekolah, jalur) = refr.map(|r| (r.sekolah, r.jalur)).unwrap_or_default();
            let mut f = FormView::new(&input, sekolah, jalur);
            let status = match hasil {
                HasilDaftar::Ditolak(err) => {
                    f.err = err;
                    StatusCode::OK
                }
                HasilDaftar::Gangguan(e) => {
                    f.gangguan = Some(e);
                    StatusCode::SERVICE_UNAVAILABLE
                }
                HasilDaftar::Diterima(..) => unreachable!(),
            };
            let resp = if htmx(&headers) { render(FormPartial { f }) } else { render(DaftarView { app, f, hasil: None }) };
            // HTMX hanya menukar konten untuk 2xx, jadi gangguan tetap dikirim 200 ke HTMX.
            if htmx(&headers) { resp } else { (status, resp).into_response() }
        }
    }
}

pub async fn healthz() -> &'static str {
    "ok"
}

pub async fn readyz(State(st): State<Arc<AppState>>) -> Response {
    if st.ready.load(Ordering::Relaxed) {
        (StatusCode::OK, "siap").into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "berhenti").into_response()
    }
}

pub fn info(app: &AppInfo) -> serde_json::Value {
    serde_json::json!({ "service": "ppdb-web", "versi": app.version, "environment": app.env, "pod": app.pod })
}
