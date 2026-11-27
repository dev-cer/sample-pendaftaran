//! Controller beranda: info PPDB, sekolah tujuan dan kuota, pendaftar terakhir di pod ini.

use crate::views::{render, HomeView};
use crate::AppState;
use axum::extract::State;
use axum::response::Response;
use std::sync::Arc;

pub async fn index(State(st): State<Arc<AppState>>) -> Response {
    render(HomeView::new(st.app.clone(), st.store.total(), st.store.terakhir(5)))
}
