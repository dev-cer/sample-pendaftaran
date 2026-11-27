# Portal PPDB Online (sampel)

Aplikasi sampel **Penerimaan Peserta Didik Baru** untuk demo *Scaling Managed Kubernetes Across Government Sector* (webinar Btech x Civo). Rust full-stack dengan pola **MVC**: Axum + Askama + HTMX + Tailwind. Instansi, sekolah, dan data pendaftar semuanya **fiktif**.

## Struktur MVC

```
src/
├── main.rs                 # router, state, graceful shutdown
├── config.rs               # konfigurasi dari environment variable
├── models/                 # MODEL: data dan aturan bisnis, tanpa HTTP/HTML
│   ├── pendaftar.rs        #   formulir, validasi NISN, verifikasi (beban CPU), penyimpanan
│   └── sekolah.rs          #   sekolah tujuan, kuota, jalur pendaftaran
├── views/mod.rs            # VIEW: struct template Askama
└── controllers/            # CONTROLLER: handler HTTP
    ├── home.rs             #   GET  /
    ├── pendaftaran.rs      #   GET  /daftar, POST /daftar (HTMX: balas potongan HTML)
    └── health.rs           #   GET  /healthz, /readyz, /info
templates/                  # VIEW: HTML (Tailwind + HTMX)
├── base.html  home.html  daftar.html
└── partials/form.html  partials/hasil.html
static/htmx.min.js          # HTMX 2.0.4 (vendored); app.css dibuat Tailwind saat build
```

## Jalankan lokal

```bash
./bin/tailwindcss -i styles/app.css -o static/app.css --minify   # unduh: tailwindcss-linux-x64 v3.4.17
cargo run
# http://localhost:8080
```

## Konfigurasi

| Variabel | Default | Di Kubernetes datang dari |
|---|---|---|
| `APP_VERSION` | `v1` | ConfigMap `ppdb-config` per environment (diubah lewat GitOps = rilis baru) |
| `APP_ENV` | `lokal` | ConfigMap `ppdb-config` |
| `PENGUMUMAN` | kosong | ConfigMap `ppdb-config` |
| `INSTANSI`, `TAHUN_AJARAN` | Dinas Pendidikan Kota Nusantara, 2027/2028 | ConfigMap |
| `POD_NAME` | `lokal` | Downward API (`metadata.name`) |
| `DB_USER` | kosong | Secret `db-credentials` dari Vault (Vault Secrets Operator); password tidak pernah ditampilkan |
| `VERIFIKASI_ITERASI` | `20000` | Beban CPU per pendaftaran (hash berulang) |
| `PORT` | `8080` | |

## Endpoint

| Path | Fungsi |
|---|---|
| `/` | Beranda: info PPDB, kuota sekolah, pendaftar terakhir di pod ini |
| `/daftar` | Formulir; `POST` memvalidasi, memverifikasi, dan menerbitkan nomor pendaftaran |
| `/healthz` | Liveness |
| `/readyz` | Readiness; 503 saat pod berhenti (SIGTERM) supaya rilis tanpa 502 |
| `/info` | JSON versi, environment, pod, database |

Uji beban (yang dipakai demo): `hey -z 240s -c 150 -m POST -T application/x-www-form-urlencoded -d "nama=Siti Aminah&nisn=0123456789&sekolah=SMPN1&jalur=domisili" http://<host>/daftar`

## Image

GitHub Actions (`.github/workflows/image.yml`) menjalankan `cargo test`, lalu build dan push `ghcr.io/dev-cer/sample-pendaftaran:<short-sha>` dan `:latest`. Setelah run pertama, ubah visibility package jadi **Public** supaya cluster bisa pull tanpa kredensial.

Manifest Kubernetes dan GitOps-nya ada di repo [dev-cer/govtech](https://github.com/dev-cer/govtech).

## Lisensi

MIT
