# Untuk Direktur Strategi

> Ringkasan rekayasa, bukan nasihat hukum, sertifikasi, atau audit. Ringkasan ini menyatakan
> apa yang ditunjukkan repositori per 2026-10-11. Tidak ada yang di sini mengklaim akreditasi.

## Pertanyaan strategis

Untuk apa ini? Saat ini ia adalah **CRM rujukan yang mengutamakan spesifikasi** dengan lapisan keamanan,
privasi, dan audit yang luar biasa lengkap. Ia bukan produk dengan pelanggan, dan ia tidak bersaing
dengan CRM yang matang
([COMPARISONS.md](../../../../COMPARISONS.md)). Pilihan di hadapan Anda adalah
apakah akan tetap menjadikannya rujukan, atau berinvestasi agar dapat diterapkan di
lingkungan yang diatur.

## Apa yang benar-benar berbeda

| Kekuatan | Mengapa bisa penting | Seberapa kokoh |
|---|---|---|
| Mengutamakan spesifikasi: satu spesifikasi menggerakkan layanan Rust dan UI Svelte ([spec](../../../../spec/index.md)) | Lebih murah untuk diaudit, dipindahkan, dan dijaga konsistensinya | Nyata; pengujian merujuk spesifikasi |
| Pemasaran yang mengutamakan persetujuan, ditegakkan di jalur pengiriman | Syarat mutlak bagi pembeli sektor kesehatan dan publik | Nyata, tetapi belum ada pengirim nyata (T39) |
| Penilaian prospek dan prakiraan berbasis aturan yang dapat dijelaskan | Pembeli yang harus membenarkan keputusan tentang orang | Nyata; sengaja tanpa pembelajaran mesin |
| Kontak diambil dari layanan identitas, tidak diduplikasi | Lebih sedikit data pribadi yang disimpan, permukaan pembobolan lebih kecil | Nyata; bergantung pada keberadaan layanan itu |
| Audit yang menunjukkan pengubahan, masking menurut peran, akses darurat beralasan | Kontrol yang pertama ditanyakan pembeli yang diatur | Diterapkan dan diuji; belum diverifikasi secara independen |

## Apa yang tidak berbeda

Keluasan fitur. Penawaran harga, kontrak, telepon, penilaian pembelajaran mesin, dan
pembayaran berada di luar cakupan ([scope](../../../../spec/scope.md)). Siapa pun yang memilih
CRM berdasarkan fitur akan memilih yang lain.

## Opsi

| Opsi | Artinya | Biaya dan risiko | Bukti yang diperlukan |
|---|---|---|---|
| A. Tetap menjadi rujukan | Pertahankan sebagai pustaka pengajaran dan pola | Terendah; nilainya reputasi | Tidak ada selain yang ada hari ini |
| B. Ceruk sektor yang diatur | Selesaikan butir terbuka dan jual atau dukung untuk lembaga kesehatan dan publik | Tertinggi; memerlukan vendor, ketentuan hukum, pengujian luar, jaminan berkelanjutan | Uji independen (T42), audit aksesibilitas (T48), e-mel nyata (T39) |
| C. Komponen, bukan produk | Tawarkan rantai audit, masking, dan pola break-glass sebagai bagian yang dapat dipakai ulang | Sedang; memerlukan pengemasan dan dokumentasi | Pekerjaan ekstraksi; tanpa sertifikasi baru |

**Rekomendasi: lakukan A sekarang, dan danai uji termurah untuk B sebelum berkomitmen.**
Penilaian NHS menyimpulkan produk ini kemungkinan berada di luar cakupan DTAC
([penilaian](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md)),
sehingga jalur ke pasar belum pasti. Keluarkan biaya lebih dulu untuk mencari tahu pembeli mana
yang membutuhkan ini dan apa yang akan mereka syaratkan, bukan untuk sertifikasi yang tidak diminta siapa pun.

## Peta jalan, dalam urutan yang disarankan bukti

1. Uji keamanan independen dan audit aksesibilitas manual (T42, T48): keduanya
   menentukan apakah B mungkin sama sekali.
2. Penyedia e-mel nyata (T39) setelah vendor dan ketentuan pemrosesan data
   dipilih; seam pengirim dan jalur SMTP sudah ada dan berjalan terhadap server uji lokal, dengan tautan berhenti berlangganan bertanda tangan, daftar penekanan, dan webhook hasil bertanda tangan generik, tetapi belum ada penyedia yang diintegrasikan ([roadmap](../../../../spec/roadmap.md)).
3. Buktikan penandatanganan rilis dengan rilis bertanda pertama (T49): syarat minimum
   pengadaan. Alur kerjanya sudah dibangun; belum dijalankan.
4. Baru kemudian fitur produk di peta jalan (atribusi, wilayah,
   jam SLA berdasarkan jam kerja, portal basis pengetahuan publik).

## Risiko strategis

- **Mengklaim lebih dari yang benar.** Cara tercepat kehilangan pembeli yang diatur adalah
  klaim kepatuhan yang tidak didukung bukti. Lihat ringkasan [GRC](../tata-kelola-risiko-kepatuhan/index.md).
- **Pemeliharaan.** Pekerjaan keamanan tidak pernah selesai: pembaruan dependensi, penyegaran
  digest, rotasi kunci, pengujian ulang. Opsi B mengikat Anda padanya.
- **Ketergantungan pada layanan identitas** yang berada di luar repositori ini.
- **Satu pemelihara.** Proyek ini hanya memiliki satu pemelihara dan menyatakan bahwa kedalaman
  tinjauan adalah milik satu orang ([MAINTAINERS.md](../../../../MAINTAINERS.md),
  [GOVERNANCE.md](../../../../GOVERNANCE.md)). Opsi B memerlukan lebih dari satu orang
  yang dapat bertanggung jawab atas pekerjaan keamanan.

## Keputusan untuk Anda

1. Mana yang dipilih di antara A, B, atau C, dan tanggal peninjauan pilihan itu.
2. Siapa target pembeli pertama, ditemukan lewat percakapan sebelum mengeluarkan biaya.
3. Apakah akan mendanai jaminan independen sekarang (lihat ringkasan [CEO](../direktur-utama/index.md)).
