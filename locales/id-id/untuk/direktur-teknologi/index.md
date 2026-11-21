# Untuk Direktur Teknologi

> Ringkasan rekayasa, bukan nasihat hukum, sertifikasi, atau audit. Ringkasan ini menyatakan
> apa yang ditunjukkan repositori per 2026-10-11. Tidak ada yang di sini mengklaim akreditasi.

## Arsitektur

- **Layanan:** Rust di atas Axum dan axa (sqlx 0.9), PostgreSQL 18, Redis untuk cache,
  penghitung bersama, dan propagasi break-glass. `forbid(unsafe_code)`, clippy
  pedantic. Sumber kebenaran perilaku adalah spesifikasi
  ([index](../../../../spec/index.md), [architecture](../../../../spec/architecture.md)).
- **UI:** SvelteKit 3 / Svelte 5, proxy backend-for-frontend sehingga tidak ada token yang
  sampai ke JavaScript browser, CSP ketat dengan hash.
- **Autentikasi:** PASETO v4.public diverifikasi secara luring, Keycloak opsional; kontrol akses
  berbasis atribut (tindakan baca/tulis/hapus/destruktif, kewajiban `mask`,
  pemeriksaan tingkat catatan) dari mesin bersama.
- **Audit:** rantai hash SHA-256 ditulis dalam transaksi yang sama dengan perubahan,
  pemicu hanya-tambah, dapat diverifikasi di `GET /api/audits/verify`.
- **Perlindungan data:** enkripsi bidang AES-256-GCM dengan rotasi kunci pada dua
  kolom; kunci lokal, tanpa KMS.

## Bukti kualitas

Uji unit (185), uji basis data (56 termasuk penegakan, masking, audit,
paginasi, caching, idempotensi, pemeriksaan skema), unit front-end (106) dan
Playwright (98) termasuk pemindaian aksesibilitas axe. CI menjalankan fmt, clippy, pengujian, build/lint front-end dan
tahap rantai pasok (`cargo audit`, `cargo deny`, `pnpm audit`, SBOM CycloneDX)
di GitHub Actions dan Woodpecker. Satu perintah mereproduksinya: `scripts/ci-check.sh`.

## Utang teknis dan batas yang jujur

- Paginasi memakai kursor offset (baris dapat terlewat atau berulang jika data berubah
  di tengah penelusuran); sengaja tanpa jumlah total.
- Kunci idempotensi bersifat opsional di API (T52): klien harus mengirim
  `Idempotency-Key` pada pembuatan agar terlindungi dari percobaan ulang yang menggandakan
  catatan. UI bawaan mengirimnya pada setiap pembuatan.
- Skema OpenAPI ditulis tangan. Skema mencakup permintaan, daftar, pembacaan detail,
  dasbor, tampilan insight, audit, privasi, dan break-glass; transisi status
  dan upsert yang mengembalikan catatan yang diperbarui tidak bertipe. Pengujian gagal ketika
  rute dan dokumen tidak cocok, dan pengujian lain memeriksa respons nyata, termasuk yang dimasking,
  terhadap skema
  ([gds-api-review](../../../../spec/compliance/gds-api-review.md)).
- Kesalahan adalah dokumen masalah RFC 9457 (T54) dengan anggota `error` dan
  `description` yang lama dipertahankan; kesalahan dari lapisan di luar router (batas
  ukuran, waktu habis) tidak dikonversi. Kebijakan pemberian versi dan penghentian adalah
  [api-lifecycle](../../../../spec/compliance/api-lifecycle.md); header
  `Deprecation` dan `Sunset` sudah ditentukan tetapi belum dibangun.
- Enkripsi bidang mencakup dua kolom; tidak ada integrasi KMS.
- Status pembatasan laju dan cache memerlukan Redis agar benar pada banyak replika.
- Tampilan insight dimasking menurut peran tetapi di-cache per token pemanggil selama 30 dtk.
- Autentikasi ulang bertahap sebelum tindakan destruktif memerlukan pekerjaan pada penyedia identitas.

## Urutan teknis yang disarankan

1. Kunci bidang berbasis KMS dan cakupan bidang yang lebih luas (sisa T41).
2. Tandai rilis pertama untuk membuktikan penandatanganan dan verifikasi tanpa kunci (T49), dan
   minta peninjau memeriksa draf standar API.
3. Bangun header `Deprecation` dan `Sunset` bersama penghentian pertama.
4. Pesan uji independen (T42) terhadap build staging.
