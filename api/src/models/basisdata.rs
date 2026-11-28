//! Model Basisdata: arsip pendaftar di PostgreSQL (ppdb-db, di LUAR cluster).
//!
//! Kredensial (DB_USER / DB_PASSWORD) dari Vault lewat Secret `db-credentials`.
//! Tersambung di latar belakang; sebelum tersambung pendaftaran disimpan di memori pod (mode cadangan).
//! Setelah tersambung, setiap pendaftaran WAJIB tersimpan: bila database gagal, pendaftar mendapat 503.

use crate::models::pendaftar::Pendaftar;
use crate::models::sekolah::{self, Jalur};
use deadpool_postgres::{Config, ManagerConfig, Pool, PoolConfig, RecyclingMethod, Runtime, Timeouts};
use std::time::Duration;
use tokio::sync::OnceCell;
use tokio_postgres::NoTls;

pub struct Basisdata {
    pool: OnceCell<Pool>,
    pub user: String,
    pub host: String,
}

pub struct Koneksi {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub dbname: String,
    pub pool: usize,
}

const SKEMA: &str = "
CREATE TABLE IF NOT EXISTS pendaftar (
    id              bigserial PRIMARY KEY,
    tahun           text        NOT NULL,
    nama            text        NOT NULL,
    nisn_tersamar   text        NOT NULL,
    sekolah         text        NOT NULL,
    jalur           text        NOT NULL,
    kode_verifikasi text        NOT NULL,
    pod             text        NOT NULL,
    dibuat          timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS pendaftar_dibuat ON pendaftar (dibuat DESC);";

const WAKTU: &str = "to_char(dibuat AT TIME ZONE 'Asia/Jakarta', 'DD-MM-YYYY HH24:MI:SS')";

/// Nomor pendaftaran berurutan dari id database: PPDB-2027-000123
fn nomor(tahun: &str, id: i64) -> String {
    format!("PPDB-{}-{id:06}", tahun.split('/').next().unwrap_or("2027"))
}

impl Basisdata {
    pub fn new(host: &str, user: &str) -> Self {
        Self { pool: OnceCell::new(), user: user.to_string(), host: host.to_string() }
    }

    pub fn aktif(&self) -> bool {
        self.pool.initialized()
    }

    /// Sambung di latar belakang (coba ulang tiap 3 detik) lalu buat tabel bila belum ada.
    pub fn mulai(self: &std::sync::Arc<Self>, k: Option<Koneksi>) {
        let Some(k) = k else {
            println!("basisdata: DB_HOST/DB_USER kosong, pendaftar hanya disimpan di memori pod");
            return;
        };
        let me = self.clone();
        tokio::spawn(async move {
            let mut cfg = Config::new();
            cfg.host = Some(k.host.clone());
            cfg.port = Some(k.port);
            cfg.user = Some(k.user.clone());
            cfg.password = Some(k.password.clone());
            cfg.dbname = Some(k.dbname.clone());
            cfg.application_name = Some("ppdb-api".into());
            cfg.connect_timeout = Some(Duration::from_secs(3));
            cfg.manager = Some(ManagerConfig { recycling_method: RecyclingMethod::Fast });
            let mut pc = PoolConfig::new(k.pool);
            pc.timeouts = Timeouts { wait: Some(Duration::from_secs(3)), create: Some(Duration::from_secs(3)), recycle: Some(Duration::from_secs(3)) };
            cfg.pool = Some(pc);
            let pool = match cfg.create_pool(Some(Runtime::Tokio1), NoTls) {
                Ok(p) => p,
                Err(e) => return eprintln!("basisdata: konfigurasi tidak valid: {e}"),
            };
            loop {
                match pool.get().await {
                    Ok(c) => match c.batch_execute(SKEMA).await {
                        Ok(()) => {
                            drop(c);
                            let _ = me.pool.set(pool);
                            println!("basisdata: tersambung ke {}:{} sebagai {}", k.host, k.port, k.user);
                            break;
                        }
                        Err(e) => eprintln!("basisdata: gagal membuat tabel ({e}), coba lagi 3 detik"),
                    },
                    Err(e) => eprintln!("basisdata: belum tersambung ({e}), coba lagi 3 detik"),
                }
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        });
    }

    /// Simpan pendaftar; database memberi id (nomor berurutan) dan waktu.
    pub async fn simpan(&self, p: &mut Pendaftar, tahun: &str, pod: &str) -> Result<(), String> {
        let pool = self.pool.get().ok_or("basisdata belum tersambung")?;
        let c = pool.get().await.map_err(|e| e.to_string())?;
        let sql = format!(
            "INSERT INTO pendaftar (tahun, nama, nisn_tersamar, sekolah, jalur, kode_verifikasi, pod)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id, {WAKTU}"
        );
        let row = c
            .query_one(&sql, &[&tahun, &p.nama, &p.nisn_tersamar, &p.sekolah.kode, &p.jalur.kode(), &p.kode_verifikasi, &pod])
            .await
            .map_err(|e| e.to_string())?;
        p.nomor = nomor(tahun, row.get::<_, i64>(0));
        p.waktu = row.get(1);
        Ok(())
    }

    pub async fn total(&self) -> Option<u64> {
        let c = self.pool.get()?.get().await.ok()?;
        let r = c.query_one("SELECT count(*) FROM pendaftar", &[]).await.ok()?;
        Some(r.get::<_, i64>(0) as u64)
    }

    pub async fn terakhir(&self, n: i64) -> Vec<Pendaftar> {
        let Some(pool) = self.pool.get() else { return vec![] };
        let Ok(c) = pool.get().await else { return vec![] };
        let sql = format!(
            "SELECT id, tahun, nama, nisn_tersamar, sekolah, jalur, kode_verifikasi, {WAKTU} FROM pendaftar ORDER BY id DESC LIMIT $1"
        );
        let Ok(rows) = c.query(&sql, &[&n]).await else { return vec![] };
        rows.iter()
            .filter_map(|r| {
                let tahun: String = r.get(1);
                Some(Pendaftar {
                    nomor: nomor(&tahun, r.get(0)),
                    nama: r.get(2),
                    nisn_tersamar: r.get(3),
                    sekolah: sekolah::cari(r.get::<_, &str>(4))?,
                    jalur: Jalur::dari_kode(r.get::<_, &str>(5))?,
                    kode_verifikasi: r.get(6),
                    waktu: r.get(7),
                })
            })
            .collect()
    }

    /// Kosongkan arsip (hanya untuk demo, dijaga IZINKAN_RESET).
    pub async fn kosongkan(&self) -> Result<(), String> {
        let pool = self.pool.get().ok_or("basisdata belum tersambung")?;
        let c = pool.get().await.map_err(|e| e.to_string())?;
        c.batch_execute("TRUNCATE pendaftar RESTART IDENTITY").await.map_err(|e| e.to_string())
    }
}
