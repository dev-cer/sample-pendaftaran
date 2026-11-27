//! Model Rekap: penghitung bersama semua pod API di ppdb-store (Valkey).
//!
//! Kunci:
//!   ppdb:diterima / ppdb:gagal   HASH kode_sekolah -> jumlah
//!   ppdb:m:<menit>:diterima|gagal  jumlah per menit (kedaluwarsa 2 jam)
//!   ppdb:pods                    ZSET pod -> detik terakhir aktif (heartbeat)
//! Bila ppdb-store tidak terjangkau, pendaftaran tetap jalan; rekap saja yang kosong.

use redis::aio::ConnectionManager;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::OnceCell;

pub struct Rekap {
    conn: OnceCell<ConnectionManager>,
    pod: String,
}

#[derive(Default)]
pub struct Ringkasan {
    pub tersambung: bool,
    pub diterima: HashMap<String, u64>,
    pub gagal: HashMap<String, u64>,
    /// (menit epoch, diterima, gagal), terlama dulu
    pub per_menit: Vec<(i64, u64, u64)>,
    pub pod_aktif: u64,
}

fn menit_ini() -> i64 {
    chrono::Utc::now().timestamp() / 60
}

impl Rekap {
    pub fn new(pod: String) -> Self {
        Self { conn: OnceCell::new(), pod }
    }

    /// Sambung ke Valkey di latar belakang (coba ulang sampai berhasil), lalu kirim heartbeat tiap 5 detik.
    pub fn mulai(self: &std::sync::Arc<Self>, url: Option<String>) {
        let Some(url) = url else {
            println!("rekap: VALKEY_HOST kosong, rekap bersama nonaktif");
            return;
        };
        let me = self.clone();
        tokio::spawn(async move {
            let client = match redis::Client::open(url) {
                Ok(c) => c,
                Err(e) => return eprintln!("rekap: URL Valkey tidak valid: {e}"),
            };
            loop {
                match ConnectionManager::new(client.clone()).await {
                    Ok(cm) => {
                        let _ = me.conn.set(cm);
                        println!("rekap: tersambung ke ppdb-store");
                        break;
                    }
                    Err(e) => {
                        eprintln!("rekap: belum tersambung ({e}), coba lagi 3 detik");
                        tokio::time::sleep(Duration::from_secs(3)).await;
                    }
                }
            }
            loop {
                if let Some(mut c) = me.conn.get().cloned() {
                    let now = chrono::Utc::now().timestamp();
                    let _: Result<(), _> = redis::pipe()
                        .zadd("ppdb:pods", &me.pod, now).ignore()
                        .zrembyscore("ppdb:pods", 0, now - 60).ignore()
                        .query_async(&mut c).await;
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }

    /// Catat satu pendaftaran (diterima atau gagal karena server sibuk). Tidak menunggu Valkey.
    pub fn catat(&self, kode_sekolah: &'static str, diterima: bool) {
        let Some(mut c) = self.conn.get().cloned() else { return };
        let jenis = if diterima { "diterima" } else { "gagal" };
        tokio::spawn(async move {
            let km = format!("ppdb:m:{}:{jenis}", menit_ini());
            let _: Result<(), _> = redis::pipe()
                .hincr(format!("ppdb:{jenis}"), kode_sekolah, 1).ignore()
                .incr(&km, 1).ignore()
                .expire(&km, 7200).ignore()
                .query_async(&mut c).await;
        });
    }

    pub async fn ringkasan(&self, menit: i64) -> Ringkasan {
        let Some(mut c) = self.conn.get().cloned() else { return Ringkasan::default() };
        let akhir = menit_ini();
        let daftar: Vec<i64> = (akhir - menit + 1..=akhir).collect();
        let mut kunci = Vec::new();
        for m in &daftar {
            kunci.push(format!("ppdb:m:{m}:diterima"));
            kunci.push(format!("ppdb:m:{m}:gagal"));
        }
        let now = chrono::Utc::now().timestamp();
        let hasil: Result<(HashMap<String, u64>, HashMap<String, u64>, Vec<Option<u64>>, u64), _> = redis::pipe()
            .hgetall("ppdb:diterima")
            .hgetall("ppdb:gagal")
            .mget(&kunci)
            .zcount("ppdb:pods", now - 15, "+inf")
            .query_async(&mut c)
            .await;
        match hasil {
            Ok((diterima, gagal, nilai, pod_aktif)) => Ringkasan {
                tersambung: true,
                diterima,
                gagal,
                per_menit: daftar
                    .iter()
                    .enumerate()
                    .map(|(i, m)| (*m, nilai[i * 2].unwrap_or(0), nilai[i * 2 + 1].unwrap_or(0)))
                    .collect(),
                pod_aktif,
            },
            Err(e) => {
                eprintln!("rekap: gagal membaca ({e})");
                Ringkasan::default()
            }
        }
    }
}
