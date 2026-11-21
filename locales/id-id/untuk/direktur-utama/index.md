# Untuk Direktur Utama

> Ringkasan rekayasa, bukan nasihat hukum, sertifikasi, atau audit. Ringkasan ini menyatakan
> apa yang ditunjukkan repositori per 2026-10-11. Tidak ada yang di sini mengklaim akreditasi.

## Keputusan dalam satu paragraf

Ini adalah CRM kelas demo dengan fondasi keamanan dan privasi yang luar biasa kuat. Ia **belum siap**
menyimpan data pasien atau warga yang nyata, dan repositori sendiri menyatakannya
([plan.md](../../../../spec/plan.md)). Mengadopsinya berarti memutuskan untuk mendanai pekerjaan eksternal yang
tersisa, bukan menyalakannya begitu saja.

## Apa yang sudah ada

- CRM yang berfungsi di seluruh penjualan, pemasaran, dukungan, dan analitik, dengan API Rust
  dan front end web, dibangun menurut spesifikasi tertulis.
- Pemasaran yang mengutamakan persetujuan, kontrol akses yang menyembunyikan nilai sensitif dari
  peran yang tidak semestinya melihatnya, jejak audit yang menunjukkan pengubahan, akses subjek data
  dan penghapusan untuk permintaan privasi, serta akses darurat ("break-glass")
  yang beralasan, dibatasi waktu, dan ditinjau.
- 54 dari 58 butir pekerjaan yang dilacak sudah selesai ([tasks.md](../../../../spec/tasks.md)).

## Apa yang menghalangi pemakaian nyata

| Penghalang | Mengapa penting | Siapa yang dapat memperbaikinya |
|---|---|---|
| Tidak ada pengiriman email nyata (T39) | Pemasaran disimulasikan; penyedia nyata memerlukan kontrak dan perjanjian pemrosesan data | Anda memilih vendor; bagian hukum menandatangani |
| Tidak ada uji keamanan independen (T42) | Rumah sakit dan pembeli pemerintah biasanya mewajibkannya; pengujian kami sendiri tidak independen | Firma luar |
| Tidak ada audit aksesibilitas manual (T48) | Lembaga publik harus menerbitkan pernyataan aksesibilitas yang dapat mereka pertanggungjawabkan | Orang yang memakai teknologi bantu |
| Penandatanganan rilis sudah dibangun tetapi belum dijalankan pada rilis nyata, dan belum ada peninjau yang memeriksa draf standar API (T49) | Daftar periksa pengadaan menanyakan keduanya | Rekayasa (menandai rilis pertama); peninjau luar |

## Keputusan yang menjadi milik Anda

1. **Pasar mana lebih dulu.** Penilaian NHS Digital Technology Assessment Criteria
   ([dokumen](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md))
   menyimpulkan produk ini kemungkinan berada di luar cakupan DTAC dan akan gagal pada
   bagian yang berlaku karena kurangnya bukti organisasi (petugas keselamatan klinis
   yang ditunjuk, sertifikasi). Pilih jalurnya sebelum mengeluarkan biaya.
2. **Apakah akan mendanai jaminan independen** (T42, T48). Tanpanya klaim yang jujur adalah
   "dibangun dengan standar tinggi, belum diuji pihak luar".
3. **Siapa yang memegang risiko** begitu data nyata disimpan. Lihat ringkasan [CRO](../direktur-risiko/index.md)
   dan [CLO](../direktur-hukum/index.md).

## Apa yang tidak boleh dikatakan ke luar

Jangan mengklaim kepatuhan, sertifikasi, "patuh HIPAA", atau "disetujui NHS".
Paket bukti ([kepatuhan](../../../../spec/compliance/index.md)) mendukung pernyataan yang lebih sempit
dan benar: kontrol dipetakan, yang sudah diterapkan diuji, celah disebutkan.
