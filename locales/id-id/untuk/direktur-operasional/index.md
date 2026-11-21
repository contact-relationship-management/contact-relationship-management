# Untuk Direktur Operasional

> Ringkasan rekayasa, bukan nasihat hukum, sertifikasi, atau audit. Ringkasan ini menyatakan
> apa yang ditunjukkan repositori per 2026-10-11. Tidak ada yang di sini mengklaim akreditasi.

## Apa yang diperlukan untuk menjalankannya

Layanan ini adalah satu proses Rust per replika ditambah PostgreSQL 18 dan Redis; front
end adalah proses Node. Keduanya dikirim sebagai image kontainer yang diperkeras dan tanpa root,
dengan image dasar yang dikunci dengan digest. Runbook operasi adalah
[operations-runbook.md](../../../../spec/compliance/operations-runbook.md).

## Fakta operasional yang perlu direncanakan

- **Gagal dengan aman saat mulai.** Di produksi layanan menolak mulai jika
  autentikasi mati, jika rantai audit tidak dapat dipakai, atau jika kunci yang diperlukan dan
  `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL` tidak ada. Penerapan yang salah konfigurasi berhenti, bukan berjalan terbuka.
- **Kesehatan dan metrik.** `/_health/live`, `/_health/ready`, `/_health/startup` tidak pernah
  dibatasi lajunya, sehingga penguncian tidak dapat memulai ulang pod yang sehat. Metrik ada di
  `/metrics.prom`.
- **Batas penyalahgunaan** aktif di produksi: 600 permintaan per menit per klien dan
  penguncian setelah 10 kali gagal masuk dalam 5 menit. Batas ini dibagikan antar
  replika lewat Redis; jika Redis gagal, kembali ke hitungan per replika.
- **Akses darurat** (break-glass): setiap pengguna yang sudah masuk dapat mengambil
  peningkatan hak baca beralasan hingga 60 menit. Orang lain harus meninjau setiap pemberian.
  **Proses yang harus Anda jalankan:** beri peringatan pada baris audit `break_glass_granted` dan
  tinjau jumlah yang belum ditinjau (halaman admin `/break-glass/review`).
- **Kesalahan** berbentuk tunggal (RFC 9457 `application/problem+json`, dengan `error` dan
  `description` dipertahankan), sehingga penguraian log dan aturan peringatan dapat mengandalkan `status` dan `type`.
- **Pertumbuhan audit.** Pembacaan diaudit, sehingga tabel audit tumbuh seiring lalu lintas.
  Tentukan ukuran retensinya. Klien yang melakukan polling akan membuatnya tumbuh cepat.

## Pekerjaan operasional yang masih terbuka

| Butir | Keadaan |
|---|---|
| Latihan pencadangan/pemulihan, gladi rotasi kunci | Tertulis di runbook; **Anda** harus menjalankan dan membuktikannya |
| Menambatkan kepala rantai audit di luar basis data | Tugas Anda; API melaporkan hash kepalanya |
| Menyegarkan digest image yang dikunci | Dependabot membuka pull request mingguan; **Anda** harus meninjau dan menggabungkannya atau kunci menjadi usang |
| Memverifikasi rilis bertanda tangan sebelum menerapkannya | Perintah ada di runbook; terapkan dengan digest, dan tolak image yang gagal verifikasi. Belum diuji sampai rilis bertanda pertama |
| Penyedia email nyata (T39) | Terbuka; mengubah gambaran insiden dan pemasok |
| Redis menyimpan data insight | Lindungi seperti basis data: TLS, kata sandi, tidak terbuka ke publik |

## Batasan yang diketahui dan perlu disampaikan kepada tim siaga

Keluar otomatis saat tidak aktif berjalan di sisi klien; token yang dicabut langsung ditolak, tetapi pemuatan ulang
kebijakan muncul di insight yang di-cache hingga 30 detik; break-glass tidak dapat
dicabut, hanya kedaluwarsa.
