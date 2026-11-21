# Governance, risico, compliance

> Technische toelichting, geen juridisch advies, certificering of audit. Ze vermeldt
> wat de repository laat zien per 2026-10-11. Niets hier maakt aanspraak op accreditatie.

## Waar te beginnen

Het bewijspakket is [spec/compliance](../../../../spec/compliance/index.md). De
basisregel is die waaraan u het team moet houden: een maatregel is alleen **Implemented**
wanneer code of configuratie haar afdwingt en een test of controle haar vastlegt; **Deployer**
wanneer uw organisatie haar moet leveren; **Gap** wanneer geen van beide vandaag waar is.

| Document | Gebruik het voor |
|---|---|
| [control-mapping](../../../../spec/compliance/control-mapping.md) | NIST SP 800-53, HIPAA Security Rule, ISO 27001 Annex A, AVG art. 32, elk gemarkeerd als Implemented, Deployer of Gap |
| [dpia-template](../../../../spec/compliance/dpia-template.md) | Gegevensbeschermingseffectbeoordeling, vooraf ingevuld met de gegevensstromen van dit systeem |
| [operations-runbook](../../../../spec/compliance/operations-runbook.md) | Hardening, back-up en herstel, sleutels, logging, incidentafhandeling |
| [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) | Concept-NHS-DTAC-verklaringen die de fabrikant kan overnemen |
| [gds-api-review](../../../../spec/compliance/gds-api-review.md) | Zelfbeoordeling tegen de Britse overheidsstandaarden voor API's (een concept, niet gecontroleerd tegen de gepubliceerde tekst) |
| [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) | Foutformaat (RFC 9457), wat als brekende wijziging geldt, de stappen voor uitfasering en de opzegtermijn |
| [NHS DTAC v2.0-beoordeling](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md) | Resultaat per vraag: waarschijnlijk buiten de scope van DTAC, en het zou zakken voor de van toepassing zijnde onderdelen wegens gebrek aan organisatorisch bewijs |
| [plan](../../../../spec/plan.md) | Oordeel over gereedheid (niet klaar) en het register van hiaten |
| [tasks](../../../../spec/tasks.md) | De taak van elk hiaat, de afhandeling en de benoemde grenzen |

## Governance

- **Beslissingsrechten:** [GOVERNANCE.md](../../../../GOVERNANCE.md), [MAINTAINERS.md](../../../../MAINTAINERS.md),
  [CODEOWNERS](../../../../CODEOWNERS). Regels voor bijdragen en hulp van AI staan in
  [CONTRIBUTING.md](../../../../CONTRIBUTING.md) en [AI_STATEMENT.md](../../../../AI_STATEMENT.md).
- **Wijzigingsbeheer:** de specificatie is de bron van waarheid; code en tests
  verwijzen ernaar. CI draait bij twee providers en `scripts/ci-check.sh` reproduceert het.
  Regel in de repository: verzwak geen test of poort om iets te laten slagen.
- **Openbaarmaking:** [SECURITY.md](../../../../SECURITY.md) is de meldroute.

## Risico

Het risicooverzicht staat in het overzicht voor de [risicodirecteur](../risicodirecteur/index.md). Kort gezegd: de hoogste
restrisico's zijn beveiliging die niet door buitenstaanders is getest (T42), niet-gekwantificeerde
toegankelijkheid (T48) en gegevensclassificatie, wat de keuze van de overnemende partij is.

## Compliance: wat echt is en wat niet

| Gebied | Positie |
|---|---|
| Toegangsbeheer | Geïmplementeerd en getest: maskering per rol, controles op recordniveau, operationele leesacties alleen voor beheerders, break-glass met verplichte beoordeling door een tweede persoon |
| Audit | Geïmplementeerd en getest: hashgeketend, append-only, binnen de transactie, audit van leesacties inclusief lijsten, deallijsten en insights. **Deployer:** verankeer de kop van de keten buiten de database |
| Privacyrechten | Geïmplementeerd: export voor inzage, wissing (geweigerd zolang verplichtingen openstaan), bewaarsweep. **Deployer:** rechtsgrond, bewaartermijnen, verwerkingsregister |
| Versleuteling | Gedeeltelijk: AES-256-GCM op veldniveau op twee kolommen met lokale sleutels, geen KMS. Versleuteling in rust en onderweg is aan de uitrollende partij |
| Toeleveringsketen | Geïmplementeerd: `cargo audit`, `cargo deny`, `pnpm audit`, SBOM's, basisimages vastgezet met digest, Dependabot-updates. **Gap:** de workflow voor ondertekende releases is niet uitgevoerd op een echte release (T49) |
| Toegankelijkheid | Gedeeltelijk: geautomatiseerde axe-scans en een eerlijke openbare verklaring. **Gap:** geen handmatige audit (T48), vertalingen niet beoordeeld |
| Onafhankelijke zekerheid | **Gap:** geen (T42) |
| E-mail- en marketingrecht | Verzendpad waarbij toestemming voorop staat is geïmplementeerd; echt verzenden bestaat alleen als SMTP-pad voor ontwikkeling; de afmeldlink, de onderdrukkingslijst en een ondertekende uitkomstwebhook bestaan, maar er is geen provider geïntegreerd en de verwerkersovereenkomst en de juridische beoordeling staan open (T39) |

## Maatregelen die een eigenaar in uw organisatie nodig hebben

1. Beoordeling en alarmering voor break-glass-toekenningen (alarmeer op `break_glass_granted`).
2. De kop van de auditketen verankeren en volgens een schema verifiëren
   (`GET /api/audits/verify`).
3. Bewijs van back-up en herstel, en een repetitie van sleutelrotatie.
4. Afhandeling van instroom, doorstroom en uitstroom in uw identiteitsprovider.
5. Bewaartermijn voor de audittabel, die groeit met het leesverkeer.
6. Redis beschermen als de database: het bevat gecachete insightgegevens.
7. Dependabot-updates samenvoegen en een ondertekende release verifiëren voordat u hem uitrolt.

## Waarvoor u uw goedkeuring moet weigeren

Elke verklaring dat het systeem conform, gecertificeerd, HIPAA-conform of door de NHS
goedgekeurd is. De verdedigbare verklaring is beperkter: maatregelen zijn in kaart gebracht, de
geïmplementeerde zijn getest, en de hiaten zijn benoemd en worden bijgehouden.
