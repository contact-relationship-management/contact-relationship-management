# Til sikkerhedsdirektøren

> Teknisk orientering, ikke juridisk rådgivning, certificering eller revision. Den angiver,
> hvad repositoriet viser pr. 2026-10-11. Intet her gør krav på akkreditering.

## Sikkerhedsbilledet i ét afsnit

Tjenesten **fejler sikkert**, styrer adgang efter rolle og efter post, skjuler følsomme værdier for roller, som ikke bør
se dem, og fører et revisionsspor, der afslører manipulation. Disse kontroller er implementeret og testet af forfatterne.
De er **ikke testet af nogen uafhængig** (T42), så betragt hvert udsagn nedenfor som "bygget og selvtestet", ikke
"verificeret".

## Kontroller og hvor de kan efterprøves

| Område | Hvad der er på plads | Hvor man ser efter |
|---|---|---|
| Autentificering | PASETO v4.public verificeret offline, valgfri Keycloak; produktion nægter at starte med autentificering slået fra | [auth](../../../../spec/auth.md) |
| Autorisation | Attributbaseret: læse, skrive, slette, destruktivt; en `mask`-forpligtelse; kontrol på postniveau; revision, opbevaring og break-glass-gennemgang kun for administratorer | [auth](../../../../spec/auth.md), persona-matrixtest |
| Eksponering af data | Maskering på poster, aggregater og indsigt; en lækage i kontodetaljer blev fundet og rettet (T47) | [tasks](../../../../spec/tasks.md) |
| Nødadgang | Skriftlig begrundelse, loft på 60 minutter, kun læsning, obligatorisk gennemgang af en anden, revideret, delt gennem Redis | break-glass-opgaver T44 |
| Revision | SHA-256-hashkæde i samme transaktion, trigger der kun tilføjer, verificeringsendepunkt; læsninger revideres | [audit](../../../../spec/audit.md) |
| Modstand mod misbrug | Hastighedsgrænse (600 i minuttet) og spærring (10 fejl på 5 minutter), delt via Redis; probes undtaget | T43 |
| Browser og transport | Streng CSP med hashes, afvisning af frames, `nosniff`, referrer-politik, lukket CORS, sikre headere; intet token i browserens JavaScript (BFF) | UI `hooks.server.ts` |
| Databeskyttelse | AES-256-GCM på to kolonner med rotation; lokale nøgler, ingen KMS | T41 |
| Forsyningskæde | `cargo audit`, `cargo deny`, `pnpm audit`, SBOM'er, hærdede billeder uden root fastlåst med digest, ugentlige Dependabot-opdateringer, et udgivelsesworkflow der signerer billeder og SBOM'er nøglefrit | [control-mapping](../../../../spec/compliance/control-mapping.md), [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Fejl | Hver fejl er ét RFC 9457-problemdokument; en `401` nævner det accepterede skema | [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) |
| Hukommelsessikkerhed | `forbid(unsafe_code)`, clippy pedantic | `src/lib.rs` |

## Kendte svagheder (angivet, ikke skjult)

- **Ingen uafhængig penetrationstest eller trusselsmodellering** (T42). Trusselsmodellen i
  [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) er et resumé, ikke en struktureret model.
- **Udgivelsessignering har ikke kørt på en rigtig udgivelse** (T49). Workflowet og verificeringskommandoerne findes;
  indtil et mærke er skubbet, er de utestede. Actions er fastlåst med commit-hash.
- **Feltkryptering** dækker to kolonner med lokale nøgler, ingen KMS.
- **Fornyet autentificering** før destruktive handlinger er ikke bygget.
- **Redis rummer personoplysninger** (cachelagret indsigt, tildelingsnøgler) og skal beskyttes som databasen: TLS,
  adgangskode, ingen offentlig eksponering.
- **Break-glass** kan ikke tilbagekaldes, kun udløbe, og ingen alarm er koblet på.
- **Hastighedsgrænse og spærring** falder tilbage til tællinger pr. replika, hvis Redis fejler; spanden for
  ikke-autentificerede er pr. adresse, og bag en proxy, der ikke står i `TRUSTED_PROXIES`, er det proxyens, så en
  angriber kan spærre andre ikke-autentificerede kaldere ude. En kalder med et gyldigt token tælles efter sit subjekt.
- **Genindlæsning af politik** viser sig i cachelagret indsigt efter op til 30 sekunder.
- **Udlogning ved inaktivitet** sker i klienten.
- **Idempotensnøgler** er valgfrie i API'en (T52): en klient, der ikke sender `Idempotency-Key`, er ikke beskyttet mod,
  at et genforsøgt oprettelseskald duplikerer en post. Den medfølgende brugerflade sender en.

## Hvad den driftsansvarlige skal gøre

1. Hold autentificering slået til; levér produktionsnøgler og `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL`.
2. Giv alarm ved `break_glass_granted` og gennemgå tildelinger hurtigt.
3. Forankr revisionskædens hoved uden for databasen og verificér det (`GET /api/audits/verify`) efter en plan.
4. Beskyt PostgreSQL og Redis (netværk, TLS, legitimationsoplysninger, sikkerhedskopier, kryptering i hvile); roter
   feltnøgler med den medfølgende opgave.
5. Flet Dependabot-opdateringerne; kør forsyningskædetrinnet igen; verificér en signeret udgivelse, før den udrulles.
6. Følg håndbogen for hændelsesrespons ([operations-runbook](../../../../spec/compliance/operations-runbook.md)).

## Håndtering af sårbarheder

Rapporter går gennem [SECURITY.md](../../../../SECURITY.md). Bestil den uafhængige test (T42) mod en staging-build før
rigtige data, og test igen efter væsentlige ændringer.

## Foreslåede sikkerhedshandlinger i rækkefølge

1. Afgræns og bestil den uafhængige penetrationstest (T42).
2. Skub et første udgivelsesmærke for at bevise signering og verificering (T49).
3. Kobl alarmering på break-glass og på fejl i verificeringen af revisionskæden.
4. Sæt Redis bag TLS og legitimationsoplysninger i alle miljøer.
5. KMS-baserede feltnøgler og bredere feltdækning (resten af T41).

Se også oversigten for [teknologidirektøren](../teknologidirektør/index.md) og for
[risikodirektøren](../risikodirektør/index.md).
