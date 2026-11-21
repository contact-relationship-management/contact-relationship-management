# Penguji Keamanan Independen

> Ringkasan rekayasa, bukan nasihat hukum, sertifikasi, atau audit. Ringkasan ini menyatakan
> apa yang ditunjukkan repositori per 2026-10-11. Tidak ada yang di sini mengklaim akreditasi.

## Mengapa ini ada

Setiap kontrol keamanan dalam proyek ini dibangun dan diuji oleh penulisnya
(CRM-T42). Itu menunjukkan kontrol berfungsi seperti yang dipahami penulisnya. Itu tidak
menunjukkan bahwa tidak ada yang terlewat. Rumah sakit dan pembeli pemerintah biasanya mewajibkan
uji oleh pihak independen sebelum mereka menyimpan data nyata, dan klaim yang jujur
sampai saat itu adalah "dibangun dengan standar tinggi, belum diuji pihak luar"
([ringkasan CEO](../direktur-utama/index.md),
[ringkasan keamanan](../direktur-keamanan/index.md)).

Halaman ini adalah apa yang diserahkan kepada firma penguji, dan apa yang diharapkan sebagai balasannya.

## Arti lolos, dan apa yang bukan

Laporan yang bersih menyatakan bahwa tim bernama, dalam waktu tertentu, dengan lingkup tertentu,
tidak menemukan masalah serius. Itu **bukan** sertifikasi, bukan akreditasi, dan bukan
jaminan. Jangan menggambarkan sistem sebagai "aman", "bersertifikat", atau "diuji penetrasi
sesuai standar" melebihi yang dinyatakan laporan
([ringkasan GRC](../tata-kelola-risiko-kepatuhan/index.md)).

## Memilih penguji

- Independen dari para penulis, dan dari siapa pun yang mengonfigurasi penerapan.
- Diisi penguji bernama yang berpengalaman, dengan akreditasi yang relevan untuk
  pasar Anda bila ada (misalnya CREST, atau skema NCSC CHECK untuk
  pekerjaan pemerintah Inggris). Tanyakan akreditasi mana yang mencakup individu yang
  mengerjakan, bukan hanya perusahaannya.
- Menyetujui secara tertulis lingkup, aturan keterlibatan, kerahasiaan, penanganan
  data, dan tanggung jawab hukum sebelum pengujian apa pun dimulai. Kontrak itu
  disepakati oleh tim hukum ([ringkasan CLO](../direktur-hukum/index.md)).

## Apa yang diuji

