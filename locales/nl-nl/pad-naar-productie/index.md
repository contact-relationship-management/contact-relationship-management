# Pad naar productie

> Contact Relationship Management is niet klaar voor productie. Deze pagina legt uit wat productie hier betekent en
> hoe ver elk onderdeel is gevorderd.

De software draait op synthetische gegevens. E-mail wordt gesimuleerd, tenzij een echte mailmodus is ingesteld. Voer
er geen echte persoonsgegevens in.

## Wat productie betekent

Een productie-implementatie bevat de gegevens van echte mensen en verstuurt echte berichten aan hen. Voor dit project
is daarvoor het volgende nodig:

- **Echte aanmelding.** Elk verzoek is geauthenticeerd en de toegangsregels komen overeen met de rollen van de
  organisatie die de software inzet.
- **Rechtmatig verzenden.** Elk bericht heeft vastgelegde toestemming en een rechtsgrond volgens het recht van elke
  plaats waarheen het wordt verzonden, en loopt via een echte e-mailprovider die bounces, klachten en afmeldingen
  afhandelt.
- **Onafhankelijke zekerheid.** Mensen buiten het project hebben de beveiliging en de toegankelijkheid getest en de
  bevindingen zijn verholpen of aanvaard.
- **Beheer.** Back-ups die zijn teruggezet, monitoring, een incidentproces en iemand die bereikbaar is.
- **Bewijs van stabiliteit.** Lange runs onder realistische belasting, op hardware zoals die van de implementatie,
  zonder fouten of vertraging.

Of iets klaar is voor productie wordt per implementatie bepaald. Een uitrollende partij kan sommige poorten alleen voor
de eigen organisatie en de eigen ontvangers sluiten. De technische beoordeling achter deze pagina is
[spec/plan.md](../../../spec/plan.md). De openstaande poorten staan in [spec/tasks.md](../../../spec/tasks.md).

## Gereedheid

Elk percentage is een schatting van 2026-11-13 op basis van de takenlijst, geen meting.

- **Authenticatie en toegangsbeheer** (70%): Aanmelding met PASETO-tokens of Keycloak/OIDC, attribuutgebaseerde
  toegangsregels met controles op recordniveau en een productiemodus die weigert te starten zonder authenticatie zijn
  gebouwd. Geen echte implementatie heeft ze met eigen attributen ingeschakeld, en de handhavingsmatrix dekt nog niet
  elke handler (CRM-G1).
- **E-mailbezorging** (40%): Een gesimuleerde verzender is de standaard en er is een SMTP-verzender. Er is geen
  adapter voor een e-mailprovider met webhooks voor bounces en klachten, en geen onderdrukkingslijst (CRM-T39).
- **Gegevensbescherming en e-marketingrecht** (15%): Toestemmingsregistraties, inzage, wissing en bewaartermijnen zijn
  gebouwd. Voor dit project is het recht van geen enkele regio gelezen en de beoordeling van het echte verzendpad staat
  open (CRM-G2, [regulatory](../../../spec/regulatory.md)).
- **Onafhankelijke beveiligingszekerheid** (10%): Er is een scope voor een penetratietest geschreven. Er is geen
  penetratietest, formele risicoanalyse of accreditatie uitgevoerd (CRM-T42). De AI-beveiligingsaudits zijn geen
  onafhankelijke zekerheid.
- **Toegankelijkheid** (50%): Geautomatiseerde axe-core-controles tegen WCAG 2.2 A en AA slagen op elke paginaroute.
  Er is geen handmatige audit met alleen toetsenbord, schermlezers of 400% zoom uitgevoerd (CRM-T48).
- **Vertalingen** (30%): De interface is beschikbaar in acht talen. Teksten in andere talen dan Engels zijn door een
  machine geschreven en geen enkele spreker heeft ze beoordeeld.
- **Beheer en operatie** (30%): Een operationeel handboek behandelt back-up en herstel, sleutelbeheer, versleuteling
  in rust, incidentafhandeling en bewaring van logs. Er is geen herstel getest en de go-livechecklist van het
  handboek staat open.
- **Prestaties en stabiliteit** (20%): Er zijn belastingtests en een langlopende stabiliteitstest. De stabiliteitstest
  heeft twee minuten tegen de demostack gedraaid; er is geen lange run en geen basislijn op hardware die lijkt op
  productie.
- **Release-integriteit** (85%): Release-images zijn multi-architectuur, ondertekend en geattesteerd, en hun
  handtekeningen kloppen. Acties en images zijn vastgezet met een digest en de geschiedenis wordt gescand op
  geheimen. De amd64-helft van een opgehaalde release-image is niet gecontroleerd en er is nog geen tweede
  codehost (CRM-T192).
- **Continue integratie** (100%): Eén script voert elke controle uit op GitHub Actions en Woodpecker. Beide
  Rust-codebases en de UI hebben ondergrenzen van 95% regeldekking; poorten voor browser, lint, toeleveringsketen,
  mutatie op gewijzigde regels en wekelijkse volledige mutatie zijn aanwezig (CRM-T94).
- **Demobeveiliging** (100%): Elke bezoeker van de rondleiding krijgt een eigen kopie van de demogegevens, begrensd op
  200 kopieën. De rondleiding staat standaard uit; stel `CONTACT_RELATIONSHIP_MANAGEMENT_DEMO_TOUR=on` in om haar aan
  te bieden (CRM-T92).

## Hoe deze pagina wordt bijgehouden

Wanneer een poort sluit, wijzig dan de schatting en het bewijs in dezelfde commit als het werk en koppel de taak.
