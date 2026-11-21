# Contact Relationship Management

Perangkat lunak CRM sumber terbuka untuk penjualan, pemasaran, dan dukungan pelanggan.

Contact Relationship Management mencatat hubungan dengan pelanggan dan calon pelanggan di seluruh penjualan, pemasaran, dan dukungan. Perangkat lunak ini menyimpan satu catatan untuk setiap kontak dan mewajibkan persetujuan yang tercatat sebelum pesan dikirim. Perangkat lunak ini belum siap untuk produksi: ia berjalan dengan data sintetis.

## Manfaat

- **Satu catatan kontak.** Penjualan, pemasaran, dukungan, dan analitik memakai catatan yang sama.
- **Persetujuan tercatat.** Tidak ada yang dikirim kepada kontak tanpa persetujuan yang tercatat. Setiap e-mail menyertakan tautan berhenti berlangganan yang bertanda tangan.
- **Data pribadi yang lebih sedikit.** CRM menyimpan rujukan ke seseorang di layanan identitas, bukan salinan nama atau alamatnya.
- **Hasil yang dapat dijelaskan.** Skor prospek dihitung dari aturan yang dapat dibaca. Prakiraan dihitung dari tahap pipeline yang berbobot.
- **Rancangan internasional.** Nama, alamat, kalender, dan arah penulisan tidak terikat pada format satu negara. Antarmukanya tersedia dalam delapan bahasa.
- **Spesifikasi publik.** Spesifikasi, kode, dan pengujian bersifat publik. Dokumentasi menyatakan apa yang sudah dibangun dan apa yang belum.

## Fitur

- **Penjualan.** Penilaian prospek berbasis aturan, pipeline transaksi dalam bentuk papan, dan prakiraan berbobot tahap.
- **Pemasaran.** Kampanye tersegmentasi dan urutan pemeliharaan prospek. Pengiriman disimulasikan kecuali mode surat dikonfigurasi.
- **Dukungan.** Tiket dengan prioritas dan tenggat tingkat layanan, serta artikel basis pengetahuan.
- **Analitik.** Dasbor untuk tingkat kemenangan, pipeline per tahap, hasil kampanye, kesehatan tingkat layanan, dan nilai seumur hidup pelanggan.
- **Kualitas data.** Deteksi duplikat berdasarkan probabilitas, dengan penggabungan dan pembatalan.
- **Tata kelola.** Log audit, akses berbasis peran, dan dukungan untuk permintaan akses subjek data dan penghapusan.

## Memulai

1. **Dapatkan kodenya.** Klon repositori. Node.js dan pnpm diperlukan.
2. **Jalankan aplikasi.** Di direktori Svelte, jalankan `pnpm install`, lalu `pnpm dev`.
3. **Masuk.** Gunakan `example@example.com`. Tidak ada layanan lain yang diperlukan.
4. **Tinjau tur.** Tur mencakup kontak, prospek, transaksi, kampanye, tiket, dan tampilan eksekutif, dengan data fiktif.

[Tutorial](../../docs/tutorial-first-tour.md) menjelaskan setiap langkah. [Pasang dan gunakan](../../INSTALL.md) mencakup layanan secara lengkap.

## Perbandingan

| Proyek | Kategori | Perbedaan |
| --- | --- | --- |
| [Twenty](../../docs/compare/twenty.md) | CRM sumber terbuka | Produk lengkap. Proyek ini adalah demonstrasi yang lebih kecil dan digerakkan oleh spesifikasi. |
| [EspoCRM](../../docs/compare/espocrm.md) | CRM sumber terbuka | Fitur lebih luas. Proyek ini menekankan aturan yang dapat dijelaskan. |
| [SuiteCRM](../../docs/compare/suitecrm.md) | CRM sumber terbuka | Matang dan besar. Proyek ini adalah rujukan yang ringkas. |
| [Salesforce](../../docs/compare/salesforce.md), [HubSpot](../../docs/compare/hubspot.md) | CRM SaaS komersial | Proyek ini memodelkan praktik yang sama. Tidak ada kode atau data yang dibagikan. |

Proyek ini adalah demonstrasi dan tidak bersaing dengan produk-produk tersebut. Lihat [semua perbandingan](../../COMPARISONS.md).

## Ringkasan peran

[Ringkasan peran](untuk/) menyajikan satu halaman untuk setiap peran senior: apa yang dilakukan perangkat lunak, apa yang tidak dilakukannya, dan keputusan mana yang menjadi tanggung jawab peran itu.

## Status

Perangkat lunak ini belum siap untuk produksi ([jalur ke produksi](jalur-ke-produksi/index.md)). Gunakan hanya data sintetis. Tidak ada e-mail nyata yang dikirim kecuali mode surat nyata dikonfigurasi. Tidak ada yang ada di situs ini yang merupakan nasihat hukum.
