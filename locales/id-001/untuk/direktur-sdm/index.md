# Untuk Direktur SDM

> Ringkasan rekayasa, bukan nasihat hukum, sertifikasi, atau audit. Ringkasan ini menyatakan
> apa yang ditunjukkan repositori per 2026-10-11. Tidak ada yang di sini mengklaim akreditasi.

## Mengapa manusia penting bagi produk ini

Akses ke data pribadi ditentukan oleh peran. Perangkat lunak menegakkan apa yang dikatakan token
seseorang; **siapa mendapat peran apa, dan kapan peran itu dicabut, adalah proses
sumber daya manusia** yang tidak dapat dilakukan repositori ini untuk Anda.

## Apa yang diberikan perangkat lunak kepada proses Anda

- **Visibilitas berbasis peran:** sebagian peran melihat nilai yang dimasking untuk saluran kontak,
  jumlah transaksi, dan tampilan uang; manajer, admin, dan tingkat layanan melihat lebih banyak
  ([auth](../../../../spec/auth.md)).
- **Pembacaan yang diaudit:** membaca data pribadi meninggalkan jejak yang menyebut orangnya.
  Staf harus diberi tahu hal ini sebelum go-live.
- **Akses darurat:** seseorang dapat mengambil peningkatan hak baca beralasan hingga 60
  menit; orang lain harus meninjaunya setelahnya. Latih kedua pihak.
- **Keluar otomatis saat tidak aktif** setelah 15 menit di browser.
- **Aksesibilitas:** pernyataan publik dan pemeriksaan otomatis; audit manual
  (T48) masih memerlukan pengguna penyandang disabilitas dan penguji teknologi bantu.
- **Bahasa:** UI diterjemahkan ke 8 lokal; terjemahannya
  **belum** ditinjau oleh penutur asli.

## Tanggung jawab Anda

| Bidang | Tindakan |
|---|---|
| Karyawan baru, pindah, dan keluar | Berikan dan cabut peran lewat penyedia identitas Anda; CRM menolak token kedaluwarsa tetapi tidak mengelola akun |
| Pelatihan | Perlindungan data, aturan persetujuan, penggunaan dan peninjauan break-glass, praktik yang aksesibel |
| Kebijakan | Penggunaan yang dapat diterima, pemberitahuan pemantauan (pembacaan diaudit), jalur disipliner untuk penyalahgunaan akses darurat |
| Kapasitas peninjau | Tentukan siapa yang meninjau pemberian break-glass dan penggantinya saat berhalangan |
| Rekrutmen | Peran yang dibutuhkan program: penghubung penguji independen, petugas keselamatan klinis jika dalam cakupan NHS, pimpinan perlindungan data, pimpinan aksesibilitas |
| Kesejahteraan | Dukung staf yang menangani kasus sensitif; CRM bukan sistem manajemen kasus tetapi dapat menyimpan data hubungan yang sensitif |

## Butir terbuka yang memerlukan manusia

Audit aksesibilitas manual dengan pengguna teknologi bantu (T48), tinjauan
terjemahan oleh penutur asli, dan kepemilikan tugas peninjauan dan peringatan di atas.
Lihat [tasks.md](../../../../spec/tasks.md).
