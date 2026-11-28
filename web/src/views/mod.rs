//! View ppdb-web: struct template Askama; HTML-nya di folder `templates/`.

use crate::models::api::{Dashboard, FormPendaftaran, Jalur, Pendaftar, Sekolah};
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
    pub store: bool,
}

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeView {
    pub app: AppInfo,
    pub r: RekapView,
    pub total: u64,
    pub sumber: String,
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

/// Baris tabel per sekolah, sudah dihitung untuk template.
pub struct BarisSekolah {
    pub nama: String,
    pub kecamatan: String,
    pub kuota: u32,
    pub diterima: String,
    pub gagal: String,
    pub gagal_ada: bool,
    /// lebar bar hijau (persen dari total percobaan ke sekolah ini)
    pub persen_ok: u64,
}

pub struct BatangMenit {
    pub jam: String,
    pub diterima: u64,
    pub gagal: u64,
    pub tinggi_ok: u64,
    pub tinggi_gagal: u64,
}

/// Rekap siap tampil (dipakai beranda dan dashboard).
pub struct RekapView {
    pub tersambung: bool,
    pub total_diterima: String,
    pub total_gagal: String,
    pub persen_berhasil: String,
    pub pod_api_aktif: u64,
    pub sekolah: Vec<BarisSekolah>,
    pub menit: Vec<BatangMenit>,
    pub puncak_per_menit: String,
}

/// 12345 -> "12.345"
pub fn ribuan(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    out
}

impl RekapView {
    pub fn dari(d: &Dashboard) -> Self {
        let total = d.total_diterima + d.total_gagal;
        let persen = if total == 0 { "-".to_string() } else { format!("{:.1}%", d.total_diterima as f64 * 100.0 / total as f64).replace('.', ",") };
        let maks = d.per_menit.iter().map(|m| m.diterima + m.gagal).max().unwrap_or(0).max(1);
        let puncak = d.per_menit.iter().map(|m| m.diterima).max().unwrap_or(0);
        Self {
            tersambung: d.tersambung,
            total_diterima: ribuan(d.total_diterima),
            total_gagal: ribuan(d.total_gagal),
            persen_berhasil: persen,
            pod_api_aktif: d.pod_api_aktif,
            sekolah: d
                .sekolah
                .iter()
                .map(|s| {
                    let t = s.diterima + s.gagal;
                    BarisSekolah {
                        nama: s.nama.clone(),
                        kecamatan: s.kecamatan.clone(),
                        kuota: s.kuota,
                        diterima: ribuan(s.diterima),
                        gagal: ribuan(s.gagal),
                        gagal_ada: s.gagal > 0,
                        persen_ok: if t == 0 { 0 } else { s.diterima * 100 / t },
                    }
                })
                .collect(),
            menit: d
                .per_menit
                .iter()
                .map(|m| BatangMenit {
                    jam: m.jam.clone(),
                    diterima: m.diterima,
                    gagal: m.gagal,
                    tinggi_ok: m.diterima * 100 / maks,
                    tinggi_gagal: m.gagal * 100 / maks,
                })
                .collect(),
            puncak_per_menit: ribuan(puncak),
        }
    }
}

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardView {
    pub app: AppInfo,
    pub r: RekapView,
}

#[derive(Template)]
#[template(path = "partials/panel.html")]
pub struct PanelPartial {
    pub r: RekapView,
}

#[derive(Template)]
#[template(path = "partials/tabel_sekolah.html")]
pub struct TabelSekolahPartial {
    pub r: RekapView,
}

pub fn render<T: Template>(t: T) -> Response {
    match t.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("galat template: {e}")).into_response(),
    }
}
