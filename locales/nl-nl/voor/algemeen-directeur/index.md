# Voor de algemeen directeur

> Technische toelichting, geen juridisch advies, certificering of audit. Ze vermeldt
> wat de repository laat zien per 2026-10-11. Niets hier maakt aanspraak op accreditatie.

## De beslissing in één alinea

Dit is een CRM op demoniveau met een uitzonderlijk sterke basis voor beveiliging en
privacy. Het is **niet klaar** om echte patiënten- of burgergegevens te bevatten, en de
repository zegt dat zelf ([plan.md](../../../../spec/plan.md)). Het in gebruik nemen
ervan is een besluit om het resterende externe werk te financieren, niet om het in te schakelen.

## Wat er is

- Een werkend CRM voor verkoop, marketing, ondersteuning en analyse, met een Rust-API
  en een web-front-end, gebouwd volgens een schriftelijke specificatie.
- Marketing waarbij toestemming voorop staat, toegangsbeheer dat gevoelige waarden verbergt
  voor rollen die ze niet horen te zien, een auditspoor dat manipulatie zichtbaar maakt, inzage
  en wissing bij privacyverzoeken, en noodtoegang ("break-glass") die
  gemotiveerd, tijdelijk en beoordeeld is.
- 54 van de 58 bijgehouden werkitems zijn klaar ([tasks.md](../../../../spec/tasks.md)).

## Wat tussen dit en echt gebruik staat

| Blokkade | Waarom het ertoe doet | Wie het kan oplossen |
|---|---|---|
| Geen echte e-mailbezorging (T39) | Marketing wordt gesimuleerd; een echte provider vraagt contracten en een verwerkersovereenkomst | U kiest de leverancier; juridische zaken tekent |
| Geen onafhankelijke beveiligingstest (T42) | Ziekenhuizen en overheidskopers eisen er meestal een; onze eigen tests zijn niet onafhankelijk | Een extern bureau |
| Geen handmatige toegankelijkheidsaudit (T48) | Overheidsinstanties moeten een toegankelijkheidsverklaring publiceren waar ze achter kunnen staan | Mensen met hulptechnologie |
| Het ondertekenen van releases is gebouwd maar nog niet uitgevoerd op een echte release, en geen reviewer heeft het concept van de API-standaarden gecontroleerd (T49) | Inkoopchecklists vragen om beide | Engineering (een eerste release taggen); een externe reviewer |

## Beslissingen die bij u liggen

1. **Welke markt eerst.** De beoordeling volgens de NHS Digital Technology Assessment Criteria
   ([docs](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md))
   concludeert dat het product waarschijnlijk buiten de scope van DTAC valt en
   zou zakken voor de van toepassing zijnde onderdelen wegens gebrek aan organisatorisch bewijs
   (benoemde clinical safety officer, certificeringen). Kies de route voordat u er geld in steekt.
2. **Of onafhankelijke zekerheid wordt gefinancierd** (T42, T48). Zonder die zekerheid is de eerlijke
   bewering: "gebouwd volgens een hoge standaard, niet getest door buitenstaanders".
3. **Wie het risico draagt** zodra er echte gegevens worden bewaard. Zie de overzichten voor de [risicodirecteur](../risicodirecteur/index.md)
   en de [juridisch directeur](../juridisch-directeur/index.md).

## Wat u extern niet moet zeggen

Beweer geen naleving, certificering, "HIPAA-conform" of "NHS-goedgekeurd".
Het bewijspakket ([compliance](../../../../spec/compliance/index.md)) ondersteunt een beperktere,
ware verklaring: beheersmaatregelen zijn in kaart gebracht, de geïmplementeerde zijn getest, hiaten worden benoemd.