| Bidang | Apa yang harus dicoba penguji | Tempat rancangannya dijelaskan |
|---|---|---|
| Autentikasi | Token yang dipalsukan, kedaluwarsa, diputar ulang, dan beraudiens salah; penurunan ke tanpa autentikasi; pemeriksaan awal yang menolak berjalan dengan autentikasi mati | [auth](../../../../spec/auth.md) |
| Otorisasi | Mencapai catatan atau tindakan yang tidak semestinya bagi sebuah peran: eskalasi horizontal dan vertikal, pemeriksaan tingkat catatan, tindakan destruktif, pembacaan khusus admin (audit, retensi, peninjauan break-glass) | [auth](../../../../spec/auth.md) |
| Pemaparan data | Nilai yang dimasking bocor lewat daftar, pembacaan detail gabungan, insight, agregat, pesan kesalahan, caching, atau kunci cache | [tasks](../../../../spec/tasks.md) (T32, T47) |
| Akses darurat | Mengambil, memperpanjang, atau menyalahgunakan break-glass; meninjau pemberian sendiri; meninjau dua kali | [auth](../../../../spec/auth.md) (T44) |
| Audit | Mengubah, menghapus, atau menyusun ulang riwayat; melewati rantai; menyembunyikan pembacaan | [audit](../../../../spec/audit.md) |
| Ketahanan terhadap penyalahgunaan | Melewati pembatasan laju dan penguncian, mengunci pengguna lain, pemalsuan header untuk kunci klien | T43 |
| Penanganan input | Injeksi (SQL, JSON, header), badan permintaan terlalu besar, kursor dan kunci idempotensi yang cacat, memutar ulang atau menabrakkan kunci antar pemanggil | T51, T52 |
| Front end web | Cross-site scripting, pemalsuan permintaan, penanganan sesi, proxy backend-for-frontend, kebijakan keamanan konten, clickjacking | UI `hooks.server.ts` |
| Infrastruktur sebagaimana dikirim | Image kontainer, port terbuka, keterjangkauan Redis dan PostgreSQL, penanganan rahasia, bawaan konfigurasi | [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Rantai pasok | Asal-usul dependensi dan image, SBOM, build tanpa tanda tangan | [control-mapping](../../../../spec/compliance/control-mapping.md) |
| Logika bisnis | Melewati persetujuan di jalur pengiriman, penghapusan yang meninggalkan data, manipulasi penilaian, pembuatan duplikat | [regulatory](../../../../spec/regulatory.md) |

**Di luar lingkup kecuali Anda sepakati lain:** layanan identitas yang menerbitkan
token (berada di luar repositori ini), platform hosting, dan penolakan layanan
bervolume besar.

## Apa yang diberikan kepada penguji

1. **Build staging** yang sesuai dengan apa yang akan berjalan di produksi: konfigurasi
   produksi, autentikasi menyala, TLS, Redis dan PostgreSQL yang nyata. Jangan
   menguji bawaan demo.
2. **Hanya data sintetis.** Tugas seed membangun portofolio bisnis tanpa orang nyata.
   Data pasien atau warga yang nyata tidak boleh ada di lingkungan pengujian.
3. **Satu akun per peran**, sehingga otorisasi dapat diuji dari setiap sisi. Kebijakan
   rujukannya adalah `config/abac-policy.reference.json`.
4. Dokumen yang ditautkan di atas, dokumen OpenAPI di `/api-docs/openapi.json`,
   dan akses baca ke repositori, sehingga pengujian dapat menggabungkan kotak hitam dan tinjauan
   kode.
5. **Kelemahan yang diketahui**, agar penguji menghabiskan waktu pada hal yang belum
   tertulis:
   - penandatanganan rilis sudah dibangun tetapi belum dijalankan pada rilis nyata (T49);
   - enkripsi bidang mencakup dua kolom dengan kunci lokal, tanpa KMS;
   - autentikasi ulang bertahap belum dibangun;
   - break-glass tidak dapat dicabut, hanya kedaluwarsa, dan belum ada peringatan terpasang;
   - pemuatan ulang kebijakan muncul di insight yang di-cache hingga 30 detik;
   - keluar otomatis saat tidak aktif berjalan di sisi klien;
   - status pembatasan laju kembali ke per replika jika Redis gagal.
   Minta mereka mengonfirmasi atau membantah masing-masing, bukan melewatkannya.

## Aturan keterlibatan yang disepakati secara tertulis

- Penguji bernama, tanggal, dan alamat sumber; kontak 24 jam di pihak Anda.
- Apa yang boleh dicoba (misalnya, tidak merusak penyimpanan data uji
  tanpa pemberitahuan) dan apa yang harus berhenti pada bukti pemaparan.
- Siapa yang segera diberi tahu tentang temuan kritis, dan bagaimana, sebelum laporan akhir.
- Bagaimana laporan, bukti, dan catatan penguji disimpan dan dimusnahkan.

## Apa yang harus Anda terima kembali

- Laporan dengan lingkup, metode, tanggal, nama penguji, dan akreditasi.
- Setiap temuan dengan tingkat keparahan, bukti, langkah reproduksi, komponen
  yang terdampak, dan perbaikan yang direkomendasikan.
- Pernyataan jelas tentang apa yang **tidak** diuji.
- Uji ulang temuan yang sudah diperbaiki, termasuk dalam keterlibatan atau diberi harga.

## Setelah laporan

1. Triase setiap temuan di `spec/tasks.md` sebagai tugas dengan pemilik dan tanggal.
2. Perbaiki temuan kritis dan tinggi sebelum data nyata apa pun, lalu minta penguji
   mengonfirmasi perbaikannya.
3. Tambahkan uji regresi untuk setiap perbaikan. Aturan proyek adalah tidak melemahkan pengujian
   atau gerbang agar sesuatu lolos.
4. Catat hasilnya dengan jujur: tanggal laporan, lingkup, temuan, dan
   statusnya. Perbarui rencana kesiapan ([plan](../../../../spec/plan.md)) dan
   paket kepatuhan ([compliance](../../../../spec/compliance/index.md)).
5. Uji ulang setelah perubahan material, dan menurut jadwal yang ditetapkan pemilik risiko Anda.
   Satu uji adalah potret, bukan keadaan.

## Urutan pekerjaan

1. Selesaikan dulu yang murah dan sudah diketahui, agar penguji tidak menghabiskan anggaran
   untuk menemukannya kembali: rilis bertanda tangan pertama (T49), peringatan untuk break-glass
   dan untuk kegagalan verifikasi audit, Redis di belakang TLS dan kredensial.
2. Siapkan build staging, akun, dan data seed.
3. Sepakati lingkup, kontrak, dan aturan keterlibatan.
4. Uji; terima temuan kritis segera.
5. Perbaiki, uji ulang, dan catat.
6. Baru kemudian perbarui klaim eksternal, dan hanya sebatas yang didukung laporan.

## Keputusan untuk Anda

1. Siapa yang memesan dan siapa yang memiliki temuan (lihat
   [ringkasan CRO](../direktur-risiko/index.md)).
2. Akreditasi mana yang diharapkan pasar Anda dari penguji.
3. Anggaran dan tanggal, ditentukan oleh biaya penundaan, bukan oleh apa yang
   nyaman.
4. Apakah akan menguji sekali sebelum peluncuran atau secara berkala.
