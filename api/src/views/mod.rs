//! View: bentuk JSON yang dikirim ke ppdb-web. Model tidak pernah diserialisasi langsung,
//! sehingga NISN asli tidak pernah keluar dari service ini.

use crate::models::pendaftar::Pendaftar;
use crate::models::sekolah::{Jalur, DAFTAR_SEKOLAH};
use serde::Serialize;

/// Identitas pod API yang menjawab: memperlihatkan load balancing antar replika.
#[derive(Serialize)]
pub struct Meta {
    pub service: &'static str,
    pub versi: String,
    pub environment: String,
    pub pod: String,
}

#[derive(Serialize)]
pub struct SekolahJson {
    pub kode: &'static str,
    pub nama: &'static str,
    pub kecamatan: &'static str,
    pub kuota: u32,
}

#[derive(Serialize)]
pub struct JalurJson {
    pub kode: &'static str,
    pub label: &'static str,
}

#[derive(Serialize)]
pub struct Referensi {
    pub sekolah: Vec<SekolahJson>,
    pub jalur: Vec<JalurJson>,
    pub meta: Meta,
}

pub fn referensi(meta: Meta) -> Referensi {
    Referensi {
        sekolah: DAFTAR_SEKOLAH
            .iter()
            .map(|s| SekolahJson { kode: s.kode, nama: s.nama, kecamatan: s.kecamatan, kuota: s.kuota })
            .collect(),
        jalur: Jalur::SEMUA.iter().map(|j| JalurJson { kode: j.kode(), label: j.label() }).collect(),
        meta,
    }
}

#[derive(Serialize)]
pub struct PendaftarJson {
    pub nomor: String,
    pub nama: String,
    pub nisn: String,
    pub sekolah: &'static str,
    pub sekolah_kode: &'static str,
    pub jalur: &'static str,
    pub waktu: String,
    pub kode_verifikasi: String,
}

impl From<&Pendaftar> for PendaftarJson {
    fn from(p: &Pendaftar) -> Self {
        Self {
            nomor: p.nomor.clone(),
            nama: p.nama.clone(),
            nisn: p.nisn_tersamar.clone(),
            sekolah: p.sekolah.nama,
            sekolah_kode: p.sekolah.kode,
            jalur: p.jalur.label(),
            waktu: p.waktu.clone(),
            kode_verifikasi: p.kode_verifikasi.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct Diterima {
    pub pendaftar: PendaftarJson,
    pub meta: Meta,
}

#[derive(Serialize)]
pub struct Ditolak {
    pub kesalahan: std::collections::BTreeMap<&'static str, &'static str>,
    pub meta: Meta,
}

#[derive(Serialize)]
pub struct Statistik {
    pub total_pod_ini: u64,
    pub terakhir: Vec<PendaftarJson>,
    /// Username database dari Vault (tanpa password); kosong = belum terhubung.
    pub database: String,
    pub meta: Meta,
}
