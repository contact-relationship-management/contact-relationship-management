# I'r Prif Swyddog Diogelwch

> Briff peirianneg, nid cyngor cyfreithiol, ardystiad nac archwiliad. Mae'n nodi'r hyn y mae'r ystorfa yn ei ddangos ar 2026-10-11. Nid yw unrhyw beth yma yn honni achrediad.

## Y sefyllfa ddiogelwch mewn un paragraff

Mae'r gwasanaeth yn **methu'n ddiogel**, yn rheoli mynediad yn ôl rôl ac yn ôl cofnod, yn cuddio gwerthoedd sensitif rhag rolau na ddylent eu gweld, ac yn cadw llwybr archwilio sy'n dangos ymyrraeth. Mae'r awduron wedi gweithredu a phrofi'r rheolaethau hynny. **Nid oes neb annibynnol wedi eu profi** (T42), felly triniwch bob datganiad isod fel “wedi'i adeiladu a'i hunan-brofi”, nid “wedi'i wirio”.

## Rheolaethau a ble i'w gwirio

| Maes | Beth sydd ar waith | Ble i edrych |
|---|---|---|
| Dilysu | PASETO v4.public wedi'i wirio all-lein, Keycloak dewisol; mae cynhyrchiad yn gwrthod cychwyn gyda dilysu wedi'i ddiffodd | [auth](../../../../spec/auth.md) |
| Awdurdodi | Yn seiliedig ar briodoleddau: darllen, ysgrifennu, dileu, dinistriol; rhwymedigaeth `mask`; gwiriadau ar lefel cofnod; archwilio, cadw ac adolygu torri'r gwydr i weinyddwyr yn unig | [auth](../../../../spec/auth.md), prawf matrics personâu |
| Datgelu data | Cuddio ar gofnodion, cyfansymiau a mewnwelediadau; canfuwyd a thrwsiwyd gollyngiad ym manylion y cyfrif (T47) | [tasks](../../../../spec/tasks.md) |
| Mynediad brys | Rheswm ysgrifenedig, uchafswm o 60 munud, darllen yn unig, adolygiad gorfodol gan ail berson, archwiliedig, a rennir drwy Redis | tasgau torri'r gwydr T44 |
| Archwilio | Cadwyn hash SHA-256 yn yr un trafodyn, sbardun atodi-yn-unig, pwynt terfyn gwirio; archwilir darlleniadau | [audit](../../../../spec/audit.md) |
| Gwrthsefyll camddefnydd | Cyfyngu cyfradd (600 y funud) a chloi allan (10 methiant mewn 5 munud), a rennir drwy Redis; probau wedi'u heithrio | T43 |
| Porwr a thrafnidiaeth | CSP llym gyda hashes, gwrthod fframiau, `nosniff`, polisi atgyfeirio, CORS caeedig, penawdau diogel; dim tocyn yn JavaScript y porwr (BFF) | Rhyngwyneb `hooks.server.ts` |
| Diogelu data | AES-256-GCM ar ddau golofn gyda chylchdroi; allweddi lleol, dim KMS | T41 |
| Cadwyn gyflenwi | `cargo audit`, `cargo deny`, `pnpm audit`, SBOMau, delweddau caledu nad ydynt yn rhedeg fel root wedi'u pinio yn ôl crynodeb | [control-mapping](../../../../spec/compliance/control-mapping.md) |
| Diogelwch cof | `forbid(unsafe_code)`, clippy pedantic | `src/lib.rs` |

## Gwendidau hysbys (wedi'u nodi, nid eu cuddio)

- **Dim prawf treiddio annibynnol na dadansoddiad bygythiadau** (T42). Crynodeb yw'r model bygythiadau yn [dtac-supporting](../../../../spec/compliance/dtac-supporting.md), nid un strwythuredig.
- **Adeiladau heb eu llofnodi**; mae crynodebau wedi'u pinio yn dyddio heb amserlen adnewyddu (T49).
- Mae **amgryptio meysydd** yn cwmpasu dwy golofn gydag allweddi lleol, dim KMS.
- Nid yw **ailddilysu cam-i-fyny** cyn gweithredoedd dinistriol wedi'i adeiladu.
- Mae **Redis yn dal data personol** (mewnwelediadau a storiwyd dros dro, allweddi caniatâd) a rhaid ei ddiogelu fel y gronfa ddata: TLS, cyfrinair, dim amlygiad cyhoeddus.
- Ni ellir **dirymu torri'r gwydr**, dim ond ei adael i ddod i ben, ac nid oes rhybudd wedi'i gysylltu.
- Mae **cyfyngu cyfradd a chloi allan** yn dychwelyd i gyfrif fesul replica os bydd Redis yn methu; mae'r bwced anawdurdodedig yn cael ei rannu oni bai bod `CRM_TRUST_FORWARDED_FOR=1` y tu ôl i ddirprwy y gellir ymddiried ynddo, felly gall ymosodwr gloi galwyr anawdurdodedig eraill allan.
- Mae **ail-lwytho polisi** yn ymddangos mewn mewnwelediadau a storiwyd dros dro ar ôl hyd at 30 eiliad.
- Mae **allgofnodi segur** ar ochr y cleient.
- Mae **allweddi idempotency yn ddewisol** (T52): nid yw cleient nad yw'n anfon `Idempotency-Key` wedi'i ddiogelu rhag i gais creu a ailgeisir ddyblygu cofnod.

## Beth mae'n rhaid i'r sawl sy'n lleoli ei wneud

1. Cadw dilysu ymlaen; darparu allweddi cynhyrchu a `CRM_REDIS_URL`.
2. Rhybuddio ar `break_glass_granted` ac adolygu caniatadau'n brydlon.
3. Angori pen y gadwyn archwilio y tu allan i'r gronfa ddata a'i wirio yn ôl amserlen (`GET /api/audits/verify`).
4. Diogelu PostgreSQL a Redis (rhwydwaith, TLS, manylion mewngofnodi, copïau wrth gefn, amgryptio wrth orffwys); cylchdroi allweddi meysydd gyda'r dasg a ddarperir.
5. Rhedeg diweddariadau dibyniaethau a delweddau; ail-redeg y cam cadwyn gyflenwi.
6. Dilyn y llawlyfr ar gyfer ymateb i ddigwyddiadau ([operations-runbook](../../../../spec/compliance/operations-runbook.md)).

## Trin gwendidau

Mae adroddiadau'n mynd drwy [SECURITY.md](../../../../SECURITY.md). Comisiynwch y prawf annibynnol (T42) yn erbyn adeiladwaith llwyfannu cyn unrhyw ddata go iawn, ac ail-brofwch ar ôl newid sylweddol.

## Camau diogelwch a awgrymir, yn eu trefn

1. Cwmpasu a chomisiynu'r prawf treiddio annibynnol (T42).
2. Penderfynu ar ddalfa allweddi, yna llofnodi adeiladau rhyddhau a gosod amserlen adnewyddu crynodebau (T49).
3. Cysylltu rhybuddio ar gyfer torri'r gwydr ac ar gyfer methiannau gwirio'r gadwyn archwilio.
4. Rhoi Redis y tu ôl i TLS a manylion mewngofnodi ym mhob amgylchedd.
5. Allweddi meysydd a gefnogir gan KMS a chwmpas meysydd ehangach (gweddill T41).

Gweler hefyd [briff y Prif Swyddog Technoleg](../prif-swyddog-technoleg/index.md) a [briff y Prif Swyddog Risg](../prif-swyddog-risg/index.md).
