# Voor de beveiligingsdirecteur

> Technische toelichting, geen juridisch advies, certificering of audit. Ze vermeldt
> wat de repository laat zien per 2026-10-11. Niets hier maakt aanspraak op accreditatie.

## Beveiligingshouding in één alinea

De dienst **faalt veilig**, regelt toegang per rol en per record, verbergt
gevoelige waarden voor rollen die ze niet horen te zien en bewaart een auditspoor
dat manipulatie zichtbaar maakt. Die maatregelen zijn door de auteurs geïmplementeerd en getest. Ze zijn
**door niemand onafhankelijk getest** (T42), dus beschouw elke bewering hieronder
als "gebouwd en zelf getest", niet als "geverifieerd".

## Maatregelen en waar u ze kunt controleren

| Gebied | Wat aanwezig is | Waar te kijken |
|---|---|---|
| Authenticatie | PASETO v4.public offline geverifieerd, optioneel Keycloak; productie weigert te starten als authenticatie uit staat | [auth](../../../../spec/auth.md) |
| Autorisatie | Attribuutgebaseerd: lezen, schrijven, verwijderen, destructief; een `mask`-verplichting; controles op recordniveau; audit, bewaring en break-glass-beoordeling alleen voor beheerders | [auth](../../../../spec/auth.md), persona-matrixtest |
| Blootstelling van gegevens | Maskering op records, aggregaten en insights; een lek in accountdetails is gevonden en verholpen (T47) | [tasks](../../../../spec/tasks.md) |
| Noodtoegang | Schriftelijke reden, limiet van 60 minuten, alleen lezen, verplichte beoordeling door een tweede persoon, geaudit, gedeeld via Redis | break-glass-taken T44 |
| Audit | SHA-256-hashketen in dezelfde transactie, append-only trigger, verificatie-eindpunt; leesacties geaudit | [audit](../../../../spec/audit.md) |
| Weerstand tegen misbruik | Snelheidsbeperking (600 per minuut) en blokkade (10 mislukkingen in 5 minuten), gedeeld via Redis; probes uitgezonderd | T43 |
| Browser en transport | Strikte CSP met hashes, framewering, `nosniff`, referrerbeleid, gesloten CORS, veilige headers; geen token in browser-JavaScript (BFF) | UI `hooks.server.ts` |
| Gegevensbescherming | AES-256-GCM op twee kolommen met rotatie; lokale sleutels, geen KMS | T41 |
| Toeleveringsketen | `cargo audit`, `cargo deny`, `pnpm audit`, SBOM's, geharde images zonder root met digest-vastzetting, wekelijkse Dependabot-updates, een release-workflow die images en SBOM's sleutelloos ondertekent | [control-mapping](../../../../spec/compliance/control-mapping.md), [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Fouten | Elke fout is één RFC 9457-probleemdocument; een `401` noemt het geaccepteerde schema | [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) |
| Geheugenveiligheid | `forbid(unsafe_code)`, clippy pedantic | `src/lib.rs` |

## Bekende zwakheden (benoemd, niet verborgen)

- **Geen onafhankelijke penetratietest of dreigingsmodellering** (T42). Het
  dreigingsmodel in [dtac-supporting](../../../../spec/compliance/dtac-supporting.md)
  is een samenvatting, geen gestructureerd model.
- **Het ondertekenen van releases is niet uitgevoerd op een echte release** (T49). De workflow en de
  verificatieopdrachten bestaan; tot er een tag wordt gepusht, zijn ze ongetest. De actions
  zijn vastgezet op commit-hash.
- **Veldversleuteling** dekt twee kolommen met lokale sleutels, geen KMS.
- **Stapsgewijze herauthenticatie** vóór destructieve acties is niet gebouwd.
- **Redis bewaart persoonsgegevens** (gecachete insights, toekenningssleutels) en moet worden beschermd
  zoals de database: TLS, wachtwoord, geen openbare blootstelling.
- **Break-glass** kan niet worden ingetrokken, alleen verlopen, en er is geen alarm gekoppeld.
- **Snelheidsbeperking en blokkade** vallen terug op tellingen per replica als Redis uitvalt; de
  ongeauthenticeerde bucket geldt per adres, en achter een proxy die niet in
  `TRUSTED_PROXIES` staat is dat van de proxy, zodat een aanvaller andere
  ongeauthenticeerde aanroepers kan buitensluiten. Een aanroeper met een geldig token wordt op zijn subject geteld.
- **Beleidsherladingen** zijn tot 30 seconden later zichtbaar in gecachete insights.
- **Uitloggen bij inactiviteit** gebeurt aan de clientkant.
- **Idempotentiesleutels** zijn optioneel in de API (T52): een client die
  `Idempotency-Key` niet meestuurt, is niet beschermd tegen een opnieuw geprobeerde aanmaak die een record dupliceert.
  De meegeleverde UI stuurt er een mee.

## Wat de uitrollende partij moet doen

1. Houd authenticatie aan; lever productiesleutels en `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL`.
2. Alarmeer op `break_glass_granted` en beoordeel toekenningen snel.
3. Verankeer de kop van de auditketen buiten de database en verifieer hem
   (`GET /api/audits/verify`) volgens een schema.
4. Bescherm PostgreSQL en Redis (netwerk, TLS, inloggegevens, back-ups, versleuteling
   in rust); roteer veldsleutels met de meegeleverde taak.
5. Voeg de Dependabot-updates samen; draai de fase voor de toeleveringsketen opnieuw; verifieer een ondertekende
   release voordat u hem uitrolt.
6. Volg het handboek voor incidentafhandeling
   ([operations-runbook](../../../../spec/compliance/operations-runbook.md)).

## Omgaan met kwetsbaarheden

Meldingen lopen via [SECURITY.md](../../../../SECURITY.md). Laat de onafhankelijke
test (T42) uitvoeren tegen een staging-build vóór echte gegevens, en
test opnieuw na een ingrijpende wijziging.

## Voorgestelde beveiligingsacties, op volgorde

1. Bepaal de scope van de onafhankelijke penetratietest en geef opdracht (T42).
2. Push een eerste release-tag om ondertekening en verificatie te bewijzen (T49).
3. Koppel alarmen voor break-glass en voor mislukte verificatie van de auditketen.
4. Zet Redis in elke omgeving achter TLS en inloggegevens.
5. Op KMS gebaseerde veldsleutels en bredere veldendekking (rest van T41).

Zie ook het overzicht voor de [technologiedirecteur](../technologiedirecteur/index.md) en het
overzicht voor de [risicodirecteur](../risicodirecteur/index.md).
