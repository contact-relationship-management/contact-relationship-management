# I'r Prif Swyddog Gweithrediadau

> Briff peirianneg, nid cyngor cyfreithiol, ardystiad nac archwiliad. Mae'n nodi'r hyn y mae'r ystorfa yn ei ddangos ar 2026-10-11. Nid yw unrhyw beth yma yn honni achrediad.

## Beth sydd ei angen i'w rhedeg

Un broses Rust fesul replica yw'r gwasanaeth, ynghyd â PostgreSQL 18 a Redis; proses Node yw'r parth blaen. Mae'r ddau yn cael eu cyflenwi fel delweddau cynhwysydd caledu, nad ydynt yn rhedeg fel root, gyda delweddau sylfaen wedi'u pinio yn ôl crynodeb (digest). Y llawlyfr gweithrediadau yw [operations-runbook.md](../../../../spec/compliance/operations-runbook.md).

## Ffeithiau gweithredol i gynllunio o'u cwmpas

- **Yn methu'n ddiogel wrth gychwyn.** Mewn cynhyrchiad mae'r gwasanaeth yn gwrthod cychwyn os yw dilysu wedi'i ddiffodd, os nad yw'r gadwyn archwilio yn ddefnyddiadwy, neu os yw allweddi gofynnol a `CRM_REDIS_URL` ar goll. Mae lleoliad sydd wedi'i gamgyflunio yn stopio yn hytrach na rhedeg ar agor.
- **Iechyd a metrigau.** Ni chyfyngir cyfradd `/_health`, `/_ping`, `/_readiness` byth, felly ni all cloi allan ailgychwyn pod iach. Mae metrigau yn `/metrics.prom`.
- **Terfynau camddefnydd** ar waith mewn cynhyrchiad: 600 cais y funud fesul cleient a chloi allan ar ôl 10 mewngofnodiad aflwyddiannus mewn 5 munud. Cânt eu rhannu ar draws replicas drwy Redis; os bydd Redis yn methu, maent yn dychwelyd i gyfrif fesul replica.
- **Mynediad brys** (torri'r gwydr): gall unrhyw ddefnyddiwr sydd wedi mewngofnodi gymryd dyrchafiad darllen â rheswm am hyd at 60 munud. Rhaid i rywun arall adolygu pob caniatâd. **Proses y mae'n rhaid i chi ei rhedeg:** gosodwch rybudd ar resi archwilio `break_glass_granted` ac adolygwch nifer y rhai heb eu hadolygu (tudalen weinyddol `/break-glass/review`).
- **Twf archwilio.** Archwilir darlleniadau, felly mae'r tabl archwilio yn tyfu gyda'r traffig. Cynlluniwch y cyfnod cadw ar ei gyfer. Bydd cleient sy'n holi'n gyson yn ei dyfu'n gyflym.

## Gwaith gweithredol agored

| Eitem | Cyflwr |
|---|---|
| Ymarferion wrth gefn/adfer, ymarfer cylchdroi allweddi | Wedi'u hysgrifennu yn y llawlyfr; **chi** sy'n gorfod eu rhedeg a chadw tystiolaeth |
| Angori pen y gadwyn archwilio y tu allan i'r gronfa ddata | Eich dyletswydd chi; mae'r API yn adrodd hash y pen |
| Adnewyddu crynodebau delweddau sydd wedi'u pinio | Angen amserlen (Renovate neu Dependabot) neu byddant yn dyddio |
| Darparwr e-bost go iawn (T39) | Agored; yn newid darlun digwyddiadau a chyflenwyr |
| Mae Redis yn dal data mewnwelediadau | Diogelwch ef fel y gronfa ddata: TLS, cyfrinair, dim amlygiad cyhoeddus |

## Cyfyngiadau hysbys sy'n werth eu dweud wrth y tîm ar alwad

Mae allgofnodi segur ar ochr y cleient; gwrthodir tocyn a ddirymwyd ar unwaith, ond mae ail-lwytho polisi yn ymddangos mewn mewnwelediadau sydd yn y storfa dros dro (cache) ar ôl hyd at 30 eiliad; ni ellir dirymu torri'r gwydr, dim ond ei adael i ddod i ben.
