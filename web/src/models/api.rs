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
    pub terakhir: Vec<Pendaftar>,
    pub database: String,
    pub meta: Meta,
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
            s => HasilDaftar::Gangguan(format!("ppdb-api menjawab {s}")),
        }
    }
}
