//! Model ppdb-web: sumber datanya adalah ppdb-api (HTTP/JSON), bukan database langsung.
//! Struct di sini mencerminkan View JSON milik ppdb-api.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Meta {
    pub versi: String,
    pub pod: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Sekolah {
    pub kode: String,
    pub nama: String,
    pub kecamatan: String,
    pub kuota: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Jalur {
    pub kode: String,
    pub label: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Referensi {
    pub sekolah: Vec<Sekolah>,
    pub jalur: Vec<Jalur>,
    pub meta: Meta,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Pendaftar {
    pub nomor: String,
    pub nama: String,
    pub nisn: String,
    pub sekolah: String,
    pub sekolah_kode: String,
    pub jalur: String,
    pub waktu: String,
    pub kode_verifikasi: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Statistik {
    pub total_pod_ini: u64,
    #[serde(default)]
    pub sumber: String,
    #[serde(default)]
    pub store: bool,
    pub terakhir: Vec<Pendaftar>,
    pub database: String,
    pub meta: Meta,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RekapSekolah {
    pub kode: String,
    pub nama: String,
    pub kecamatan: String,
    pub kuota: u32,
    pub diterima: u64,
    pub gagal: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RekapMenit {
    pub jam: String,
    pub diterima: u64,
    pub gagal: u64,
}

/// Rekap bersama semua pod API (dari ppdb-store lewat ppdb-api).
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Dashboard {
    pub tersambung: bool,
    pub sekolah: Vec<RekapSekolah>,
    pub total_diterima: u64,
    pub total_gagal: u64,
    pub per_menit: Vec<RekapMenit>,
    pub pod_api_aktif: u64,
}

#[derive(Debug, Deserialize)]
struct Diterima {
    pendaftar: Pendaftar,
    meta: Meta,
}

#[derive(Debug, Deserialize)]
struct Ditolak {
    kesalahan: BTreeMap<String, String>,
}

/// Data formulir dari browser, diteruskan ke ppdb-api sebagai JSON.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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

pub enum HasilDaftar {
    Diterima(Pendaftar, Meta),
    Ditolak(BTreeMap<String, String>),
    /// ppdb-api penuh (503): pendaftar diminta mencoba lagi.
    Sibuk,
    /// ppdb-api tidak bisa dihubungi atau galat.
    Gangguan(String),
}

#[derive(Clone)]
pub struct ApiClient {
    base: String,
    http: reqwest::Client,
}

impl ApiClient {
    pub fn new(base: &str) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .pool_max_idle_per_host(64)
            .build()
            .expect("klien HTTP");
        Self { base: base.trim_end_matches('/').to_string(), http }
    }

    pub async fn referensi(&self) -> Result<Referensi, String> {
        self.get("/api/referensi").await
    }

    pub async fn statistik(&self) -> Result<Statistik, String> {
        self.get("/api/statistik").await
    }

    pub async fn dashboard(&self) -> Result<Dashboard, String> {
        self.get("/api/dashboard").await
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, String> {
        let r = self.http.get(format!("{}{path}", self.base)).send().await.map_err(|e| e.to_string())?;
        if !r.status().is_success() {
            return Err(format!("ppdb-api menjawab {}", r.status()));
        }
        r.json::<T>().await.map_err(|e| e.to_string())
    }

    pub async fn daftar(&self, f: &FormPendaftaran) -> HasilDaftar {
        let r = match self.http.post(format!("{}/api/pendaftaran", self.base)).json(f).send().await {
            Ok(r) => r,
            Err(e) => return HasilDaftar::Gangguan(e.to_string()),
        };
        match r.status().as_u16() {
            201 => match r.json::<Diterima>().await {
                Ok(d) => HasilDaftar::Diterima(d.pendaftar, d.meta),
                Err(e) => HasilDaftar::Gangguan(e.to_string()),
            },
            422 => match r.json::<Ditolak>().await {
                Ok(d) => HasilDaftar::Ditolak(d.kesalahan),
                Err(e) => HasilDaftar::Gangguan(e.to_string()),
            },
            503 => HasilDaftar::Sibuk,
            s => HasilDaftar::Gangguan(format!("ppdb-api menjawab {s}")),
        }
    }
}
