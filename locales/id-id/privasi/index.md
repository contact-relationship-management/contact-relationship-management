# Pemberitahuan privasi: apa yang disimpan di browser Anda

Pemberitahuan ini mencakup dua hal: situs web ini (`contact-relationship-management.github.io`), dan antarmuka
pengguna Contact Relationship Management sebagaimana repositori ini mengirimkannya. Pemberitahuan ini mencantumkan
semua yang disimpan oleh keduanya di browser Anda, dan alasannya.

Organisasi yang menjalankan CRM memproses data pribadi tentang kontaknya. Pemrosesan itu dicakup oleh pemberitahuan
privasi organisasi tersebut, bukan yang ini ([spec/regulatory.md](../../../spec/regulatory.md)).

## Apa yang disimpan, dan mengapa

Setiap item menyimpan pilihan yang Anda buat: tema, bahasa dan lokal, ukuran teks, zona waktu, atau tata letak
sebuah tabel. Tidak satu pun menyimpan nama, alamat email, alamat IP, atau pengenal apa pun milik Anda, dan tidak ada
yang dipakai untuk melacak Anda, mengukur Anda, atau menampilkan iklan kepada Anda. Masing-masing ada agar halaman
berikutnya yang Anda buka memakai pilihan yang sama, dan Anda tidak perlu membuatnya lagi.

Tidak ada yang disimpan sampai Anda membuat pilihan. Pengunjung yang tidak mengubah apa pun tidak memiliki apa pun
yang tersimpan.

### Di situs web ini

| Nama | Jenis | Isinya | Disimpan selama |
| --- | --- | --- | --- |
| `lily-theme` | Cookie dan `localStorage` | `system` ("Ikuti perangkat ini") atau nama sebuah tema, seperti `dark` | Cookie: 400 hari. `localStorage`: sampai Anda menghapusnya |
| `lily-text-size` | `localStorage` | Ukuran teks yang Anda pilih | Sampai Anda menghapusnya |
| `lily-time-zone` | `localStorage` | Zona waktu yang Anda pilih | Sampai Anda menghapusnya |

Tema disimpan dalam cookie dan dalam `localStorage`, sehingga tetap ada jika browser Anda menghapus salah satunya.
Halaman situs ini adalah berkas statis, dan tidak ada program di situs yang membaca cookie tersebut. Karena berupa
cookie, browser Anda mengirimkannya ke GitHub Pages, yang menjadi host situs ini, pada setiap permintaan ke situs.

Bahasa dan lokal situs adalah bagian dari alamat halaman, seperti `/de-de/`, sehingga tidak ada yang disimpan untuk
keduanya.

### Di antarmuka pengguna CRM

| Nama | Jenis | Isinya | Disimpan selama |
| --- | --- | --- | --- |
| `crm-theme` | Cookie | `system` ("Ikuti perangkat ini") atau nama sebuah tema | 400 hari |
| `mxi.crm.locale` | `localStorage` | Bahasa dan lokal yang Anda pilih, seperti `de-de` | Sampai Anda menghapusnya |
| `lily-text-size` | `localStorage` | Ukuran teks yang Anda pilih | Sampai Anda menghapusnya |
| `crm-grid-<table>` | `localStorage` | Lebar kolom, kolom tersembunyi, dan urutan penyortiran satu tabel | Sampai Anda menghapusnya |
| `crm-grid-order-<table>` | `localStorage` | Urutan kolom satu tabel | Sampai Anda menghapusnya |
| `__Host-mxi_session` | Cookie | Pengenal acak untuk sesi masuk Anda | Sampai Anda keluar atau sesi berakhir |

Server CRM membaca cookie `crm-theme`, agar dapat mengirim setiap halaman dalam tema Anda dan halaman tidak tampil
lebih dulu dalam tema lain. CRM tidak pernah membacanya untuk tujuan lain.

Cookie sesi bukan sebuah pilihan. Cookie ini diatur saat Anda masuk, skrip browser tidak dapat membacanya
(`HttpOnly`), dan tanpanya Anda tidak dapat tetap masuk.

## Siapa yang menerimanya

Item-item ini tetap berada di browser Anda. Tidak ada yang dikirim ke pihak ketiga, kecuali cookie tema situs, yang
dikirim browser Anda ke host situs seperti dijelaskan di atas. Cookie bersifat pihak pertama (`SameSite=Lax`), dan
`Secure` pada halaman `https`.

## Cara menghapusnya

Untuk mengubah pilihan, buat lagi pilihan itu. Untuk menghapus semuanya, hapus data situs ini di pengaturan browser
Anda: halaman berikutnya kemudian memakai bawaan lagi, yaitu mengikuti pengaturan terang atau gelap perangkat Anda,
ukuran teks standar, dan bahasa bawaan halaman.
