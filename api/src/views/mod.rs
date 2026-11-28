//! View: bentuk JSON yang dikirim ke ppdb-web. Model tidak pernah diserialisasi langsung,
//! sehingga NISN asli tidak pernah keluar dari service ini.

use crate::models::pendaftar::Pendaftar;
use crate::models::rekap::Ringkasan;
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
    /// total tersimpan: dari database bila tersambung, selain itu hitungan pod ini
    pub total_pod_ini: u64,
    /// "database" atau "memori pod"
    pub sumber: &'static str,
    /// ppdb-store (Valkey) tersambung
    pub store: bool,
    pub terakhir: Vec<PendaftarJson>,
    /// user@host database (kredensial dari Vault, tanpa password); kosong = belum tersambung.
    pub database: String,
    pub meta: Meta,
}

#[derive(Serialize)]
pub struct Sibuk {
    pub pesan: &'static str,
    pub meta: Meta,
}

#[derive(Serialize)]
pub struct RekapSekolah {
    pub kode: &'static str,
    pub nama: &'static str,
    pub kecamatan: &'static str,
    pub kuota: u32,
    pub diterima: u64,
    pub gagal: u64,
}

#[derive(Serialize)]
pub struct RekapMenit {
    pub jam: String,
    pub diterima: u64,
    pub gagal: u64,
}

#[derive(Serialize)]
pub struct Dashboard {
    /// false = ppdb-store belum terhubung; angka di bawah kosong.
    pub tersambung: bool,
    pub sekolah: Vec<RekapSekolah>,
    pub total_diterima: u64,
    pub total_gagal: u64,
    pub per_menit: Vec<RekapMenit>,
    pub pod_api_aktif: u64,
    pub meta: Meta,
}

pub fn dashboard(r: Ringkasan, tz_jam: i32, meta: Meta) -> Dashboard {
    let sekolah: Vec<RekapSekolah> = DAFTAR_SEKOLAH
        .iter()
        .map(|s| RekapSekolah {
            kode: s.kode,
            nama: s.nama,
            kecamatan: s.kecamatan,
            kuota: s.kuota,
            diterima: r.diterima.get(s.kode).copied().unwrap_or(0),
            gagal: r.gagal.get(s.kode).copied().unwrap_or(0),
        })
        .collect();
    let tz = chrono::FixedOffset::east_opt(tz_jam * 3600).unwrap_or_else(|| chrono::FixedOffset::east_opt(0).unwrap());
    Dashboard {
        tersambung: r.tersambung,
        total_diterima: sekolah.iter().map(|s| s.diterima).sum(),
        total_gagal: sekolah.iter().map(|s| s.gagal).sum(),
        sekolah,
        per_menit: r
            .per_menit
            .iter()
            .map(|(m, d, g)| RekapMenit {
                jam: chrono::DateTime::from_timestamp(m * 60, 0)
                    .map(|t| t.with_timezone(&tz).format("%H:%M").to_string())
                    .unwrap_or_default(),
                diterima: *d,
                gagal: *g,
            })
            .collect(),
        pod_api_aktif: r.pod_aktif,
        meta,
    }
}
