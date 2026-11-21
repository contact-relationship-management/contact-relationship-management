# Untuk Direktur Informasi

> Ringkasan rekayasa, bukan nasihat hukum, sertifikasi, atau audit. Ringkasan ini menyatakan
> apa yang ditunjukkan repositori per 2026-10-11. Tidak ada yang di sini mengklaim akreditasi.

## Tampilan informasi dan data

CRM ini menyimpan **data hubungan**: kontak dan akun sebagai pembungkus untuk
pengenal yang disimpan di sistem lain, aktivitas, transaksi, tiket, riwayat persetujuan
dan log audit. Data demografis sengaja tetap berada di registri identitas
([domain-model](../../../../spec/domain-model.md)). Jika organisasi Anda mencatat pasien atau
warga sebagai kontak, klasifikasi data itu **Anda** yang menentukan;
rencananya ditulis dengan pembacaan yang lebih ketat ([plan](../../../../spec/plan.md)).

## Ke mana data pergi

| Penyimpanan | Isinya | Perlindungan di repositori |
|---|---|---|
| PostgreSQL | Semua catatan, rantai audit, pemberian break-glass | Enkripsi bidang pada dua kolom; pemicu audit hanya-tambah; enkripsi saat disimpan adalah tanggung jawab penerap |
| Redis | Cache respons insight (30 dtk), penghitung pembatasan laju, kunci pemberian break-glass | Dikunci per pemanggil; **memerlukan TLS, kata sandi, dan tidak terbuka ke publik** |
| Log/metrik | Metadata permintaan, metrik Prometheus | Retensi adalah tanggung jawab penerap |
| Penyedia e-mail (mendatang) | Isi pesan dan penerima | Belum dibangun (T39) |

## Integrasi dan identitas

- Masuk berasal dari layanan identitas yang menerbitkan token PASETO, atau Keycloak.
  Karena itu penerimaan/mutasi/keluarnya pegawai, MFA, dan kebijakan sesi adalah urusan **penyedia identitas
  Anda**; CRM menegakkan apa yang dikatakan token dan menolak yang kedaluwarsa.
- API didokumentasikan dalam OpenAPI ([gds-api-review](../../../../spec/compliance/gds-api-review.md)),
  diberi versi lewat header, dipaginasi dengan header `Link`, dan mengembalikan kesalahan sebagai
  dokumen masalah RFC 9457. Aturan perubahan dan penghentian ada di
  [api-lifecycle](../../../../spec/compliance/api-lifecycle.md).
- SBOM (CycloneDX) dibuat untuk layanan dan UI di CI; alur kerja rilis
  menandatangani SBOM dan image.

## Kewajiban tata kelola informasi yang tetap menjadi milik Anda

Bukti pencadangan dan pemulihan, jadwal retensi per tabel, penambatan kepala rantai audit
di luar basis data, penyimpanan kunci bidang di KMS, perjanjian pemrosesan data untuk setiap pemasok,
dan penilaian dampak perlindungan data
([dpia-template](../../../../spec/compliance/dpia-template.md)). Pemetaan kontrol
([control-mapping](../../../../spec/compliance/control-mapping.md)) menandai setiap kontrol sebagai
Implemented, Deployer, atau Gap.
