# Voor de operationeel directeur

> Technische toelichting, geen juridisch advies, certificering of audit. Ze vermeldt
> wat de repository laat zien per 2026-10-11. Niets hier maakt aanspraak op accreditatie.

## Wat het draaien hiervan vraagt

De dienst is één Rust-proces per replica plus PostgreSQL 18 en Redis; de
front-end is een Node-proces. Beide worden geleverd als geharde containerimages zonder root, met
basisimages die met een digest zijn vastgezet. Het operationele handboek is
[operations-runbook.md](../../../../spec/compliance/operations-runbook.md).

## Operationele feiten om rekening mee te houden

- **Faalt veilig bij het starten.** In productie weigert de dienst te starten als
  authenticatie uit staat, als de auditketen onbruikbaar is, of als vereiste sleutels en
  `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL` ontbreken. Een verkeerd geconfigureerde uitrol stopt in plaats van open te draaien.
- **Health en metrics.** `/_health/live`, `/_health/ready`, `/_health/startup` worden nooit
  snelheidsbeperkt, zodat een blokkade een gezonde pod niet kan herstarten. Metrics op
  `/metrics.prom`.
- **Misbruiklimieten** staan aan in productie: 600 verzoeken per minuut per client en een
  blokkade na 10 mislukte aanmeldingen in 5 minuten. Ze worden via Redis over
  replica's gedeeld; als Redis uitvalt, vallen ze terug op tellingen per replica.
- **Noodtoegang** (break-glass): elke aangemelde gebruiker kan een gemotiveerde
  leesverhoging nemen van maximaal 60 minuten. Iemand anders moet elke toekenning beoordelen.
  **Proces dat u moet draaien:** alarmeer op auditrijen `break_glass_granted` en
  beoordeel het aantal onbeoordeelde toekenningen (beheerpagina `/break-glass/review`).
- **Fouten** hebben één vorm (RFC 9457 `application/problem+json`, met `error` en
  `description` behouden), zodat logparsing en alarmregels op `status` en `type` kunnen vertrouwen.
- **Groei van de audit.** Leesacties worden geaudit, dus de audittabel groeit met het verkeer.
  Dimensioneer de bewaartermijn daarvoor. Een client die pollt, laat haar snel groeien.

## Openstaand operationeel werk

| Onderdeel | Stand |
|---|---|
| Oefeningen voor back-up en herstel, repetitie van sleutelrotatie | Beschreven in het handboek; **u** moet ze uitvoeren en bewijzen |
| De kop van de auditketen buiten de database verankeren | Uw plicht; de API meldt de kophash |
| Vastgezette image-digests vernieuwen | Dependabot opent wekelijks pull requests; **u** moet ze beoordelen en samenvoegen, anders raken de vastzettingen verouderd |
| Een ondertekende release verifiëren voordat u hem uitrolt | De opdrachten staan in het handboek; rol uit op digest en weiger een image dat de verificatie niet doorstaat. Niet getest tot een eerste getagde release |
| Echte e-mailprovider (T39) | Open; verandert het beeld van incidenten en leveranciers |
| Redis bevat insight-gegevens | Bescherm het zoals de database: TLS, wachtwoord, geen openbare blootstelling |

## Bekende beperkingen die het waard zijn het bereikbaarheidsteam te vertellen

Uitloggen bij inactiviteit gebeurt aan de clientkant; een ingetrokken token wordt direct geweigerd, maar een
herlading van het beleid is tot 30 seconden later zichtbaar in gecachete insights; break-glass kan niet
worden ingetrokken, alleen verlopen.
