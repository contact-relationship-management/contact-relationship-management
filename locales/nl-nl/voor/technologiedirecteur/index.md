# Voor de technologiedirecteur

> Technische toelichting, geen juridisch advies, certificering of audit. Ze vermeldt
> wat de repository laat zien per 2026-10-11. Niets hier maakt aanspraak op accreditatie.

## Architectuur

- **Dienst:** Rust op Axum en axa (sqlx 0.9), PostgreSQL 18, Redis voor cache,
  gedeelde tellers en verspreiding van break-glass. `forbid(unsafe_code)`, clippy
  pedantic. De bron van waarheid voor gedrag is de specificatie
  ([index](../../../../spec/index.md), [architecture](../../../../spec/architecture.md)).
- **UI:** SvelteKit 3 / Svelte 5, een backend-for-frontend-proxy zodat geen token ooit
  browser-JavaScript bereikt, strikte CSP met hashes.
- **Auth:** PASETO v4.public offline geverifieerd, optioneel Keycloak; attribuutgebaseerd
  toegangsbeheer (acties lezen/schrijven/verwijderen/destructief, een `mask`-verplichting,
  controles op recordniveau) vanuit een gedeelde engine.
- **Audit:** SHA-256-hashketen geschreven in dezelfde transactie als de wijziging,
  append-only trigger, verifieerbaar via `GET /api/audits/verify`.
- **Gegevensbescherming:** AES-256-GCM-veldversleuteling met sleutelrotatie op twee
  kolommen; lokale sleutels, geen KMS.

## Bewijs van kwaliteit

Unittests (185), databasetests (56, waaronder handhaving, maskering, audit,
paginering, caching, idempotentie, schemacontroles), front-end-unittests (106) en
Playwright (98), inclusief axe-toegankelijkheidsscans. CI voert fmt, clippy, tests, front-end-build/lint en een
fase voor de toeleveringsketen uit (`cargo audit`, `cargo deny`, `pnpm audit`, CycloneDX-SBOM's)
op GitHub Actions en Woodpecker. Eén opdracht reproduceert het: `scripts/ci-check.sh`.

## Technische schuld en eerlijke grenzen

- Paginering gebruikt een offsetcursor (rijen kunnen worden overgeslagen of herhaald als gegevens
  halverwege de doorloop veranderen); bewust geen totaalaantal.
- Idempotentiesleutels zijn optioneel in de API (T52): een client moet
  `Idempotency-Key` meesturen bij een aanmaak om beschermd te zijn tegen een nieuwe poging die een
  record dupliceert. De meegeleverde UI stuurt er bij elke aanmaak een mee.
- OpenAPI-schema's worden met de hand geschreven. Ze dekken verzoeken, lijsten, detailweergaven,
  dashboards, insightweergaven, audit, privacy en break-glass; statusovergangen
  en upserts die het bijgewerkte record teruggeven zijn niet getypeerd. Een test faalt wanneer een
  route en het document verschillen, en andere testen echte antwoorden, ook gemaskeerde,
  tegen de schema's
  ([gds-api-review](../../../../spec/compliance/gds-api-review.md)).
- Fouten zijn RFC 9457-probleemdocumenten (T54) waarbij de eerdere leden `error` en
  `description` behouden blijven; fouten uit lagen buiten de router (groottelimiet,
  time-out) worden niet omgezet. Het beleid voor versiebeheer en uitfasering is
  [api-lifecycle](../../../../spec/compliance/api-lifecycle.md); de headers
  `Deprecation` en `Sunset` zijn gespecificeerd maar niet gebouwd.
- Veldversleuteling dekt twee kolommen; geen KMS-integratie.
- Staat van snelheidsbeperking en cache vereist Redis voor correctheid bij meerdere replica's.
- Insightweergaven worden per rol gemaskeerd maar 30 s per token van de aanroeper gecachet.
- Stapsgewijze herauthenticatie vóór destructieve acties vraagt werk aan de identiteitsprovider.

## Aanbevolen technische volgorde

1. Op KMS gebaseerde veldsleutels en bredere veldendekking (rest van T41).
2. Tag een eerste release om sleutelloze ondertekening en verificatie te bewijzen (T49) en
   laat een reviewer het concept van de API-standaarden controleren.
3. Bouw de headers `Deprecation` en `Sunset` samen met de eerste uitfasering.
4. Geef opdracht voor de onafhankelijke test (T42) tegen een staging-build.
