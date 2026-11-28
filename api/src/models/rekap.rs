//! Model Rekap: penghitung bersama semua pod API di ppdb-store (Valkey).
//!
//! ppdb-store berjalan di LUAR cluster (VM data). Kunci diawali VALKEY_PREFIX (mis. ppdb:production),
//! dan user ACL tiap environment hanya boleh menyentuh prefix-nya sendiri.
//!   <prefix>:diterima / :gagal          HASH kode_sekolah -> jumlah (gabungan prod-dc dan prod-drc)
//!   <prefix>:m:<menit>:diterima|gagal   jumlah per menit (kedaluwarsa 2 jam)
//!   <prefix>:pods:<cluster>             ZSET pod -> detik terakhir aktif (heartbeat per cluster)
//! Bila ppdb-store tidak terjangkau, pendaftaran tetap jalan; rekap saja yang kosong.

use redis::aio::ConnectionManager;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::OnceCell;

pub struct Rekap {
    conn: OnceCell<ConnectionManager>,
    pod: String,
    prefix: String,
    cluster: String,
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
    pub fn new(pod: String, prefix: String, cluster: String) -> Self {
        Self { conn: OnceCell::new(), pod, prefix, cluster }
    }

    fn k(&self, s: &str) -> String {
        format!("{}:{s}", self.prefix)
    }

    pub fn aktif(&self) -> bool {
        self.conn.initialized()
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
                    let kp = me.k(&format!("pods:{}", me.cluster));
                    let _: Result<(), _> = redis::pipe()
                        .zadd(&kp, &me.pod, now).ignore()
                        .zrembyscore(&kp, 0, now - 60).ignore()
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
        let (km, kh) = (self.k(&format!("m:{}:{jenis}", menit_ini())), self.k(jenis));
        tokio::spawn(async move {
            let _: Result<(), _> = redis::pipe()
                .hincr(kh, kode_sekolah, 1).ignore()
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
            kunci.push(self.k(&format!("m:{m}:diterima")));
            kunci.push(self.k(&format!("m:{m}:gagal")));
        }
        let now = chrono::Utc::now().timestamp();
        let hasil: Result<(HashMap<String, u64>, HashMap<String, u64>, Vec<Option<u64>>, u64), _> = redis::pipe()
            .hgetall(self.k("diterima"))
            .hgetall(self.k("gagal"))
            .mget(&kunci)
            .zcount(self.k(&format!("pods:{}", self.cluster)), now - 15, "+inf")
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

    /// Kosongkan hitungan (hanya untuk demo): total per sekolah dan per menit 3 jam terakhir.
    pub async fn kosongkan(&self) -> Result<(), String> {
        let mut c = self.conn.get().cloned().ok_or("ppdb-store belum tersambung")?;
        let akhir = menit_ini();
        let mut kunci = vec![self.k("diterima"), self.k("gagal")];
        for m in akhir - 180..=akhir {
            kunci.push(self.k(&format!("m:{m}:diterima")));
            kunci.push(self.k(&format!("m:{m}:gagal")));
        }
        redis::cmd("DEL").arg(&kunci).query_async::<()>(&mut c).await.map_err(|e| e.to_string())
    }
}
