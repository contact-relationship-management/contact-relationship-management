# Profwr Diogelwch Annibynnol

> Briff peirianneg, nid cyngor cyfreithiol, ardystiad nac archwiliad. Mae'n nodi'r hyn y mae'r ystorfa yn ei ddangos ar 2026-10-11. Nid yw unrhyw beth yma yn honni achrediad.

## Pam mae hyn yn bodoli

Adeiladwyd a phrofwyd pob rheolaeth ddiogelwch yn y prosiect hwn gan ei awduron (CRM-T42). Mae hynny'n dangos bod y rheolaethau'n gweithio fel y mae'r awduron yn eu deall. Nid yw'n dangos nad oes dim ar goll. Fel arfer mae ysbytai a phrynwyr y llywodraeth yn mynnu prawf gan rywun annibynnol cyn dal data go iawn, a than hynny'r hawliad gonest yw “wedi'i adeiladu i safon uchel, heb ei brofi gan bobl o'r tu allan” ([briff y Prif Swyddog Gweithredol](../prif-swyddog-gweithredol/index.md), [briff y Prif Swyddog Diogelwch](../prif-swyddog-diogelwch/index.md)).

Mae'r dudalen hon yn nodi beth i'w roi i gwmni profi, a beth i'w ddisgwyl yn ôl.

## Beth mae pasio yn ei olygu, a beth nad yw'n ei olygu

Mae adroddiad glân yn dweud bod tîm a enwir, mewn amser a enwir, â chwmpas a enwir, heb ganfod problemau difrifol. **Nid** ardystiad ydyw, nid achrediad, ac nid gwarant. Peidiwch â disgrifio'r system fel un “ddiogel”, “wedi'i hardystio” neu “wedi'i phrofi'n dreiddiol yn ôl safon” y tu hwnt i'r hyn y mae'r adroddiad yn ei nodi ([briff Llywodraethu, Risg, Cydymffurfiaeth](../llywodraethu-risg-cydymffurfiaeth/index.md)).

## Dewis y profwr

- Annibynnol ar yr awduron, ac ar unrhyw un a gyflunodd y lleoliad.
- Staff a enwir, profiadol, ag achrediad perthnasol ar gyfer eich marchnad lle bo'n berthnasol (er enghraifft CREST, neu gynllun CHECK yr NCSC ar gyfer gwaith llywodraeth y DU). Gofynnwch pa achrediad sy'n cwmpasu'r unigolion sy'n gwneud y gwaith, nid y cwmni yn unig.
- Cytuno'n ysgrifenedig ar y cwmpas, rheolau ymgysylltu, cyfrinachedd, trin data a chyfrifoldeb cyn i unrhyw brofi ddechrau. Y tîm cyfreithiol sy'n cytuno ar y contract ([briff y Prif Swyddog Cyfreithiol](../prif-swyddog-cyfreithiol/index.md)).

## Beth i'w brofi

