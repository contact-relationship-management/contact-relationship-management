# Rheoli Perthnasoedd Cyswllt

Meddalwedd CRM ffynhonnell agored ar gyfer gwerthu, marchnata a chymorth i gwsmeriaid.

Mae Rheoli Perthnasoedd Cyswllt yn olrhain perthnasoedd â chwsmeriaid a darpar gwsmeriaid ar draws gwerthu, marchnata a chymorth. Mae'n cadw un cofnod ar gyfer pob cyswllt ac mae angen caniatâd wedi'i gofnodi cyn anfon neges. Nid yw'r feddalwedd yn barod ar gyfer cynhyrchu: mae'n rhedeg ar ddata synthetig.

## Manteision

- **Un cofnod cyswllt.** Mae gwerthu, marchnata, cymorth a dadansoddeg yn defnyddio'r un cofnod.
- **Caniatâd wedi'i gofnodi.** Nid anfonir dim at gyswllt heb ganiatâd wedi'i gofnodi. Mae pob e-bost yn cynnwys dolen dad-danysgrifio wedi'i llofnodi.
- **Llai o ddata personol.** Mae'r CRM yn storio cyfeiriad at berson mewn gwasanaeth hunaniaeth, nid copi o'r enw na'r cyfeiriad post.
- **Canlyniadau esboniadwy.** Cyfrifir sgoriau arweinwyr o reolau darllenadwy. Cyfrifir rhagolygon o gamau piblinell wedi'u pwysoli.
- **Dylunio rhyngwladol.** Nid yw enwau, cyfeiriadau, calendrau a chyfeiriadau ysgrifennu ynghlwm wrth fformat un wlad. Mae'r rhyngwyneb ar gael mewn wyth iaith.
- **Manyleb gyhoeddus.** Mae'r fanyleb, y cod a'r profion yn gyhoeddus. Mae'r ddogfennaeth yn nodi beth sydd wedi'i adeiladu a beth sydd heb ei adeiladu.

## Nodweddion

- **Gwerthu.** Sgorio arweinwyr yn ôl rheolau, piblinellau bargeinion ar fwrdd, a rhagolygon wedi'u pwysoli fesul cam.
- **Marchnata.** Ymgyrchoedd wedi'u segmentu a dilyniannau meithrin. Mae anfon wedi'i efelychu oni bai bod modd post wedi'i ffurfweddu.
- **Cymorth.** Tocynnau gyda blaenoriaethau a therfynau amser lefel gwasanaeth, ac erthyglau sylfaen wybodaeth.
- **Dadansoddeg.** Dangosfyrddau ar gyfer cyfradd ennill, piblinell fesul cam, enillion ymgyrch, iechyd lefel gwasanaeth a gwerth oes cwsmer.
- **Ansawdd data.** Canfod dyblygiadau yn ôl tebygolrwydd, gyda chyfuno a dadwneud.
- **Llywodraethu.** Log archwilio, mynediad yn ôl rôl, a chymorth ar gyfer ceisiadau mynediad gwrthrych a dileu.

## Dechrau arni

1. **Cael y cod.** Clonio'r ystorfa. Mae angen Node.js a pnpm.
2. **Dechrau'r rhaglen.** Yn y cyfeiriadur Svelte, rhedwch `pnpm install`, yna `pnpm dev`.
3. **Mewngofnodi.** Defnyddiwch `example@example.com`. Nid oes angen unrhyw wasanaeth arall.
4. **Adolygu'r daith.** Mae'r daith yn ymdrin â chysylltiadau, arweinwyr, bargeinion, ymgyrchoedd, tocynnau a'r olwg weithredol, gan ddefnyddio data ffuglennol.

Mae'r [tiwtorial](../../docs/tutorial-first-tour.md) (Saesneg) yn disgrifio pob cam. Mae [gosod a defnyddio](../../INSTALL.md) (Saesneg) yn ymdrin â'r gwasanaeth llawn.

## Cymhariaeth

| Prosiect | Categori | Gwahaniaeth |
| --- | --- | --- |
| [Twenty](../../docs/compare/twenty.md) | CRM ffynhonnell agored | Cynnyrch llawn. Arddangosiad llai wedi'i yrru gan fanyleb yw'r prosiect hwn. |
| [EspoCRM](../../docs/compare/espocrm.md) | CRM ffynhonnell agored | Nodweddion ehangach. Mae'r prosiect hwn yn pwysleisio rheolau esboniadwy. |
| [SuiteCRM](../../docs/compare/suitecrm.md) | CRM ffynhonnell agored | Aeddfed a mawr. Cyfeirnod cryno yw'r prosiect hwn. |
| [Salesforce](../../docs/compare/salesforce.md), [HubSpot](../../docs/compare/hubspot.md) | CRM SaaS masnachol | Mae'r prosiect hwn yn modelu'r un arfer. Ni rennir cod na data. |

Arddangosiad yw'r prosiect hwn ac nid yw'n cystadlu â'r cynhyrchion hyn. Gweler [pob cymhariaeth](../../COMPARISONS.md) (Saesneg).

## Briffiau rôl

Mae'r [briffiau rôl](for/) yn darparu un dudalen ar gyfer pob uwch rôl: beth mae'r feddalwedd yn ei wneud, beth nad yw'n ei wneud, a pha benderfyniadau sy'n perthyn i'r rôl honno.

## Statws

Nid yw'r feddalwedd hon yn barod ar gyfer cynhyrchu ([y llwybr i gynhyrchu](path-to-production/index.md)). Defnyddiwch ddata synthetig yn unig. Nid anfonir e-bost go iawn oni bai bod modd post go iawn wedi'i ffurfweddu. Nid oes dim ar y wefan hon yn gyngor cyfreithiol.
