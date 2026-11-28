# Update repo GitLab Btech (api dan web terpisah) ke fitur database

Perubahan: commit `e4eaa53` -> `d9a0670` repo ini (arsip PostgreSQL, Valkey ACL + prefix, reset demo).
Repo Btech memecah `api/` dan `web/` jadi project sendiri, jadi pakai patch di folder ini (path relatif ke root project).

## api (`devops/application/ppdb/api`, branch dev)

```bash
git apply --reject ppdb-api-database.patch      # 7 file diubah + src/models/basisdata.rs
```

Dua penyesuaian khusus repo Btech:

1. Unit test `src/models/rekap.rs` (sekitar baris 157): `Rekap::new` sekarang 3 parameter
   ```rust
   let rekap = Rekap::new("pod-test".into(), "ppdb:test".into(), "test".into());
   ```
2. `Cargo.lock` harus diperbarui (pipeline memakai `--locked`): jalankan `cargo check` sekali lalu commit `Cargo.lock`.
   Dependency baru: `tokio-postgres 0.7`, `deadpool-postgres 0.14`.

Sudah diuji: `git apply` bersih dan `cargo check` lolos di salinan branch dev.

## web (`devops/application/ppdb/web`)

```bash
git apply --reject ppdb-web-database.patch       # footer Database/Rekap, kartu "Tersimpan di database"
```

## Env yang dibaca ppdb-api (api-env-secret)

`DB_HOST DB_PORT DB_USER DB_PASSWORD DB_NAME DB_POOL VALKEY_HOST VALKEY_PORT VALKEY_USER VALKEY_PASSWORD VALKEY_PREFIX MAX_ANTREAN CLUSTER IZINKAN_RESET`,
plus `POD_NAME` dari fieldRef `metadata.name` di Deployment.
Log startup yang benar: `rekap: tersambung ke ppdb-store` dan `basisdata: tersambung ke <host>:5432 sebagai <user>`.
