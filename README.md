# Portal PPDB Online (sampel)

Aplikasi sampel **Penerimaan Peserta Didik Baru** untuk demo *Scaling Managed Kubernetes Across Government Sector* (webinar Btech x Civo). Terdiri dari **2 microservice Rust**, masing-masing berpola **MVC**. Instansi, sekolah, dan data pendaftar semuanya **fiktif**.

```
Pengguna -> Traefik (ingress) -> ppdb-web :8080 -> ppdb-api :8081 -> ppdb-store (Valkey) :6379
                                  halaman HTML       REST JSON          penghitung bersama semua pod
                                  /dashboard rekap   antrean maks/pod   (password dari Vault)
                                                     HPA 2-12
```

Saat satu pod API sedang memproses lebih dari `MAX_ANTREAN` pendaftaran, pendaftar berikutnya dijawab **503 "server sibuk"** dan dicatat sebagai **gagal**. Dengan 2 pod tetap, banyak yang gagal; setelah HPA menambah pod, angka gagal turun. Semuanya terlihat di `/dashboard` (per sekolah, per menit, jumlah pod API aktif).

## Struktur MVC per service

```
api/  ppdb-api   (Axum, JSON)
  src/models/       MODEL       pendaftar.rs (validasi NISN, verifikasi, penyimpanan), sekolah.rs (kuota, jalur)
  src/views/        VIEW        bentuk respons JSON (NISN asli tidak pernah keluar)
  src/controllers/  CONTROLLER  /api/referensi, /api/pendaftaran, /api/statistik, /healthz, /readyz, /info

web/  ppdb-web   (Axum + Askama + HTMX + Tailwind)
  src/models/api.rs MODEL       klien ppdb-api (sumber data)
  src/views/ + templates/  VIEW halaman dan potongan HTMX
  src/controllers/  CONTROLLER  /, /daftar (GET/POST), /healthz, /readyz, /info
```

## Jalankan lokal

```bash
VALKEY_HOST=127.0.0.1 cargo run -p ppdb-api              # :8081 (redis-server/valkey-server lokal untuk rekap)
(cd web && ../bin/tailwindcss -i styles/app.css -o static/app.css --minify && cargo run -p ppdb-web)   # :8080
# http://localhost:8080   (tailwindcss standalone v3.4.17 di bin/)
```

## Konfigurasi

| Variabel | Service | Default | Di Kubernetes |
|---|---|---|---|
| `APP_VERSION`, `APP_ENV` | web, api | `v1`, `lokal` | ConfigMap `ppdb-config` per environment (ubah `APP_VERSION` = rilis baru) |
| `INSTANSI`, `TAHUN_AJARAN`, `PENGUMUMAN` | web (api: tahun) | Dinas Pendidikan Kota Nusantara, 2027/2028 | ConfigMap |
| `POD_NAME` | web, api | `web-lokal`, `api-lokal` | Downward API; tampil di footer (pod web dan pod API) |
| `API_URL` | web | `http://localhost:8081` | `http://ppdb-api:8081` |
| `DB_USER` | api | kosong | Secret `db-credentials` dari Vault (VSO); password tidak pernah ditampilkan |
| `VERIFIKASI_ITERASI` | api | `20000` | Beban CPU per pendaftaran |
| `MAX_ANTREAN` | api | `32` | Pendaftaran bersamaan per pod; lebih = 503 dan dicatat gagal |
| `VALKEY_HOST`, `VALKEY_PASSWORD` | api | kosong (rekap nonaktif) | `ppdb-store`; password dari Secret `db-credentials` (Vault) |

Uji beban: `hey -z 240s -c 150 -m POST -T application/x-www-form-urlencoded -d "nama=Siti+Aminah&nisn=0123456789&sekolah=SMPN1&jalur=domisili" http://<host>/daftar`

## Image

GitHub Actions (`.github/workflows/image.yml`): `cargo test`, lalu build dan push ke satu package GHCR:

- `ghcr.io/dev-cer/sample-pendaftaran:web-<short-sha>` dan `:web-latest`
- `ghcr.io/dev-cer/sample-pendaftaran:api-<short-sha>` dan `:api-latest`

Package harus **Public** supaya cluster bisa pull tanpa kredensial. Manifest Kubernetes dan GitOps ada di [dev-cer/govtech](https://github.com/dev-cer/govtech).

## Lisensi

MIT
