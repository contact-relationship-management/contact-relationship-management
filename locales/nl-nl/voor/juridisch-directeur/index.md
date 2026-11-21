# Voor de juridisch directeur

> Technische toelichting, geen juridisch advies, certificering of audit. Ze vermeldt
> wat de repository laat zien per 2026-10-11. Niets hier maakt aanspraak op accreditatie.

## Juridische positie in het kort

De repository is **niet klaar voor productie** en bevat synthetische gegevens. Ze beweert geen
naleving van enig regime. Ze implementeert technische functies die er meerdere
ondersteunen en noemt wat een uitrollende partij moet toevoegen ([regulatory](../../../../spec/regulatory.md)).

## Betrokken regimes

| Regime | Wat de software ondersteunt | Wat u moet aanleveren |
|---|---|---|
| UK/EU GDPR (AVG) | Toestemmingsgeschiedenis, export voor inzage, wissing (geweigerd zolang er openstaande verplichtingen zijn), bewaarsweep, gegevensminimalisatie, audit van leesacties | Rechtsgrond per doel, DPIA ([sjabloon](../../../../spec/compliance/dpia-template.md)), verwerkingsregister, bewaartermijnen, rollen van verwerkingsverantwoordelijke en verwerker |
| PECR / e-privacy | Verzending alleen met toestemming, onmiddellijke afmelding | Identificatie van de afzender en beslissingen over soft opt-in voor een echte afzender |
| HIPAA (bij Amerikaanse gezondheidsgegevens) | Auditcontroles, toegangsbeheer, integriteit, functies voor transportbeveiliging | Een business associate agreement, risicoanalyse, beleid, training van medewerkers. De software maakt u niet conform |
| NHS DTAC / DCB0129/0160 | Conceptbewijs ([dtac-supporting](../../../../spec/compliance/dtac-supporting.md)) | Benoemde clinical safety officer, gevarenlogboek, certificeringen. De beoordeling zegt dat DTAC waarschijnlijk niet van toepassing is |
| Toegankelijkheid (WCAG 2.2 AA, publieke sector) | Geautomatiseerde controles en een eerlijke verklaring | Een handmatige audit (T48) vóór elke bewering van naleving |
| Overheidsbeveiliging (NIST 800-53, ISO 27001, Cyber Essentials) | Controlemapping ([control-mapping](../../../../spec/compliance/control-mapping.md)) | Certificering en onafhankelijke zekerheid |

## Contracten die u nodig hebt en die nog niet bestaan

1. Verwerkersovereenkomst en voorwaarden voor internationale doorgifte met de e-mailprovider (T39).
2. Opdrachtomschrijving en vertrouwelijkheidsvoorwaarden met het bureau voor de penetratietest (T42).
3. Hosting- en subverwerkersvoorwaarden voor PostgreSQL, Redis en logs.
4. Licentie: zie [LICENSE.md](../../../../LICENSE.md); de repository bevat ook
   verklaringen over beveiliging, governance en gebruik van AI.

## Specifieke juridische aandachtspunten

- **Wissing** anonimiseert de contactpersoon en wordt geweigerd zolang er een openstaande deal, een ticket
  of een actieve nurture-inschrijving bestaat; de auditketen bewaart records van acties,
  dus regel in uw DPIA hoe dat samenhangt met wissingsverzoeken.
- **Break-glass**-leesacties van gemaskeerde gegevens zijn alleen rechtmatig op uw gedocumenteerde grondslag;
  het redenveld is bewijs, geen bevoegdheid.
- **Scoring** van leads: als die ooit significante beslissingen over
  mensen beïnvloedt, gelden de regels voor geautomatiseerde besluitvorming ([regulatory](../../../../spec/regulatory.md)).
- **API-toezeggingen:** [api-lifecycle](../../../../spec/compliance/api-lifecycle.md)
  stelt een opzegtermijn van 180 dagen voordat een API-wijziging wordt verwijderd. Dat is de
  keuze van de beheerder; als klanten erop gaan vertrouwen, beslis dan of het in een contract hoort.
- **Publieke beweringen:** beweer geen naleving, certificering of goedkeuring.
