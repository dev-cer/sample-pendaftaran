//! Controller: handler REST. Menerima JSON, memanggil Model, mengembalikan View (JSON).

use crate::models::pendaftar::{self, InputPendaftaran, Pendaftar};
use crate::views::{self, Dashboard, Diterima, Ditolak, Meta, PendaftarJson, Sibuk, Statistik};
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::sync::atomic::Ordering;
use std::sync::Arc;

fn meta(st: &AppState) -> Meta {
    Meta { service: "ppdb-api", versi: st.cfg.versi.clone(), environment: st.cfg.environment.clone(), pod: st.cfg.pod.clone() }
}

/// GET /api/referensi: sekolah tujuan, kuota, dan jalur.
pub async fn referensi(State(st): State<Arc<AppState>>) -> Json<views::Referensi> {
    Json(views::referensi(meta(&st)))
}

/// Menurunkan hitungan antrean saat request selesai (termasuk bila klien memutus).
struct SlotAntrean<'a>(&'a AppState);
impl Drop for SlotAntrean<'_> {
    fn drop(&mut self) {
        self.0.antrean.fetch_sub(1, Ordering::Relaxed);
    }
}

/// POST /api/pendaftaran: 201 + bukti, 422 + kesalahan per isian, atau 503 bila pod ini penuh.
pub async fn daftar(State(st): State<Arc<AppState>>, Json(input): Json<InputPendaftaran>) -> Response {
    let (sekolah, jalur) = match input.validasi() {
        Ok(v) => v,
        Err(kesalahan) => return (StatusCode::UNPROCESSABLE_ENTITY, Json(Ditolak { kesalahan, meta: meta(&st) })).into_response(),
    };
    // Kapasitas pod: lebih dari MAX_ANTREAN yang sedang diproses = ditolak (server sibuk) dan dicatat gagal.
    if st.antrean.fetch_add(1, Ordering::Relaxed) >= st.cfg.max_antrean {
        st.antrean.fetch_sub(1, Ordering::Relaxed);
        st.rekap.catat(sekolah.kode, false);
        return (StatusCode::SERVICE_UNAVAILABLE, Json(Sibuk { pesan: "Server sedang sibuk, silakan coba lagi.", meta: meta(&st) })).into_response();
    }
    let _slot = SlotAntrean(&st);
    let (nisn, nama, iterasi) = (input.nisn.trim().to_string(), input.nama.trim().to_string(), st.cfg.iterasi);
    // Verifikasi di thread pool blocking supaya event loop tetap responsif.
    let kode = tokio::task::spawn_blocking(move || pendaftar::verifikasi(&nisn, &nama, iterasi))
        .await
        .unwrap_or_else(|_| "GAGAL".into());
    let p = Pendaftar {
        nomor: pendaftar::nomor_pendaftaran(&st.cfg.tahun_ajaran),
        nama: input.nama.trim().to_string(),
        nisn_tersamar: pendaftar::samarkan_nisn(input.nisn.trim()),
        sekolah,
        jalur,
        waktu: chrono::Local::now().format("%d-%m-%Y %H:%M:%S").to_string(),
        kode_verifikasi: kode,
    };
    st.store.simpan(p.clone());
    st.rekap.catat(p.sekolah.kode, true);
    (StatusCode::CREATED, Json(Diterima { pendaftar: PendaftarJson::from(&p), meta: meta(&st) })).into_response()
}

/// GET /api/statistik: jumlah pendaftar yang diproses pod ini dan 5 terakhir.
pub async fn statistik(State(st): State<Arc<AppState>>) -> Json<Statistik> {
    Json(Statistik {
        total_pod_ini: st.store.total(),
        terakhir: st.store.terakhir(5).iter().map(PendaftarJson::from).collect(),
        database: st.cfg.db_user.clone(),
        meta: meta(&st),
    })
}

/// GET /api/dashboard: rekap bersama semua pod (per sekolah, per menit, pod aktif).
pub async fn dashboard(State(st): State<Arc<AppState>>) -> Json<Dashboard> {
    let r = st.rekap.ringkasan(10).await;
    Json(views::dashboard(r, st.cfg.tz_jam, meta(&st)))
}

pub async fn healthz() -> &'static str {
    "ok"
}

/// 503 saat pod berhenti, supaya Service berhenti mengirim request sebelum proses keluar.
pub async fn readyz(State(st): State<Arc<AppState>>) -> Response {
    if st.ready.load(Ordering::Relaxed) {
        (StatusCode::OK, "siap").into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "berhenti").into_response()
    }
}

pub async fn info(State(st): State<Arc<AppState>>) -> Json<Meta> {
    Json(meta(&st))
}
