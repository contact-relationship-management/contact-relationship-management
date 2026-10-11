# I'r Prif Swyddog Gwybodaeth

> Briff peirianneg, nid cyngor cyfreithiol, ardystiad nac archwiliad. Mae'n nodi'r hyn y mae'r ystorfa yn ei ddangos ar 2026-10-11. Nid yw unrhyw beth yma yn honni achrediad.

## Golwg ar wybodaeth a data

Mae'r CRM hwn yn dal **data perthnasoedd**: cysylltiadau a chyfrifon fel amlenni o amgylch dynodwyr a ddelir mewn systemau eraill, gweithgareddau, bargeinion, tocynnau, hanes cydsyniad a log archwilio. Mae data demograffig yn aros yn fwriadol yn y cofrestrfeydd hunaniaeth ([domain-model](../../../../spec/domain-model.md)). Os yw eich sefydliad yn cofnodi cleifion neu ddinasyddion fel cysylltiadau, **chi** sydd i benderfynu dosbarthiad y data hwnnw; mae'r cynllun wedi'i ysgrifennu yn ôl y dehongliad llymaf ([plan](../../../../spec/plan.md)).

## I ble mae data'n mynd

| Storfa | Yn dal | Diogelwch yn yr ystorfa |
|---|---|---|
| PostgreSQL | Pob cofnod, y gadwyn archwilio, caniatadau torri'r gwydr | Amgryptio meysydd ar ddau golofn; sbardun archwilio atodi-yn-unig; amgryptio wrth orffwys yw cyfrifoldeb y defnyddiwr sy'n ei leoli |
| Redis | Storfa dros dro o ymatebion mewnwelediadau (30 eiliad), cownteri cyfyngu cyfradd, allweddi caniatâd torri'r gwydr | Allweddi fesul galwr; **angen TLS, cyfrinair a dim amlygiad cyhoeddus** |
| Logiau/metrigau | Metadata ceisiadau, metrigau Prometheus | Cadw yw cyfrifoldeb y defnyddiwr sy'n ei leoli |
| Darparwr e-bost (yn y dyfodol) | Cynnwys negeseuon a derbynwyr | Heb ei adeiladu (T39) |

## Integreiddio a hunaniaeth

- Daw mewngofnodi o wasanaeth hunaniaeth sy'n cyhoeddi tocynnau PASETO, neu Keycloak. Felly mae dechreuwyr/symudwyr/gadawyr, MFA a pholisi sesiwn yn eiddo i **ddarparwr hunaniaeth chi**; mae'r CRM yn gorfodi'r hyn y mae'r tocyn yn ei ddweud ac yn gwrthod rhai sydd wedi dod i ben.
- Dogfennir yr API yn OpenAPI ([gds-api-review](../../../../spec/compliance/gds-api-review.md)), fersiynir ef gan bennawd, ac erbyn hyn mae wedi'i dudalennu gyda phenawdau `Link`.
- Cynhyrchir SBOMau (CycloneDX) ar gyfer y gwasanaeth a'r rhyngwyneb yn CI.

## Dyletswyddau llywodraethu gwybodaeth sy'n aros gyda chi

Tystiolaeth wrth gefn ac adfer, amserlenni cadw fesul tabl, angori pen y gadwyn archwilio y tu allan i'r gronfa ddata, dalfa KMS ar gyfer allweddi meysydd, cytundebau prosesu data ag unrhyw gyflenwr, a'r asesiad effaith diogelu data ([dpia-template](../../../../spec/compliance/dpia-template.md)). Mae'r mapio rheolaethau ([control-mapping](../../../../spec/compliance/control-mapping.md)) yn nodi pob rheolaeth fel “Wedi'i weithredu”, “Defnyddiwr sy'n lleoli” neu “Bwlch”.
