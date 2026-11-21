# Untuk Direktur Hukum

> Ringkasan rekayasa, bukan nasihat hukum, sertifikasi, atau audit. Ringkasan ini menyatakan
> apa yang ditunjukkan repositori per 2026-10-11. Tidak ada yang di sini mengklaim akreditasi.

## Sikap hukum secara singkat

Repositori ini **belum siap untuk produksi** dan menyimpan data sintetis. Ia tidak mengklaim
kepatuhan terhadap rezim mana pun. Ia menerapkan fitur teknis yang mendukung
beberapa rezim, dan mencantumkan apa yang harus ditambahkan penerap ([regulatory](../../../../spec/regulatory.md)).

## Rezim yang tersentuh

| Rezim | Yang didukung perangkat lunak | Yang harus Anda sediakan |
|---|---|---|
| GDPR Inggris/UE | Riwayat persetujuan, ekspor akses subjek data, penghapusan (ditolak selama ada kewajiban terbuka), penyapuan retensi, minimisasi data, audit pembacaan | Dasar hukum per tujuan, DPIA ([templat](../../../../spec/compliance/dpia-template.md)), catatan pemrosesan, jadwal retensi, peran pengendali/pemroses |
| PECR / e-privacy | Pengiriman yang dibatasi persetujuan, berhenti berlangganan seketika | Identifikasi pengirim dan keputusan soft-opt-in untuk pengirim nyata |
| HIPAA (jika data kesehatan AS) | Fitur kontrol audit, kontrol akses, integritas, keamanan transmisi | Perjanjian rekanan bisnis, analisis risiko, kebijakan, pelatihan tenaga kerja. Perangkat lunak tidak membuat Anda patuh |
| NHS DTAC / DCB0129/0160 | Draf bukti ([dtac-supporting](../../../../spec/compliance/dtac-supporting.md)) | Petugas keselamatan klinis yang ditunjuk, log bahaya, sertifikasi. Penilaian menyatakan DTAC kemungkinan tidak berlaku |
| Aksesibilitas (WCAG 2.2 AA, sektor publik) | Pemeriksaan otomatis dan pernyataan yang jujur | Audit manual (T48) sebelum klaim kepatuhan apa pun |
| Keamanan pemerintah (NIST 800-53, ISO 27001, Cyber Essentials) | Pemetaan kontrol ([control-mapping](../../../../spec/compliance/control-mapping.md)) | Sertifikasi dan jaminan independen |

## Kontrak yang akan Anda perlukan tetapi belum ada

1. Perjanjian pemrosesan data dan ketentuan transfer internasional dengan penyedia e-mel (T39).
2. Surat perintah kerja dan ketentuan kerahasiaan dengan firma uji penetrasi (T42).
3. Ketentuan hosting dan sub-pemroses untuk PostgreSQL, Redis, dan log.
4. Lisensi: lihat [LICENSE.md](../../../../LICENSE.md); repositori juga memuat
   pernyataan keamanan, tata kelola, dan penggunaan AI.

## Hal-hal hukum yang perlu diwaspadai

- **Penghapusan** menganonimkan kontak dan ditolak selama ada transaksi, tiket,
  atau pendaftaran pemeliharaan prospek yang aktif; rantai audit menyimpan catatan tindakan,
  jadi tetapkan dalam DPIA Anda bagaimana hal itu berinteraksi dengan permintaan penghapusan.
- **Break-glass** pembacaan data yang dimasking sah hanya atas dasar yang Anda dokumentasikan;
  kolom alasan adalah bukti, bukan kewenangan.
- **Penilaian** prospek: jika suatu saat memengaruhi keputusan penting tentang
  orang, aturan keputusan otomatis berlaku ([regulatory](../../../../spec/regulatory.md)).
- **Komitmen API:** [api-lifecycle](../../../../spec/compliance/api-lifecycle.md)
  menetapkan masa pemberitahuan 180 hari sebelum perubahan API dihapus. Itu
  pilihan pemelihara; jika pelanggan akan bergantung padanya, putuskan apakah itu perlu
  masuk ke kontrak.
- **Klaim publik:** jangan menyatakan kepatuhan, sertifikasi, atau persetujuan.
