//! View ppdb-web: struct template Askama; HTML-nya di folder `templates/`.

use crate::models::api::{FormPendaftaran, Jalur, Pendaftar, Sekolah};
use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use std::collections::BTreeMap;

/// Data kerangka halaman. `api_pod` dan `db` diisi per request dari jawaban ppdb-api.
#[derive(Clone, Default)]
pub struct AppInfo {
    pub version: String,
    pub env: String,
    pub instansi: String,
    pub tahun_ajaran: String,
    pub pengumuman: String,
    pub pod: String,
    pub api_pod: String,
    pub db: String,
}

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeView {
    pub app: AppInfo,
    pub sekolah: Vec<Sekolah>,
    pub total: u64,
    pub terakhir: Vec<Pendaftar>,
    pub gangguan: Option<String>,
}

pub struct FormView {
    pub nama: String,
    pub nisn: String,
    pub sekolah_dipilih: String,
    pub jalur_dipilih: String,
    pub err: BTreeMap<String, String>,
    pub gangguan: Option<String>,
    pub sekolah: Vec<Sekolah>,
    pub jalur: Vec<Jalur>,
}

impl FormView {
    pub fn new(f: &FormPendaftaran, sekolah: Vec<Sekolah>, jalur: Vec<Jalur>) -> Self {
        Self {
            nama: f.nama.clone(),
            nisn: f.nisn.clone(),
            sekolah_dipilih: f.sekolah.clone(),
            jalur_dipilih: f.jalur.clone(),
            err: BTreeMap::new(),
            gangguan: None,
            sekolah,
            jalur,
        }
    }
}

#[derive(Template)]
#[template(path = "daftar.html")]
pub struct DaftarView {
    pub app: AppInfo,
    pub f: FormView,
    pub hasil: Option<Pendaftar>,
}

#[derive(Template)]
#[template(path = "partials/form.html")]
pub struct FormPartial {
    pub f: FormView,
}

#[derive(Template)]
#[template(path = "partials/hasil.html")]
pub struct HasilPartial {
    pub app: AppInfo,
    pub p: Pendaftar,
}

pub fn render<T: Template>(t: T) -> Response {
    match t.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("galat template: {e}")).into_response(),
    }
}
