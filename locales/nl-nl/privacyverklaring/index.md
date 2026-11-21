# Privacyverklaring: wat in uw browser wordt bewaard

Deze verklaring behandelt twee dingen: deze website (`contact-relationship-management.github.io`) en de
gebruikersinterface van Contact Relationship Management zoals deze repository die levert. Ze noemt alles wat een van
beide in uw browser opslaat, en waarom.

Een organisatie die het CRM draait, verwerkt persoonsgegevens over haar contactpersonen. Die verwerking valt onder de
eigen privacyverklaring van die organisatie, niet onder deze ([spec/regulatory.md](../../../spec/regulatory.md)).

## Wat wordt opgeslagen, en waarom

Elk item bevat een keuze die u hebt gemaakt: een thema, een taal en landinstelling, een tekstgrootte, een tijdzone of
de indeling van een tabel. Geen van deze items bevat uw naam, e-mailadres, IP-adres of een andere identificatie, en
geen ervan wordt gebruikt om u te volgen, te meten of advertenties te tonen. Elk bestaat zodat de volgende pagina die u
opent dezelfde keuze gebruikt en u die niet opnieuw hoeft te maken.

Er wordt niets opgeslagen totdat u een keuze maakt. Een bezoeker die niets wijzigt, heeft niets opgeslagen.

### Op deze website

| Naam | Soort | Wat het bevat | Bewaard gedurende |
| --- | --- | --- | --- |
| `lily-theme` | Cookie en `localStorage` | `system` ("Volg dit apparaat") of de naam van een thema, zoals `dark` | Cookie: 400 dagen. `localStorage`: totdat u het wist |
| `lily-text-size` | `localStorage` | De tekstgrootte die u hebt gekozen | Totdat u het wist |
| `lily-time-zone` | `localStorage` | De tijdzone die u hebt gekozen | Totdat u het wist |

Het thema wordt bewaard in een cookie en in `localStorage`, zodat het er nog is als uw browser een van beide wist. De
pagina's van de site zijn statische bestanden en geen enkel programma op de site leest de cookie. Omdat het een
cookie is, stuurt uw browser hem bij elk verzoek aan de site mee naar GitHub Pages, dat de site host.

De taal en landinstelling van de site maken deel uit van het paginaadres, zoals `/de-de/`, dus daarvoor wordt niets
opgeslagen.

### In de gebruikersinterface van het CRM

| Naam | Soort | Wat het bevat | Bewaard gedurende |
| --- | --- | --- | --- |
| `crm-theme` | Cookie | `system` ("Volg dit apparaat") of de naam van een thema | 400 dagen |
| `mxi.crm.locale` | `localStorage` | De taal en landinstelling die u hebt gekozen, zoals `de-de` | Totdat u het wist |
| `lily-text-size` | `localStorage` | De tekstgrootte die u hebt gekozen | Totdat u het wist |
| `crm-grid-<table>` | `localStorage` | Kolombreedtes, verborgen kolommen en sorteervolgorde van één tabel | Totdat u het wist |
| `crm-grid-order-<table>` | `localStorage` | De kolomvolgorde van één tabel | Totdat u het wist |
| `__Host-mxi_session` | Cookie | Een willekeurige identificatie voor uw aangemelde sessie | Totdat u zich afmeldt of de sessie eindigt |

De server van het CRM leest de cookie `crm-theme`, zodat hij elke pagina in uw thema kan verzenden en de pagina niet
eerst in een ander thema verschijnt. Het CRM leest hem nooit voor een ander doel.

De sessiecookie is geen keuze. Hij wordt gezet wanneer u zich aanmeldt, de scripts van de browser kunnen hem niet
lezen (`HttpOnly`) en zonder hem kunt u niet aangemeld blijven.

## Wie het ontvangt

Deze items blijven in uw browser. Niets ervan wordt naar een derde gestuurd, afgezien van de themacookie van de site,
die uw browser naar de host van de site stuurt zoals hierboven beschreven. Cookies zijn first-party (`SameSite=Lax`)
en `Secure` op een `https`-pagina.

## Hoe u het verwijdert

Om een keuze te wijzigen, maakt u haar opnieuw. Om alles te verwijderen, wist u de gegevens van deze site in de
instellingen van uw browser: de volgende pagina gebruikt dan weer de standaardwaarden, dat wil zeggen de lichte of
donkere instelling van uw apparaat, de standaardtekstgrootte en de standaardtaal van de pagina.
