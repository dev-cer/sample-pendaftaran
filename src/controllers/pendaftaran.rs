//! Controller pendaftaran: tampilkan formulir, terima kiriman, validasi,
//! verifikasi (beban CPU), simpan, lalu kembalikan bukti pendaftaran.
//! Permintaan dari HTMX (header `HX-Request`) dibalas potongan HTML saja.

use crate::models::pendaftar::{self, FormPendaftaran, Pendaftar};
use crate::views::{render, DaftarView, FormPartial, FormView, HasilPartial};
use crate::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Response;
use axum::Form;
use std::sync::Arc;

fn dari_htmx(h: &HeaderMap) -> bool {
    h.contains_key("hx-request")
}

pub async fn form(State(st): State<Arc<AppState>>) -> Response {
    render(DaftarView { app: st.app.clone(), f: FormView::kosong(), hasil: None })
}

pub async fn kirim(State(st): State<Arc<AppState>>, headers: HeaderMap, Form(input): Form<FormPendaftaran>) -> Response {
    let (sekolah, jalur) = match input.validasi() {
        Ok(v) => v,
        Err(err) => {
            let f = FormView::dari(&input, err);
            return if dari_htmx(&headers) {
                render(FormPartial { f })
            } else {
                render(DaftarView { app: st.app.clone(), f, hasil: None })
            };
        }
    };

    // Verifikasi berjalan di thread pool blocking supaya tidak menahan event loop.
    let (nisn, nama, iterasi) = (input.nisn.trim().to_string(), input.nama.trim().to_string(), st.cfg.verifikasi_iterasi);
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

    if dari_htmx(&headers) {
        render(HasilPartial { app: st.app.clone(), p })
    } else {
        render(DaftarView { app: st.app.clone(), f: FormView::kosong(), hasil: Some(p) })
    }
}
