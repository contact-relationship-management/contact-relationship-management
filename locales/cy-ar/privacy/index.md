# Hysbysiad preifatrwydd: beth a gedwir yn eich porwr

Mae'r hysbysiad hwn yn ymdrin â dau beth: y wefan hon (`contact-relationship-management.github.io`), a rhyngwyneb
defnyddiwr Rheoli Perthnasoedd Cysylltiadau fel y mae'r ystorfa hon yn ei ddarparu. Mae'n rhestru popeth y mae'r naill
neu'r llall yn ei storio yn eich porwr, a pham.

Mae sefydliad sy'n rhedeg y CRM yn prosesu data personol am ei gysylltiadau. Hysbysiad preifatrwydd y sefydliad hwnnw ei
hun sy'n ymdrin â'r prosesu hwnnw, nid yr hysbysiad hwn ([spec/regulatory.md](../../../spec/regulatory.md)).

## Beth a storir, a pham

Mae pob eitem yn dal dewis a wnaethoch chi: thema, iaith a locale, maint testun, cylchfa amser, neu gynllun tabl. Nid
oes yr un ohonynt yn dal eich enw, eich cyfeiriad e-bost, eich cyfeiriad IP nac unrhyw ddynodwr, ac ni ddefnyddir yr un
ohonynt i'ch olrhain, i'ch mesur nac i ddangos hysbysebion i chi. Mae pob un yn bodoli fel bod y dudalen nesaf a agorwch
yn defnyddio'r un dewis, ac nad oes rhaid i chi ei wneud eto.

Ni storir dim nes i chi wneud dewis. Os nad yw ymwelydd yn newid dim, ni storir dim.

### Ar y wefan hon

| Enw | Math | Beth mae'n ei ddal | Am ba hyd y'i cedwir |
| --- | --- | --- | --- |
| `lily-theme` | Cwci a `localStorage` | `system` ("Dilyn y ddyfais hon") neu enw thema, fel `dark` | Cwci: 400 diwrnod. `localStorage`: nes i chi ei glirio |
| `lily-text-size` | `localStorage` | Y maint testun a ddewisoch | Nes i chi ei glirio |
| `lily-time-zone` | `localStorage` | Y gylchfa amser a ddewisoch | Nes i chi ei glirio |

Cedwir y thema mewn cwci ac yn `localStorage`, fel ei bod yno o hyd os yw eich porwr yn clirio un ohonynt. Ffeiliau
statig yw tudalennau'r wefan, ac nid oes unrhyw raglen ar y wefan yn darllen y cwci. Gan mai cwci ydyw, mae eich porwr yn
ei anfon at GitHub Pages, sy'n cynnal y wefan, gyda phob cais i'r wefan.

Mae iaith a locale y wefan yn rhan o gyfeiriad y dudalen, fel `/cy-ar/`, felly ni storir dim ar eu cyfer.

### Yn rhyngwyneb defnyddiwr y CRM

| Enw | Math | Beth mae'n ei ddal | Am ba hyd y'i cedwir |
| --- | --- | --- | --- |
| `crm-theme` | Cwci | `system` ("Dilyn y ddyfais hon") neu enw thema | 400 diwrnod |
| `mxi.crm.locale` | `localStorage` | Yr iaith a'r locale a ddewisoch, fel `cy-001` | Nes i chi ei glirio |
| `lily-text-size` | `localStorage` | Y maint testun a ddewisoch | Nes i chi ei glirio |
| `crm-grid-<table>` | `localStorage` | Lled colofnau, colofnau cudd a threfn didoli un tabl | Nes i chi ei glirio |
| `crm-grid-order-<table>` | `localStorage` | Trefn colofnau un tabl | Nes i chi ei glirio |
| `__Host-mxi_session` | Cwci | Dynodwr ar hap ar gyfer eich sesiwn wedi mewngofnodi | Nes i chi allgofnodi neu i'r sesiwn ddod i ben |

Mae gweinydd y CRM yn darllen y cwci `crm-theme`, fel y gall anfon pob tudalen yn eich thema ac nad yw'r dudalen yn
ymddangos mewn thema arall yn gyntaf. Nid yw'r CRM byth yn ei ddarllen at unrhyw ddiben arall.

Nid dewis yw cwci'r sesiwn. Fe'i gosodir pan fyddwch yn mewngofnodi, ni all sgriptiau'r porwr ei ddarllen (`HttpOnly`),
a hebddo ni allwch aros wedi mewngofnodi.

## Pwy sy'n ei dderbyn

Mae'r eitemau hyn yn aros yn eich porwr. Nid oes dim ynddynt yn cael ei anfon at drydydd parti, ar wahân i gwci thema'r
wefan, y mae eich porwr yn ei anfon at westeiwr y wefan fel y disgrifir uchod. Cwcis parti cyntaf ydynt (`SameSite=Lax`),
a `Secure` ar dudalen `https`.

## Sut i'w ddileu

I newid dewis, gwnewch ef eto. I ddileu popeth, cliriwch ddata'r wefan hon yng ngosodiadau eich porwr: mae'r dudalen
nesaf wedyn yn defnyddio'r rhagosodiadau eto, sef dilyn gosodiad golau neu dywyll eich dyfais, y maint testun safonol,
ac iaith ddiofyn y dudalen.
