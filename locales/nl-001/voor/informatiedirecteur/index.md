# Voor de informatiedirecteur

> Technische toelichting, geen juridisch advies, certificering of audit. Ze vermeldt
> wat de repository laat zien per 2026-10-11. Niets hier maakt aanspraak op accreditatie.

## Informatie- en gegevensoverzicht

Dit CRM bevat **relatiegegevens**: contactpersonen en accounts als omhulsels rond
identificatoren die in andere systemen worden bewaard, activiteiten, deals, tickets, toestemmingsgeschiedenis
en een auditlog. Demografische gegevens blijven bewust in de identiteitsregisters
([domain-model](../../../../spec/domain-model.md)). Als uw organisatie patiënten of
burgers als contactpersonen vastlegt, is de classificatie van die gegevens aan **u**;
het plan is geschreven volgens de strengere lezing ([plan](../../../../spec/plan.md)).

## Waar gegevens heen gaan

| Opslag | Bevat | Bescherming in de repository |
|---|---|---|
| PostgreSQL | Alle records, auditketen, break-glass-toekenningen | Veldversleuteling op twee kolommen; append-only audittrigger; versleuteling in rust is aan de uitrollende partij |
| Redis | Cache van insight-antwoorden (30 s), tellers voor snelheidsbeperking, sleutels van break-glass-toekenningen | Per aanroeper gesleuteld; **vereist TLS, een wachtwoord en geen openbare blootstelling** |
| Logs/metrics | Verzoekmetadata, Prometheus-metrics | Bewaartermijn is aan de uitrollende partij |
| E-mailprovider (toekomstig) | Berichtinhoud en ontvangers | Niet gebouwd (T39) |

## Integratie en identiteit

- Aanmelding komt van een identiteitsdienst die PASETO-tokens uitgeeft, of Keycloak.
  Instroom, doorstroom en uitstroom, MFA en sessiebeleid zijn dus **van uw identiteitsprovider**;
  het CRM handhaaft wat het token zegt en weigert verlopen tokens.
- De API is gedocumenteerd in OpenAPI ([gds-api-review](../../../../spec/compliance/gds-api-review.md)),
  geversioneerd via een header, gepagineerd met `Link`-headers en geeft fouten terug als
  RFC 9457-probleemdocumenten. De regels voor wijziging en uitfasering staan in
  [api-lifecycle](../../../../spec/compliance/api-lifecycle.md).
- SBOM's (CycloneDX) worden voor de dienst en de UI in CI geproduceerd; de release-workflow
  ondertekent ze en de images.

## Plichten voor informatiebeheer die bij u blijven

Bewijs van back-up en herstel, bewaartermijnen per tabel, het verankeren van de kop van de
auditketen buiten de database, KMS-bewaring van veldsleutels, verwerkersovereenkomsten
met elke leverancier en de gegevensbeschermingseffectbeoordeling
([dpia-template](../../../../spec/compliance/dpia-template.md)). De controlemapping
([control-mapping](../../../../spec/compliance/control-mapping.md)) markeert elke maatregel als
Implemented, Deployer of Gap.
