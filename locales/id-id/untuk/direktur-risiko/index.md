# Untuk Direktur Risiko

> Ringkasan rekayasa, bukan nasihat hukum, sertifikasi, atau audit. Ringkasan ini menyatakan
> apa yang ditunjukkan repositori per 2026-10-11. Tidak ada yang di sini mengklaim akreditasi.

## Posisi risiko

**Secara keseluruhan: belum siap untuk data yang diatur.** Perangkat lunak ini terkendali dengan baik untuk
tahapnya, tetapi risiko sisa utamanya belum dimitigasi oleh pengujian dan proses
yang hanya dapat disediakan oleh organisasi pengadopsi atau pihak luar.

## Risiko utama

| # | Risiko | Dasar penilaian | Mitigasi di repositori | Sisa |
|---|---|---|---|---|
| 1 | Kerentanan yang belum ditemukan | Tidak ada uji penetrasi independen (T42) | Pengujian sendiri, gerbang rantai pasok, header/CSP yang ketat | **Tinggi** sampai diuji |
| 2 | Pengungkapan bidang sensitif kepada peran yang salah | Aturan akses yang kompleks | Masking berbasis peran pada catatan, agregat, dan insight; uji matriks persona; kebocoran pada detail akun ditemukan dan diperbaiki (T47) | Sedang: masking diuji, belum diverifikasi secara independen |
| 3 | Penyalahgunaan akses darurat oleh orang dalam | Break-glass meningkatkan hak baca | Alasan tertulis, batas 60 menit, hanya-baca, peninjauan wajib oleh orang kedua, diaudit | Sedang: belum ada peringatan terpasang; tidak ada pencabutan |
| 4 | Jejak audit diubah | Memerlukan akses basis data | Rantai hash ditambah pemicu hanya-tambah; melaporkan putusnya yang pertama | Rendah-Sedang: hash kepala harus ditambatkan di luar basis data |
| 5 | Pemasaran yang melanggar hukum | Kesalahan persetujuan | Jalur pengiriman menegakkan persetujuan, bukan UI; riwayat persetujuan yang tidak dapat diubah | Sedang sampai penyedia nyata diintegrasikan dan ditinjau (T39) |
| 6 | Ketersediaan / penyalahgunaan | API publik | Pembatasan laju dan penguncian, dibagikan lewat Redis | Sedang: batas kembali ke per replika jika Redis gagal |
| 7 | Ketidakpatuhan aksesibilitas | Kewajiban sektor publik | Pemindaian axe otomatis; pernyataan aksesibilitas yang jujur | **Belum terukur** sampai audit manual (T48) |
| 8 | Rantai pasok | Dependensi dan image | `cargo deny/audit`, `pnpm audit`, SBOM, kunci digest yang disegarkan Dependabot, alur kerja rilis bertanda tangan tanpa kunci | Sedang: penandatanganan belum dijalankan pada rilis nyata (T49) |
| 9 | Catatan duplikat akibat percobaan ulang | Kunci idempotensi bersifat opsional di API (T52) | Pemutaran ulang untuk klien yang mengirim kunci; UI bawaan selalu mengirimnya | Rendah |
| 10 | Salah klasifikasi data | Penerap yang menentukan apa itu "kontak" | Catatan cakupan dan templat DPIA | Tergantung Anda |

## Celah yang terbuka karena sifatnya

T39 (penyedia email nyata), T42 (jaminan independen), T48 (audit
aksesibilitas manual), T49 (rilis bertanda tangan pertama, pemeriksaan draf API oleh peninjau). Lihat [tasks.md](../../../../spec/tasks.md)
dan [plan.md](../../../../spec/plan.md).

## Tindakan risiko yang disarankan

Daftarkan risiko 1, 7, dan 10 secara resmi; wajibkan T42 sebelum data nyata apa pun; tetapkan
pemilik untuk peninjauan dan peringatan break-glass; tetapkan frekuensi penambatan kepala audit.
