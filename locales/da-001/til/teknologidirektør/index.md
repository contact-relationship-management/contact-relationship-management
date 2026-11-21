# Til teknologidirektøren

> Teknisk orientering, ikke juridisk rådgivning, certificering eller revision. Den angiver,
> hvad repositoriet viser pr. 2026-10-11. Intet her gør krav på akkreditering.

## Arkitektur

- **Tjeneste:** Rust på Axum og axa (sqlx 0.9), PostgreSQL 18, Redis til cache, delte tællere og udbredelse af
  break-glass. `forbid(unsafe_code)`, clippy pedantic. Kilden til sandhed for adfærd er specifikationen
  ([index](../../../../spec/index.md), [architecture](../../../../spec/architecture.md)).
- **Brugerflade:** SvelteKit 3 / Svelte 5, en backend-for-frontend-proxy, så intet token nogensinde når browserens
  JavaScript, streng CSP med hashes.
- **Autentificering:** PASETO v4.public verificeret offline, valgfri Keycloak; attributbaseret adgangskontrol
  (læse/skrive/slette/destruktive handlinger, en `mask`-forpligtelse, kontrol på postniveau) fra en fælles motor.
- **Revision:** SHA-256-hashkæde skrevet i samme transaktion som ændringen, trigger der kun tilføjer, kan verificeres på
  `GET /api/audits/verify`.
- **Databeskyttelse:** AES-256-GCM-feltkryptering med nøglerotation på to kolonner; lokale nøgler, ingen KMS.

## Dokumentation for kvalitet

Enhedstests (185), databasetests (56 inklusive håndhævelse, maskering, revision, paginering, caching, idempotens og
skemakontroller), enhedstests af brugerfladen (106) og Playwright (98) inklusive axe-scanninger af tilgængelighed. CI
kører fmt, clippy, tests, build og lint af brugerfladen og et forsyningskædetrin (`cargo audit`, `cargo deny`,
`pnpm audit`, CycloneDX-SBOM'er) på GitHub Actions og Woodpecker. Én kommando gengiver det: `scripts/ci-check.sh`.

## Teknisk gæld og ærlige grænser

- Pagineringen bruger en offset-markør (rækker kan springes over eller gentages, hvis data ændres undervejs); ingen
  samlet optælling, med vilje.
- Idempotensnøgler er valgfrie i API'en (T52): en klient skal sende `Idempotency-Key` ved en oprettelse for at være
  beskyttet mod, at et genforsøg duplikerer en post. Den medfølgende brugerflade sender en ved hver oprettelse.
- OpenAPI-skemaerne er skrevet i hånden. De dækker forespørgsler, lister, detaljelæsninger, dashboards, indsigtsvisninger,
  revision, privatliv og break-glass; tilstandsovergange og upserts, der returnerer den opdaterede post, er utypede. En
  test fejler, når en rute og dokumentet er uenige, og andre kontrollerer rigtige svar, også maskerede, mod skemaerne
  ([gds-api-review](../../../../spec/compliance/gds-api-review.md)).
- Fejl er RFC 9457-problemdokumenter (T54), hvor de tidligere felter `error` og `description` er bevaret; fejl fra lag
  uden for routeren (størrelsesgrænse, timeout) omdannes ikke. Politikken for versionering og udfasning er
  [api-lifecycle](../../../../spec/compliance/api-lifecycle.md); headerne `Deprecation` og `Sunset` er specificeret, men
  ikke bygget.
- Feltkryptering dækker to kolonner; ingen KMS-integration.
- Tilstand for hastighedsgrænse og cache kræver Redis for korrekthed med flere replikaer.
- Indsigtsvisninger maskeres efter rolle, men cachelagres pr. kalders token i 30 s.
- Fornyet autentificering før destruktive handlinger kræver arbejde hos identitetsudbyderen.

## Anbefalet teknisk rækkefølge

1. KMS-baserede feltnøgler og bredere feltdækning (resten af T41).
2. Mærk en første udgivelse for at bevise nøglefri signering og verificering (T49), og lad en reviewer kontrollere
   udkastet til API-standarder.
3. Byg headerne `Deprecation` og `Sunset` sammen med den første udfasning.
4. Bestil den uafhængige test (T42) mod en staging-build.
