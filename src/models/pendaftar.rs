//! Model Pendaftar: data formulir, validasi, verifikasi, dan penyimpanan.
//!
//! Penyimpanan sengaja di memori per pod (aplikasi sampel untuk demo platform).
//! Di produksi, ganti `PendaftarStore` dengan repository ke database, kredensialnya
//! dari Secret `db-credentials` (Vault).

use super::sekolah::{self, Jalur, Sekolah};
use rand::Rng;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::sync::RwLock;

/// Data mentah dari formulir.
#[derive(Debug, Default, Deserialize)]
pub struct FormPendaftaran {
    #[serde(default)]
    pub nama: String,
    #[serde(default)]
    pub nisn: String,
    #[serde(default)]
    pub sekolah: String,
    #[serde(default)]
    pub jalur: String,
}

/// Pendaftar yang sudah lolos validasi dan verifikasi.
#[derive(Clone, Debug)]
pub struct Pendaftar {
    pub nomor: String,
    pub nama: String,
    pub nisn_tersamar: String,
    pub sekolah: &'static Sekolah,
    pub jalur: Jalur,
    pub waktu: String,
    pub kode_verifikasi: String,
}

/// Kesalahan validasi per isian, ditampilkan kembali di formulir.
#[derive(Debug, Default)]
pub struct Kesalahan {
    pub nama: Option<&'static str>,
    pub nisn: Option<&'static str>,
    pub sekolah: Option<&'static str>,
    pub jalur: Option<&'static str>,
}

impl Kesalahan {
    pub fn ada(&self) -> bool {
        self.nama.is_some() || self.nisn.is_some() || self.sekolah.is_some() || self.jalur.is_some()
    }
}

impl FormPendaftaran {
    /// Validasi aturan PPDB: nama wajib, NISN 10 digit, sekolah dan jalur dikenal.
    pub fn validasi(&self) -> Result<(&'static Sekolah, Jalur), Kesalahan> {
        let mut k = Kesalahan::default();
        let nama = self.nama.trim();
        if nama.chars().count() < 3 {
            k.nama = Some("Nama lengkap minimal 3 huruf.");
        }
        let nisn = self.nisn.trim();
        if nisn.len() != 10 || !nisn.chars().all(|c| c.is_ascii_digit()) {
            k.nisn = Some("NISN harus 10 digit angka.");
        }
        let sekolah = sekolah::cari(self.sekolah.trim());
        if sekolah.is_none() {
            k.sekolah = Some("Pilih sekolah tujuan.");
        }
        let jalur = Jalur::dari_kode(self.jalur.trim());
        if jalur.is_none() {
            k.jalur = Some("Pilih jalur pendaftaran.");
        }
        if k.ada() {
            Err(k)
        } else {
            Ok((sekolah.unwrap(), jalur.unwrap()))
        }
    }
}

/// Verifikasi data: hash berulang atas NISN + nama + garam acak.
/// Ini mensimulasikan pekerjaan nyata (cek data kependudukan, tanda tangan dokumen)
/// sehingga beban CPU naik wajar saat banyak orang mendaftar bersamaan.
pub fn verifikasi(nisn: &str, nama: &str, iterasi: u32) -> String {
    let garam: [u8; 16] = rand::thread_rng().gen();
    let mut h = Sha256::new();
    h.update(garam);
    h.update(nisn.as_bytes());
    h.update(nama.as_bytes());
    let mut d = h.finalize();
    for _ in 0..iterasi {
        d = Sha256::digest(d);
    }
    d.iter().take(4).map(|b| format!("{b:02X}")).collect()
}

pub fn samarkan_nisn(nisn: &str) -> String {
    let n = nisn.len();
    if n <= 4 {
        return "*".repeat(n);
    }
    format!("{}{}", "*".repeat(n - 4), &nisn[n - 4..])
}

pub fn nomor_pendaftaran(tahun: &str) -> String {
    let n: u32 = rand::thread_rng().gen_range(100_000..999_999);
    format!("PPDB-{}-{n}", tahun.split('/').next().unwrap_or("2027"))
}

/// Penyimpanan di memori: maksimal `KAPASITAS` pendaftar terakhir per pod.
pub struct PendaftarStore {
    data: RwLock<VecDeque<Pendaftar>>,
    total: RwLock<u64>,
}

const KAPASITAS: usize = 1000;

impl PendaftarStore {
    pub fn new() -> Self {
        Self { data: RwLock::new(VecDeque::with_capacity(KAPASITAS)), total: RwLock::new(0) }
    }

    pub fn simpan(&self, p: Pendaftar) {
        let mut d = self.data.write().unwrap();
        if d.len() == KAPASITAS {
            d.pop_front();
        }
        d.push_back(p);
        *self.total.write().unwrap() += 1;
    }

    /// Jumlah pendaftar yang dilayani pod ini sejak berjalan.
    pub fn total(&self) -> u64 {
        *self.total.read().unwrap()
    }

    pub fn terakhir(&self, n: usize) -> Vec<Pendaftar> {
        self.data.read().unwrap().iter().rev().take(n).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(nama: &str, nisn: &str, sekolah: &str, jalur: &str) -> FormPendaftaran {
        FormPendaftaran { nama: nama.into(), nisn: nisn.into(), sekolah: sekolah.into(), jalur: jalur.into() }
    }

    #[test]
    fn validasi_lolos() {
        assert!(form("Siti Aminah", "0123456789", "SMPN1", "domisili").validasi().is_ok());
    }

    #[test]
    fn validasi_nisn_salah() {
        let k = form("Siti Aminah", "12345", "SMPN1", "domisili").validasi().unwrap_err();
        assert!(k.nisn.is_some() && k.nama.is_none());
    }

    #[test]
    fn samarkan() {
        assert_eq!(samarkan_nisn("0123456789"), "******6789");
    }

    #[test]
    fn verifikasi_berbeda_tiap_kali() {
        assert_ne!(verifikasi("0123456789", "A", 10), verifikasi("0123456789", "A", 10));
    }
}
