# Jalur ke produksi

> Contact Relationship Management belum siap untuk produksi. Halaman ini menjelaskan apa arti produksi di sini, dan
> sejauh mana setiap bagian telah berkembang.

Perangkat lunak ini berjalan dengan data sintetis. Email disimulasikan kecuali mode surat nyata dikonfigurasi. Jangan
memasukkan data pribadi nyata ke dalamnya.

## Apa arti produksi

Penerapan produksi menyimpan data orang sungguhan dan mengirim pesan nyata kepada mereka. Untuk proyek ini, itu
memerlukan semua hal berikut:

- **Masuk yang nyata.** Setiap permintaan diautentikasi, dan aturan akses sesuai dengan peran organisasi yang
  menerapkannya.
- **Pengiriman yang sah.** Setiap pesan memiliki persetujuan tercatat dan dasar hukum menurut hukum setiap tempat
  tujuan pengiriman, dan dikirim melalui penyedia email nyata yang menangani pantulan, keluhan, dan berhenti
  berlangganan.
- **Jaminan independen.** Pihak di luar proyek telah menguji keamanan dan aksesibilitas, dan temuannya sudah
  diperbaiki atau diterima.
- **Operasi.** Pencadangan yang sudah dipulihkan, pemantauan, proses insiden, dan seseorang yang siaga.
- **Bukti stabilitas.** Pengoperasian panjang di bawah beban realistis, pada perangkat keras seperti milik penerapan,
  tanpa kesalahan atau perlambatan.

Kesiapan produksi diputuskan per penerapan. Penerap dapat menutup sebagian gerbang hanya untuk organisasi dan
penerimanya sendiri. Penilaian rekayasa di balik halaman ini adalah [spec/plan.md](../../../spec/plan.md). Gerbang
yang masih terbuka ada di [spec/tasks.md](../../../spec/tasks.md).

## Kesiapan

Setiap persentase adalah perkiraan yang dibuat pada 2026-11-13 dari daftar tugas, bukan pengukuran.

- **Autentikasi dan kontrol akses** (70%): Masuk dengan token PASETO atau Keycloak/OIDC, aturan akses berbasis atribut
  dengan pemeriksaan tingkat catatan, dan mode produksi yang menolak mulai tanpa autentikasi sudah dibangun. Belum ada
  penerapan nyata yang mengaktifkannya dengan atributnya sendiri, dan matriks penegakan belum mencakup setiap handler
  (CRM-G1).
- **Pengiriman email** (40%): Pengirim simulasi adalah bawaannya, dan pengirim SMTP tersedia. Belum ada adaptor
  penyedia email dengan webhook pantulan dan keluhan atau daftar penekanan (CRM-T39).
- **Perlindungan data dan hukum pemasaran elektronik** (15%): Catatan persetujuan, akses subjek data, penghapusan, dan
  retensi sudah dibangun. Hukum suatu wilayah belum dibaca untuk proyek ini, dan tinjauan jalur pengiriman nyata masih
  terbuka (CRM-G2, [regulasi](../../../spec/regulatory.md)).
- **Jaminan keamanan independen** (10%): Lingkup uji penetrasi sudah ditulis. Belum ada uji penetrasi, analisis risiko
  formal, atau akreditasi yang dilakukan (CRM-T42). Audit keamanan AI bukan jaminan independen.
- **Aksesibilitas** (50%): Pemeriksaan otomatis axe-core terhadap WCAG 2.2 A dan AA lolos di setiap rute halaman.
  Belum ada audit manual dengan penggunaan papan ketik saja, pembaca layar, atau zoom 400% (CRM-T48).
- **Terjemahan** (30%): Antarmuka tersedia dalam delapan bahasa. String dalam bahasa selain Inggris ditulis oleh mesin,
  dan belum ada penutur yang meninjaunya.
- **Operasi** (30%): Runbook operasi mencakup pencadangan dan pemulihan, manajemen kunci, enkripsi saat disimpan,
  respons insiden, dan retensi log. Belum ada pemulihan yang diuji, dan daftar periksa go-live runbook masih terbuka.
- **Kinerja dan stabilitas** (20%): Uji beban dan uji stabilitas jangka panjang tersedia. Uji stabilitas berjalan
  selama dua menit terhadap tumpukan demo; belum ada pengoperasian panjang dan belum ada tolok ukur pada perangkat
  keras yang menyerupai produksi.
- **Integritas rilis** (85%): Image rilis berarsitektur ganda, bertanda tangan dan beratestasi, dan tanda tangannya
  terverifikasi. Actions dan image dikunci dengan digest, dan riwayat dipindai untuk rahasia. Separuh amd64 dari image
  rilis yang ditarik belum diperiksa, dan belum ada host kode kedua (CRM-T192).
- **Integrasi berkelanjutan** (100%): Satu skrip menjalankan setiap pemeriksaan di GitHub Actions dan Woodpecker.
  Kedua basis kode Rust dan UI memiliki batas bawah 95% baris; gerbang browser, lint, rantai pasok, mutasi pada baris
  yang berubah, dan mutasi penuh mingguan sudah ada (CRM-T94).
- **Pengaman demo** (100%): Setiap pengunjung tur mendapat salinan data demo yang terpisah, dibatasi 200 salinan. Tur
  nonaktif secara bawaan; setel `CONTACT_RELATIONSHIP_MANAGEMENT_DEMO_TOUR=on` untuk menawarkannya (CRM-T92).

## Cara halaman ini dijaga

Saat sebuah gerbang tertutup, ubah perkiraan dan buktinya dalam commit yang sama dengan pekerjaannya, dan tautkan tugasnya.
