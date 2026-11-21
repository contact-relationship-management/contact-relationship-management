# Untuk Direktur Keamanan

> Ringkasan rekayasa, bukan nasihat hukum, sertifikasi, atau audit. Ringkasan ini menyatakan
> apa yang ditunjukkan repositori per 2026-10-11. Tidak ada yang di sini mengklaim akreditasi.

## Postur keamanan dalam satu paragraf

Layanan ini **gagal dengan aman**, mengendalikan akses menurut peran dan menurut catatan, menyembunyikan
nilai sensitif dari peran yang tidak semestinya melihatnya, dan menyimpan jejak audit yang
menunjukkan pengubahan. Kontrol-kontrol itu diterapkan dan diuji oleh para penulisnya. Kontrol itu
**belum diuji oleh pihak independen mana pun** (T42), jadi anggaplah setiap pernyataan di bawah
sebagai "dibangun dan diuji sendiri", bukan "terverifikasi".

## Kontrol dan tempat memeriksanya

| Bidang | Apa yang tersedia | Tempat melihat |
|---|---|---|
| Autentikasi | PASETO v4.public diverifikasi secara luring, Keycloak opsional; produksi menolak mulai dengan autentikasi mati | [auth](../../../../spec/auth.md) |
| Otorisasi | Berbasis atribut: baca, tulis, hapus, destruktif; kewajiban `mask`; pemeriksaan tingkat catatan; audit, retensi, dan peninjauan break-glass khusus admin | [auth](../../../../spec/auth.md), uji matriks persona |
| Pemaparan data | Masking pada catatan, agregat, dan insight; kebocoran detail akun ditemukan dan diperbaiki (T47) | [tasks](../../../../spec/tasks.md) |
| Akses darurat | Alasan tertulis, batas 60 menit, hanya-baca, peninjauan wajib oleh orang kedua, diaudit, dibagikan lewat Redis | tugas break-glass T44 |
| Audit | Rantai hash SHA-256 dalam transaksi yang sama, pemicu hanya-tambah, endpoint verifikasi; pembacaan diaudit | [audit](../../../../spec/audit.md) |
| Ketahanan terhadap penyalahgunaan | Pembatasan laju (600 per menit) dan penguncian (10 kegagalan dalam 5 menit), dibagikan lewat Redis; probe dikecualikan | T43 |
| Browser dan transportasi | CSP ketat dengan hash, penolakan frame, `nosniff`, kebijakan referrer, CORS tertutup, header aman; tidak ada token di JavaScript browser (BFF) | UI `hooks.server.ts` |
| Perlindungan data | AES-256-GCM pada dua kolom dengan rotasi; kunci lokal, tanpa KMS | T41 |
| Rantai pasok | `cargo audit`, `cargo deny`, `pnpm audit`, SBOM, image tanpa root yang diperkeras dan dikunci dengan digest, pembaruan Dependabot mingguan, alur kerja rilis yang menandatangani image dan SBOM tanpa kunci | [control-mapping](../../../../spec/compliance/control-mapping.md), [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Kesalahan | Setiap kesalahan adalah satu dokumen masalah RFC 9457; `401` menyebut skema yang diterima | [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) |
| Keamanan memori | `forbid(unsafe_code)`, clippy pedantic | `src/lib.rs` |

## Kelemahan yang diketahui (dinyatakan, tidak disembunyikan)

- **Tidak ada uji penetrasi independen atau latihan pemodelan ancaman** (T42). Model
  ancaman di [dtac-supporting](../../../../spec/compliance/dtac-supporting.md)
  adalah ringkasan, bukan yang terstruktur.
- **Penandatanganan rilis belum dijalankan pada rilis nyata** (T49). Alur kerja dan
  perintah verifikasinya ada; sampai tag didorong, semuanya belum teruji. Actions
  dikunci dengan hash commit.
- **Enkripsi bidang** mencakup dua kolom dengan kunci lokal, tanpa KMS.
- **Autentikasi ulang bertahap** sebelum tindakan destruktif belum dibangun.
- **Redis menyimpan data pribadi** (insight yang di-cache, kunci pemberian) dan harus dilindungi
  seperti pangkalan data: TLS, kata sandi, tidak terbuka ke publik.
- **Break-glass** tidak dapat dicabut, hanya kedaluwarsa, dan belum ada peringatan terpasang.
- **Pembatasan laju dan penguncian** kembali ke hitungan per replika jika Redis gagal; bucket
  tanpa autentikasi dihitung per alamat, dan di belakang proxy yang tidak tercantum di
  `TRUSTED_PROXIES` itu adalah milik proxy, sehingga penyerang dapat mengunci pemanggil
  tanpa autentikasi lainnya. Pemanggil dengan token valid dihitung menurut subjeknya.
- **Pemuatan ulang kebijakan** muncul di insight yang di-cache hingga 30 detik.
- **Keluar otomatis saat tidak aktif** berjalan di sisi klien.
- **Kunci idempotensi** bersifat opsional di API (T52): klien yang tidak mengirim
  `Idempotency-Key` tidak terlindungi dari pembuatan ulang yang menggandakan catatan.
  UI bawaan mengirimnya.

## Yang harus dilakukan penerap

1. Biarkan autentikasi menyala; sediakan kunci produksi dan `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL`.
2. Beri peringatan pada `break_glass_granted` dan tinjau pemberian dengan segera.
3. Tambatkan kepala rantai audit di luar pangkalan data dan verifikasi
   (`GET /api/audits/verify`) sesuai jadwal.
4. Lindungi PostgreSQL dan Redis (jaringan, TLS, kredensial, pencadangan, enkripsi
   saat disimpan); rotasi kunci bidang dengan tugas yang disediakan.
5. Gabungkan pembaruan Dependabot; jalankan ulang tahap rantai pasok; verifikasi rilis
   bertanda tangan sebelum menerapkannya.
6. Ikuti runbook untuk respons insiden
   ([operations-runbook](../../../../spec/compliance/operations-runbook.md)).

## Penanganan kerentanan

Laporan melalui [SECURITY.md](../../../../SECURITY.md). Pesan uji independen
(T42) terhadap build staging sebelum data nyata apa pun, dan uji ulang setelah perubahan material.

## Tindakan keamanan yang disarankan, berurutan

1. Tentukan lingkup dan pesan uji penetrasi independen (T42).
2. Dorong tag rilis pertama untuk membuktikan penandatanganan dan verifikasi (T49).
3. Pasang peringatan untuk break-glass dan untuk kegagalan verifikasi rantai audit.
4. Tempatkan Redis di belakang TLS dan kredensial di setiap lingkungan.
5. Kunci bidang berbasis KMS dan cakupan bidang yang lebih luas (sisa T41).

Lihat juga ringkasan [CTO](../direktur-teknologi/index.md) dan
ringkasan [CRO](../direktur-risiko/index.md).
