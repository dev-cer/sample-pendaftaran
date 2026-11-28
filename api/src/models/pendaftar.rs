//! Model Pendaftar: validasi aturan PPDB, verifikasi data (beban CPU), dan penyimpanan.
//!
//! Penyimpanan sengaja di memori per pod (aplikasi sampel untuk demo platform).
//! Di produksi, `PendaftarStore` diganti repository ke database dengan kredensial
//! dari Secret `db-credentials` (Vault).

use super::sekolah::{self, Jalur, Sekolah};
use rand::Rng;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

/// Data masukan dari ppdb-web (JSON).
#[derive(Debug, Default, Deserialize)]
pub struct InputPendaftaran {
    #[serde(default)]
    pub nama: String,
    #[serde(default)]
    pub nisn: String,
    #[serde(default)]
    pub sekolah: String,
    #[serde(default)]
    pub jalur: String,
}

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

/// Kesalahan per isian: nama isian -> pesan.
pub type Kesalahan = BTreeMap<&'static str, &'static str>;

impl InputPendaftaran {
    pub fn validasi(&self) -> Result<(&'static Sekolah, Jalur), Kesalahan> {
        let mut k = Kesalahan::new();
        if self.nama.trim().chars().count() < 3 {
            k.insert("nama", "Nama lengkap minimal 3 huruf.");
        }
        let nisn = self.nisn.trim();
        if nisn.len() != 10 || !nisn.chars().all(|c| c.is_ascii_digit()) {
            k.insert("nisn", "NISN harus 10 digit angka.");
        }
        let sekolah = sekolah::cari(self.sekolah.trim());
        if sekolah.is_none() {
            k.insert("sekolah", "Pilih sekolah tujuan.");
        }
        let jalur = Jalur::dari_kode(self.jalur.trim());
        if jalur.is_none() {
            k.insert("jalur", "Pilih jalur pendaftaran.");
        }
        match (sekolah, jalur) {
            (Some(s), Some(j)) if k.is_empty() => Ok((s, j)),
            _ => Err(k),
        }
    }
}

/// Verifikasi data: hash berulang atas NISN + nama + garam acak. Mensimulasikan pekerjaan
/// nyata (cek data kependudukan, tanda tangan dokumen) supaya CPU naik wajar saat ramai.
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
    total: AtomicU64,
}

const KAPASITAS: usize = 1000;

impl Default for PendaftarStore {
    fn default() -> Self {
        Self { data: RwLock::new(VecDeque::with_capacity(KAPASITAS)), total: AtomicU64::new(0) }
    }
}

impl PendaftarStore {
    pub fn simpan(&self, p: Pendaftar) {
        let mut d = self.data.write().unwrap();
        if d.len() == KAPASITAS {
            d.pop_front();
        }
        d.push_back(p);
        self.total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn total(&self) -> u64 {
        self.total.load(Ordering::Relaxed)
    }

    pub fn kosongkan(&self) {
        self.data.write().unwrap().clear();
        self.total.store(0, Ordering::Relaxed);
    }

    pub fn terakhir(&self, n: usize) -> Vec<Pendaftar> {
        self.data.read().unwrap().iter().rev().take(n).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(nama: &str, nisn: &str, sekolah: &str, jalur: &str) -> InputPendaftaran {
        InputPendaftaran { nama: nama.into(), nisn: nisn.into(), sekolah: sekolah.into(), jalur: jalur.into() }
    }

    #[test]
    fn validasi_lolos() {
        assert!(input("Siti Aminah", "0123456789", "SMPN1", "domisili").validasi().is_ok());
    }

    #[test]
    fn validasi_nisn_salah() {
        let k = input("Siti Aminah", "12345", "SMPN1", "domisili").validasi().unwrap_err();
        assert!(k.contains_key("nisn") && !k.contains_key("nama"));
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
