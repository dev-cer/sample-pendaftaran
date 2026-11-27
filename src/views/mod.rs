//! View: struct template Askama. Setiap struct = satu halaman atau potongan HTMX.
//! File HTML-nya ada di folder `templates/`.

use crate::config::Config;
use crate::models::pendaftar::{FormPendaftaran, Kesalahan, Pendaftar};
use crate::models::sekolah::{Jalur, Sekolah, DAFTAR_SEKOLAH};
use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// Data yang tampil di kerangka halaman (header, footer, lencana versi).
#[derive(Clone)]
pub struct AppInfo {
    pub version: String,
    pub env: String,
    pub instansi: String,
    pub tahun_ajaran: String,
    pub pengumuman: String,
    pub pod: String,
    pub db_user: String,
}

impl From<&Config> for AppInfo {
    fn from(c: &Config) -> Self {
        Self {
            version: c.app_version.clone(),
            env: c.app_env.clone(),
            instansi: c.instansi.clone(),
            tahun_ajaran: c.tahun_ajaran.clone(),
            pengumuman: c.pengumuman.clone(),
            pod: c.pod_name.clone(),
            db_user: c.db_user.clone(),
        }
    }
}

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeView {
    pub app: AppInfo,
    pub sekolah: &'static [Sekolah],
    pub total: u64,
    pub terakhir: Vec<Pendaftar>,
}

impl HomeView {
    pub fn new(app: AppInfo, total: u64, terakhir: Vec<Pendaftar>) -> Self {
        Self { app, sekolah: DAFTAR_SEKOLAH, total, terakhir }
    }
}

/// Halaman formulir lengkap (juga dipakai tanpa JavaScript).
#[derive(Template)]
#[template(path = "daftar.html")]
pub struct DaftarView {
    pub app: AppInfo,
    pub f: FormView,
    pub hasil: Option<Pendaftar>,
}

/// Isi formulir + pesan kesalahan; dipakai halaman penuh dan potongan HTMX.
pub struct FormView {
    pub nama: String,
    pub nisn: String,
    pub sekolah_dipilih: String,
    pub jalur_dipilih: String,
    pub err: Kesalahan,
    pub sekolah: &'static [Sekolah],
    pub jalur: [Jalur; 4],
}

impl FormView {
    pub fn kosong() -> Self {
        Self::dari(&FormPendaftaran::default(), Kesalahan::default())
    }

    pub fn dari(f: &FormPendaftaran, err: Kesalahan) -> Self {
        Self {
            nama: f.nama.clone(),
            nisn: f.nisn.clone(),
            sekolah_dipilih: f.sekolah.clone(),
            jalur_dipilih: f.jalur.clone(),
            err,
            sekolah: DAFTAR_SEKOLAH,
            jalur: Jalur::SEMUA,
        }
    }
}

/// Potongan HTMX: formulir dengan pesan kesalahan.
#[derive(Template)]
#[template(path = "partials/form.html")]
pub struct FormPartial {
    pub f: FormView,
}

/// Potongan HTMX: bukti pendaftaran.
#[derive(Template)]
#[template(path = "partials/hasil.html")]
pub struct HasilPartial {
    pub app: AppInfo,
    pub p: Pendaftar,
}

/// Render template jadi respons HTML; galat template jadi 500.
pub fn render<T: Template>(t: T) -> Response {
    match t.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("galat template: {e}")).into_response(),
    }
}
