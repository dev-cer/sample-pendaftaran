//! Konfigurasi dari environment variable. Di Kubernetes nilainya datang dari
//! ConfigMap `ppdb-config` (per environment), Downward API (nama pod), dan
//! Secret `db-credentials` yang ditarik Vault Secrets Operator dari Vault.

use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    /// Versi rilis yang tampil di halaman, mis. "v1". Diubah lewat GitOps.
    pub app_version: String,
    /// staging | production
    pub app_env: String,
    /// Nama instansi (fiktif) yang menyelenggarakan PPDB.
    pub instansi: String,
    /// Tahun ajaran yang dibuka.
    pub tahun_ajaran: String,
    /// Pengumuman singkat di beranda (opsional).
    pub pengumuman: String,
    /// Nama pod yang melayani request: memperlihatkan load balancing.
    pub pod_name: String,
    /// Username database dari Vault (tanpa password). Kosong = belum terhubung.
    pub db_user: String,
    /// Iterasi hash saat verifikasi data: beban CPU realistis per pendaftaran.
    pub verifikasi_iterasi: u32,
    pub port: u16,
    pub static_dir: String,
}

fn var(key: &str, default: &str) -> String {
    env::var(key).ok().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| default.to_string())
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            app_version: var("APP_VERSION", "v1"),
            app_env: var("APP_ENV", "lokal"),
            instansi: var("INSTANSI", "Dinas Pendidikan Kota Nusantara"),
            tahun_ajaran: var("TAHUN_AJARAN", "2027/2028"),
            pengumuman: var("PENGUMUMAN", ""),
            pod_name: var("POD_NAME", "lokal"),
            db_user: var("DB_USER", ""),
            verifikasi_iterasi: var("VERIFIKASI_ITERASI", "20000").parse().unwrap_or(20000),
            port: var("PORT", "8080").parse().unwrap_or(8080),
            static_dir: var("STATIC_DIR", "static"),
        }
    }
}
