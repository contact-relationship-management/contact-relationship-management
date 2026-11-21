# För teknikchef

> Teknisk sammanfattning, inte juridisk rådgivning, certifiering eller revision. Den anger
> vad arkivet visar per 2026-10-11. Inget här gör anspråk på ackreditering.

## Arkitektur

- **Tjänst:** Rust på Axum och axa (sqlx 0.9), PostgreSQL 18, Redis för cache,
  delade räknare och spridning av break-glass. `forbid(unsafe_code)`, clippy
  pedantic. Sanningskälla för beteendet är specifikationen
  ([index](../../../../spec/index.md), [architecture](../../../../spec/architecture.md)).
- **Gränssnitt:** SvelteKit 3 / Svelte 5, en backend-for-frontend-proxy så att inget token någonsin
  når webbläsarens JavaScript, strikt CSP med hashar.
- **Autentisering:** PASETO v4.public verifieras offline, valfri Keycloak; attributbaserad
  åtkomstkontroll (läs-/skriv-/radera-/destruktiva åtgärder, ett `mask`-krav,
  kontroller på postnivå) från en delad motor.
- **Granskning:** SHA-256-hashkedja som skrivs i samma transaktion som ändringen,
  utlösare som bara läggs till, verifierbar på `GET /api/audits/verify`.
- **Dataskydd:** AES-256-GCM-fältkryptering med nyckelrotation på två
  kolumner; lokala nycklar, ingen KMS.

## Kvalitetsbelägg

Enhetstester (185), databastester (56 inklusive tillämpning, maskering, granskning,
sidindelning, cachning, idempotens, schemakontroller), front-end-enhetstester (106) och
Playwright (98) inklusive axe-tillgänglighetsskanningar. CI kör fmt, clippy, tester, front-end-bygge/lint och ett
steg för leveranskedjan (`cargo audit`, `cargo deny`, `pnpm audit`, CycloneDX-SBOM:er)
på GitHub Actions och Woodpecker. Ett kommando återskapar det: `scripts/ci-check.sh`.

## Teknisk skuld och ärliga begränsningar

- Sidindelningen använder en offsetmarkör (rader kan hoppas över eller upprepas om data ändras
  mitt under genomgången); ingen totalsumma, avsiktligt.
- Idempotensnycklar är frivilliga i API:et (T52): en klient måste skicka
  `Idempotency-Key` vid skapande för att skyddas mot att ett omförsök dubblerar
  en post. Det medföljande gränssnittet skickar en vid varje skapande.
- OpenAPI-scheman är handskrivna. De täcker begäranden, listor, detaljläsningar,
  instrumentpaneler, insiktsvyer, granskning, integritet och break-glass; tillståndsövergångar
  och upserts som returnerar den uppdaterade posten är otypade. Ett test misslyckas när en
  rutt och dokumentet inte stämmer överens, och andra jämför verkliga svar, även maskerade,
  mot schemana
  ([gds-api-review](../../../../spec/compliance/gds-api-review.md)).
- Fel är problemdokument enligt RFC 9457 (T54) med de tidigare medlemmarna `error` och
  `description` bevarade; fel från lager utanför routern (storleksgräns,
  timeout) konverteras inte. Policyn för versionshantering och utfasning är
  [api-lifecycle](../../../../spec/compliance/api-lifecycle.md); headrarna
  `Deprecation` och `Sunset` är specificerade men inte byggda.
- Fältkrypteringen täcker två kolumner; ingen KMS-integration.
- Hastighetsgräns- och cachetillstånd kräver Redis för korrekthet med flera repliker.
- Insiktsvyer maskeras per roll men cachas per anroparens token i 30 s.
- Förnyad autentisering (step-up) före destruktiva åtgärder kräver arbete hos identitetsleverantören.

## Rekommenderad teknisk ordning

1. KMS-baserade fältnycklar och bredare fälttäckning (resten av T41).
2. Tagga en första release för att bevisa nyckellös signering och verifiering (T49), och
   låt en granskare kontrollera utkastet till API-standarder.
3. Bygg headrarna `Deprecation` och `Sunset` tillsammans med den första utfasningen.
4. Beställ det oberoende testet (T42) mot ett staging-bygge.
