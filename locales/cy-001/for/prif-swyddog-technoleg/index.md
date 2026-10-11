# I'r Prif Swyddog Technoleg

> Briff peirianneg, nid cyngor cyfreithiol, ardystiad nac archwiliad. Mae'n nodi'r hyn y mae'r ystorfa yn ei ddangos ar 2026-10-11. Nid yw unrhyw beth yma yn honni achrediad.

## Pensaernïaeth

- **Gwasanaeth:** Rust ar Loco (Axum, SeaORM 2.0), PostgreSQL 18, Redis ar gyfer storfa dros dro (cache), cownteri a rennir a lledaenu caniatâd torri'r gwydr. `forbid(unsafe_code)`, clippy pedantic. Ffynhonnell y gwirionedd ar gyfer ymddygiad yw'r fanyleb ([index](../../../../spec/index.md), [architecture](../../../../spec/architecture.md)).
- **Rhyngwyneb:** SvelteKit 2 / Svelte 5, dirprwy backend-for-frontend fel nad yw unrhyw docyn byth yn cyrraedd JavaScript y porwr, CSP llym gyda hashes.
- **Dilysu:** PASETO v4.public wedi'i wirio all-lein, Keycloak dewisol; rheolaeth mynediad yn seiliedig ar briodoleddau (gweithredoedd darllen/ysgrifennu/dileu/dinistriol, rhwymedigaeth `mask`, gwiriadau ar lefel cofnod) o injan a rennir.
- **Archwilio:** cadwyn hash SHA-256 a ysgrifennir yn yr un trafodyn â'r newid, sbardun atodi-yn-unig, y gellir ei wirio yn `GET /api/audits/verify`.
- **Diogelu data:** amgryptio meysydd AES-256-GCM gyda chylchdroi allweddi ar ddau golofn; allweddi lleol, dim KMS.

## Tystiolaeth ansawdd

Profion uned (116), profion cronfa ddata (22, gan gynnwys gorfodi, cuddio, archwilio, tudaleniad, storio dros dro), profion uned parth blaen (35) a Playwright (39) gan gynnwys sganiau hygyrchedd axe. Mae CI yn rhedeg fmt, clippy, profion, adeiladu/lint y parth blaen a cham cadwyn gyflenwi (`cargo audit`, `cargo deny`, `pnpm audit`, SBOMau CycloneDX) ar GitHub Actions a Woodpecker. Mae un gorchymyn yn ei atgynhyrchu: `scripts/ci-check.sh`.

## Dyled dechnegol a chyfyngiadau gonest

- Mae tudaleniad yn defnyddio cyrchwr gwrthbwyso (gall rhesi gael eu hepgor neu eu hailadrodd os bydd data'n newid yn ystod y daith); dim cyfanswm yn ôl y bwriad.
- Mae allweddi idempotency yn ddewisol (T52): rhaid i gleient anfon `Idempotency-Key` ar gais creu i gael ei ddiogelu rhag i ailgais ddyblygu cofnod.
- Mae OpenAPI wedi'i ysgrifennu â llaw heb sgemâu cais/ymateb; nid yw'r adeiladu yn methu pan nad yw llwybr wedi'i ddogfennu ([gds-api-review](../../../../spec/compliance/gds-api-review.md)).
- Gwallau yw `{error, description}` Loco, nid RFC 9457.
- Mae amgryptio meysydd yn cwmpasu dwy golofn; dim integreiddio KMS.
- Mae angen Redis ar gyflwr cyfyngu cyfradd a storfa dros dro er mwyn cywirdeb aml-replica.
- Mae golygfeydd mewnwelediadau wedi'u cuddio yn ôl rôl ond yn cael eu storio dros dro fesul tocyn galwr am 30 eiliad.
- Mae ailddilysu cam-i-fyny cyn gweithredoedd dinistriol angen gwaith gyda'r darparwr hunaniaeth.

## Dilyniant technegol a argymhellir

1. Sgemâu OpenAPI ynghyd â phrawf cwmpas llwybrau.
2. Allweddi meysydd a gefnogir gan KMS a chwmpas meysydd ehangach (gweddill T41).
3. Llofnodi adeiladau rhyddhau (T49) unwaith y penderfynir ar ddalfa allweddi.
4. Comisiynu'r prawf annibynnol (T42) yn erbyn adeiladwaith llwyfannu.
