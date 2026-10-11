# I'r Prif Swyddog Cyfreithiol

> Briff peirianneg, nid cyngor cyfreithiol, ardystiad nac archwiliad. Mae'n nodi'r hyn y mae'r ystorfa yn ei ddangos ar 2026-10-11. Nid yw unrhyw beth yma yn honni achrediad.

## Y sefyllfa gyfreithiol yn gryno

Meddalwedd arddangos â data synthetig yw'r ystorfa. Nid yw'n honni cydymffurfio ag unrhyw drefn. Mae'n gweithredu nodweddion technegol sy'n cefnogi sawl un, ac yn rhestru'r hyn y mae'n rhaid i'r sawl sy'n ei lleoli ei ychwanegu ([regulatory](../../../../spec/regulatory.md)).

## Trefniadau perthnasol

| Trefn | Beth mae'r feddalwedd yn ei gefnogi | Beth mae'n rhaid i chi ei ddarparu |
|---|---|---|
| GDPR y DU/UE | Hanes cydsyniad, allforio mynediad at ddata personol, dileu (a wrthodir tra bo rhwymedigaethau agored), ysgubo cadw, lleihau data, archwilio darlleniadau | Sail gyfreithlon ar gyfer pob diben, DPIA ([templed](../../../../spec/compliance/dpia-template.md)), cofnodion prosesu, amserlen gadw, rolau rheolydd/prosesydd |
| PECR / e-breifatrwydd | Anfon yn amodol ar gydsyniad, dad-danysgrifio ar unwaith | Penderfyniadau ar adnabod yr anfonwr a dewis-i-mewn meddal ar gyfer anfonwr go iawn |
| HIPAA (os yw'n ddata iechyd yr UD) | Rheolaethau archwilio, rheoli mynediad, uniondeb, nodweddion diogelwch trosglwyddo | Cytundeb cydymaith busnes, dadansoddi risg, polisïau, hyfforddi'r gweithlu. Nid yw'r feddalwedd yn eich gwneud yn cydymffurfio |
| DTAC y GIG / DCB0129/0160 | Drafftiau tystiolaeth ([dtac-supporting](../../../../spec/compliance/dtac-supporting.md)) | Swyddog diogelwch clinigol a enwir, log peryglon, ardystiadau. Mae'r asesiad yn dweud ei bod yn debygol nad yw DTAC yn berthnasol |
| Hygyrchedd (WCAG 2.2 AA, y sector cyhoeddus) | Gwiriadau awtomataidd a datganiad gonest | Archwiliad â llaw (T48) cyn unrhyw hawliad cydymffurfio |
| Diogelwch y llywodraeth (NIST 800-53, ISO 27001, Cyber Essentials) | Mapio rheolaethau ([control-mapping](../../../../spec/compliance/control-mapping.md)) | Ardystiad a sicrwydd annibynnol |

## Contractau y bydd eu hangen arnoch nad ydynt yn bodoli eto

1. Cytundeb prosesu data a thelerau trosglwyddo rhyngwladol gyda'r darparwr e-bost (T39).
2. Datganiad gwaith a thelerau cyfrinachedd gyda'r cwmni profi treiddio (T42).
3. Telerau cynnal a thelerau is-brosesydd ar gyfer PostgreSQL, Redis a logiau.
4. Trwydded: gweler [LICENSE.md](../../../../LICENSE.md); mae'r ystorfa hefyd yn cynnwys datganiadau diogelwch, llywodraethu a defnyddio AI.

## Pwyntiau cyfreithiol penodol i'w gwylio

- Mae **dileu** yn anonymeiddio'r cyswllt a chaiff ei wrthod tra bo bargen agored, tocyn agored neu gofrestriad meithrin gweithredol; mae'r gadwyn archwilio yn cadw cofnodion o weithredoedd, felly penderfynwch yn eich DPIA sut mae hynny'n rhyngweithio â cheisiadau dileu.
- Mae darlleniadau **torri'r gwydr** o ddata wedi'i guddio yn gyfreithlon yn unig ar eich sail ddogfenedig; tystiolaeth yw'r maes rheswm, nid awdurdod.
- **Sgorio** arweinwyr: os bydd byth yn dylanwadu ar benderfyniadau arwyddocaol am bobl, mae rheolau gwneud penderfyniadau awtomataidd yn berthnasol ([regulatory](../../../../spec/regulatory.md)).
- **Hawliadau cyhoeddus:** peidiwch â datgan cydymffurfiaeth, ardystiad na chymeradwyaeth.