| Maes | Beth y dylai'r profwyr geisio ei wneud | Ble disgrifir y dyluniad |
|---|---|---|
| Dilysu | Tocynnau ffug, wedi dod i ben, wedi'u hailchwarae ac ar gyfer y gynulleidfa anghywir; syrthio i gyflwr anawdurdodedig; y gwiriad cychwyn sy'n gwrthod rhedeg gyda dilysu wedi'i ddiffodd | [auth](../../../../spec/auth.md) |
| Awdurdodi | Cyrraedd cofnodion neu weithredoedd na ddylai rôl eu cael: dyrchafu llorweddol a fertigol, gwiriadau ar lefel cofnod, gweithredoedd dinistriol, darlleniadau i weinyddwyr yn unig (archwilio, cadw, adolygu torri'r gwydr) | [auth](../../../../spec/auth.md) |
| Datgelu data | Gwerthoedd wedi'u cuddio'n gollwng drwy restrau, darlleniadau manylion cyfun, mewnwelediadau, cyfansymiau, negeseuon gwall, y storfa dros dro neu'r allwedd storio | [tasks](../../../../spec/tasks.md) (T32, T47) |
| Mynediad brys | Cymryd, ymestyn neu gamddefnyddio torri'r gwydr; adolygu eich caniatâd eich hun; adolygu ddwywaith | [auth](../../../../spec/auth.md) (T44) |
| Archwilio | Newid, dileu neu ad-drefnu hanes; osgoi'r gadwyn; cuddio darlleniadau | [audit](../../../../spec/audit.md) |
| Gwrthsefyll camddefnydd | Osgoi cyfyngu cyfradd a chloi allan, cloi defnyddwyr eraill allan, ffugio penawdau ar gyfer allwedd y cleient | T43 |
| Trin mewnbwn | Chwistrellu (SQL, JSON, penawdau), cyrff rhy fawr, cyrchwyr ac allweddi idempotency camffurfiedig, ailchwarae neu wrthdaro allweddi rhwng galwyr | T51, T52 |
| Parth blaen gwe | Sgriptio traws-safle, ffugio ceisiadau, trin sesiynau, y dirprwy backend-for-frontend, polisi diogelwch cynnwys, clicio-herwgipio | Rhyngwyneb `hooks.server.ts` |
| Seilwaith fel y'i cyflenwir | Delweddau cynhwysydd, porthladdoedd agored, cyrraedd Redis a PostgreSQL, trin cyfrinachau, rhagosodiadau cyflunio | [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Cadwyn gyflenwi | Tarddiad dibyniaethau a delweddau, y SBOMau, adeiladau heb eu llofnodi | [control-mapping](../../../../spec/compliance/control-mapping.md) |
| Rhesymeg busnes | Osgoi cydsyniad yn y llwybr anfon, dileu sy'n gadael data ar ôl, trin sgorio, creu dyblyg | [regulatory](../../../../spec/regulatory.md) |

**Y tu allan i'r cwmpas oni chytunwch fel arall:** y gwasanaeth hunaniaeth sy'n cyhoeddi tocynnau (mae y tu allan i'r ystorfa hon), y llwyfan cynnal, a gwrthod gwasanaeth ar raddfa fawr.

## Beth i'w roi i'r profwyr

1. **Adeiladwaith llwyfannu** sy'n cyfateb i'r hyn a fydd yn rhedeg mewn cynhyrchiad: cyflunio cynhyrchiad, dilysu ymlaen, TLS, Redis a PostgreSQL go iawn. Peidiwch â phrofi rhagosodiadau'r arddangosiad.
2. **Data synthetig yn unig.** Mae'r dasg hadu yn adeiladu llyfr busnes heb unrhyw bobl go iawn. Rhaid i ddata go iawn am gleifion neu ddinasyddion beidio â bod mewn amgylchedd profi byth.
3. **Un cyfrif fesul rôl**, fel y gellir profi awdurdodi o bob ochr. Y polisi cyfeirio yw `config/abac-policy.reference.json`.
4. Y dogfennau a gysylltir uchod, dogfen OpenAPI yn `/api-docs/openapi.json`, a mynediad darllen i'r ystorfa, fel y gall y prawf gyfuno profi blwch du ac adolygu cod.
5. Y **gwendidau hysbys**, fel y bydd profwyr yn treulio eu hamser ar yr hyn nad yw eisoes wedi'i ysgrifennu:
   - mae llofnodi rhyddhau wedi'i adeiladu ond heb redeg ar ryddhad go iawn (T49);
   - mae amgryptio meysydd yn cwmpasu dwy golofn gydag allweddi lleol, dim KMS;
   - nid yw ailddilysu cam-i-fyny wedi'i adeiladu;
   - ni ellir dirymu torri'r gwydr, dim ond ei adael i ddod i ben, ac nid oes rhybudd wedi'i gysylltu;
   - mae ail-lwytho polisi yn ymddangos mewn mewnwelediadau a storiwyd dros dro ar ôl hyd at 30 eiliad;
   - mae allgofnodi segur ar ochr y cleient;
   - mae cyflwr cyfyngu cyfradd yn dychwelyd i fesul replica os bydd Redis yn methu.
   Gofynnwch iddynt gadarnhau neu wrthbrofi pob un, nid ei hepgor.

## Rheolau ymgysylltu i'w cytuno'n ysgrifenedig

- Profwyr a enwir, dyddiadau a chyfeiriadau ffynhonnell; cyswllt 24 awr ar eich ochr chi.
- Beth y gellir ei geisio (er enghraifft, peidio â dinistrio'r storfa data brofi heb rybudd) a beth y mae'n rhaid iddo stopio wrth brawf o amlygiad.
- Pwy a hysbysir ar unwaith am ganfyddiad critigol, a sut, cyn yr adroddiad terfynol.
- Sut y caiff yr adroddiad, y dystiolaeth ac unrhyw nodiadau'r profwyr eu storio a'u dinistrio.

## Beth y dylech ei gael yn ôl

- Adroddiad â chwmpas, dull, dyddiadau, enwau'r profwyr ac achrediad.
- Pob canfyddiad gyda difrifoldeb, tystiolaeth, camau i'w atgynhyrchu, y gydran yr effeithir arni ac atgyweiriad a argymhellir.
- Datganiad clir o'r hyn **na** chafodd ei brofi.
- Ail-brawf o'r canfyddiadau a drwsiwyd, wedi'i gynnwys yn yr ymgysylltiad neu wedi'i brisio.

## Ar ôl yr adroddiad

1. Didolwch bob canfyddiad yn `spec/tasks.md` fel tasg â pherchennog a dyddiad.
2. Trwsiwch ganfyddiadau critigol ac uchel cyn unrhyw ddata go iawn, yna gofynnwch i'r profwyr gadarnhau'r atgyweiriadau.
3. Ychwanegwch brawf atchweliad ar gyfer pob atgyweiriad. Rheol y prosiect yw peidio â gwanhau prawf na gât i wneud i rywbeth basio.
4. Cofnodwch y canlyniad yn onest: dyddiad yr adroddiad, y cwmpas, y canfyddiadau a'u statws. Diweddarwch y cynllun parodrwydd ([plan](../../../../spec/plan.md)) a'r pecyn cydymffurfiaeth ([compliance](../../../../spec/compliance/index.md)).
5. Ail-brofwch ar ôl newid sylweddol, ac yn ôl amserlen a osodir gan eich perchennog risg. Ciplun yw un prawf, nid cyflwr.

## Trefn y gwaith

1. Caewch yr hyn sy'n rhad ac yn hysbys yn gyntaf, fel nad yw'r profwyr yn gwario'r gyllideb yn ei ailddarganfod: rhyddhad cyntaf wedi'i lofnodi (T49), rhybuddio ar dorri'r gwydr ac ar fethiannau gwirio'r archwilio, Redis y tu ôl i TLS a manylion mewngofnodi.
2. Paratowch yr adeiladwaith llwyfannu, y cyfrifon a'r data hadu.
3. Cytunwch ar y cwmpas, y contract a'r rheolau ymgysylltu.
4. Profwch; derbyniwch ganfyddiadau critigol ar unwaith.
5. Trwsiwch, ail-brofwch a chofnodwch.
6. Dim ond wedyn y diweddarwch hawliadau allanol, a dim ond i'r hyn y mae'r adroddiad yn ei gefnogi.

## Penderfyniadau i chi

1. Pwy sy'n comisiynu a phwy sy'n berchen ar y canfyddiadau (gweler [briff y Prif Swyddog Risg](../prif-swyddog-risg/index.md)).
2. Pa achrediad y mae eich marchnad yn ei ddisgwyl gan y profwyr.
3. Y gyllideb a'r dyddiad, wedi'u pennu gan gost oedi yn hytrach na chyfleustra.
4. A ddylid profi unwaith cyn lansio neu yn ôl amserlen reolaidd.
