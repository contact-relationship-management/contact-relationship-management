# Tata Kelola, Risiko, Kepatuhan

> Ringkasan rekayasa, bukan nasihat hukum, sertifikasi, atau audit. Ringkasan ini menyatakan
> apa yang ditunjukkan repositori per 2026-10-11. Tidak ada yang di sini mengklaim akreditasi.

## Dari mana memulai

Paket bukti adalah [spec/compliance](../../../../spec/compliance/index.md). Aturan dasarnya adalah
yang harus dipegang tim: sebuah kontrol berstatus **Implemented** hanya
jika kode atau konfigurasi menegakkannya dan ada pengujian atau pemeriksaan yang menguncinya; **Deployer**
jika organisasi Anda yang harus menyediakannya; **Gap** jika keduanya tidak benar hari ini.

| Dokumen | Gunakan untuk |
|---|---|
| [control-mapping](../../../../spec/compliance/control-mapping.md) | NIST SP 800-53, HIPAA Security Rule, ISO 27001 Annex A, GDPR Pasal 32, masing-masing ditandai Implemented, Deployer, atau Gap |
| [dpia-template](../../../../spec/compliance/dpia-template.md) | Penilaian dampak perlindungan data, sudah terisi dengan alur data sistem ini |
| [operations-runbook](../../../../spec/compliance/operations-runbook.md) | Pengerasan, pencadangan dan pemulihan, kunci, pencatatan log, respons insiden |
| [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) | Draf pernyataan NHS DTAC untuk diadopsi produsen |
| [gds-api-review](../../../../spec/compliance/gds-api-review.md) | Tinjauan mandiri terhadap standar API pemerintah Inggris (draf, belum dicocokkan dengan teks yang diterbitkan) |
| [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) | Format kesalahan (RFC 9457), apa yang dianggap perubahan yang merusak, langkah penghentian dan masa pemberitahuan |
| [Penilaian NHS DTAC v2.0](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md) | Hasil per pertanyaan: kemungkinan di luar cakupan DTAC, dan akan gagal pada bagian yang berlaku karena kurangnya bukti organisasi |
| [plan](../../../../spec/plan.md) | Putusan kesiapan (belum siap) dan daftar celah |
| [tasks](../../../../spec/tasks.md) | Tugas setiap celah, penyelesaiannya, dan batas yang dinyatakan |

## Tata kelola

- **Hak keputusan:** [GOVERNANCE.md](../../../../GOVERNANCE.md), [MAINTAINERS.md](../../../../MAINTAINERS.md),
  [CODEOWNERS](../../../../CODEOWNERS). Aturan kontribusi dan bantuan AI ada di
  [CONTRIBUTING.md](../../../../CONTRIBUTING.md) dan [AI_STATEMENT.md](../../../../AI_STATEMENT.md).
- **Kontrol perubahan:** spesifikasi adalah sumber kebenaran; kode dan pengujian
  merujuknya. CI berjalan di dua penyedia, dan `scripts/ci-check.sh` mereproduksinya.
  Aturan di repositori: jangan melemahkan pengujian atau gerbang agar sesuatu lolos.
- **Pengungkapan:** [SECURITY.md](../../../../SECURITY.md) adalah jalur pelaporan.

## Risiko

Tampilan risiko ada di ringkasan [CRO](../direktur-risiko/index.md). Singkatnya: risiko
sisa tertinggi adalah keamanan yang belum diuji pihak luar (T42), aksesibilitas
yang belum terukur (T48), dan klasifikasi data, yang menjadi keputusan pengadopsi.

## Kepatuhan: apa yang nyata dan apa yang tidak

| Bidang | Posisi |
|---|---|
| Kontrol akses | Diterapkan dan diuji: masking berbasis peran, pemeriksaan tingkat catatan, pembacaan operasional khusus admin, break-glass dengan peninjauan wajib oleh orang kedua |
| Audit | Diterapkan dan diuji: berantai hash, hanya-tambah, dalam transaksi, audit pembacaan termasuk daftar, daftar transaksi, dan insight. **Deployer:** tambatkan kepala rantai di luar pangkalan data |
| Hak privasi | Diterapkan: ekspor akses subjek data, penghapusan (ditolak selama kewajiban terbuka), penyapuan retensi. **Deployer:** dasar hukum, jadwal retensi, catatan pemrosesan |
| Enkripsi | Sebagian: AES-256-GCM tingkat bidang pada dua kolom dengan kunci lokal, tanpa KMS. Enkripsi saat disimpan dan saat transit adalah tanggung jawab penerap |
| Rantai pasok | Diterapkan: `cargo audit`, `cargo deny`, `pnpm audit`, SBOM, image dasar dikunci dengan digest, pembaruan Dependabot. **Gap:** alur kerja rilis bertanda tangan belum dijalankan pada rilis nyata (T49) |
| Aksesibilitas | Sebagian: pemindaian axe otomatis dan pernyataan publik yang jujur. **Gap:** tidak ada audit manual (T48), terjemahan belum ditinjau |
| Jaminan independen | **Gap:** tidak ada (T42) |
| Hukum e-mel dan pemasaran | Jalur pengiriman yang mengutamakan persetujuan sudah diterapkan; pengiriman nyata hanya ada sebagai jalur SMTP untuk pengembangan; tautan berhenti berlangganan, daftar penekanan, dan webhook hasil bertanda tangan ada, tetapi belum ada penyedia yang diintegrasikan dan perjanjian pemroses serta tinjauan hukum masih terbuka (T39) |

## Kontrol yang memerlukan pemilik di organisasi Anda

1. Peninjauan dan peringatan untuk pemberian break-glass (beri peringatan pada `break_glass_granted`).
2. Menambatkan kepala rantai audit dan memverifikasinya sesuai jadwal
   (`GET /api/audits/verify`).
3. Bukti pencadangan dan pemulihan, dan gladi rotasi kunci.
4. Penanganan karyawan baru, pindah, dan keluar di penyedia identitas Anda.
5. Retensi untuk tabel audit, yang tumbuh seiring lalu lintas pembacaan.
6. Melindungi Redis seperti pangkalan data: ia menyimpan data insight yang di-cache.
7. Menggabungkan pembaruan Dependabot, dan memverifikasi rilis bertanda tangan sebelum menerapkannya.

## Apa yang harus ditolak untuk disetujui

Pernyataan apa pun bahwa sistem ini patuh, bersertifikat, patuh HIPAA, atau disetujui
NHS. Pernyataan yang dapat dipertanggungjawabkan lebih sempit: kontrol dipetakan, yang
sudah diterapkan diuji, dan celahnya disebutkan serta dilacak.
