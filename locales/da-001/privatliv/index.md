# Privatlivserklæring: hvad der gemmes i din browser

Denne erklæring omfatter to ting: dette websted (`contact-relationship-management.github.io`) og brugerfladen i
Contact Relationship Management, som dette repositorium leverer den. Den opregner alt, som et af dem gemmer i din
browser, og hvorfor.

En organisation, der driver CRM-systemet, behandler personoplysninger om sine kontakter. Den behandling er omfattet af
organisationens egen privatlivserklæring, ikke af denne ([spec/regulatory.md](../../../spec/regulatory.md)).

## Hvad der gemmes, og hvorfor

Hvert element rummer et valg, som du har truffet: et tema, et sprog og en landestandard, en tekststørrelse, en
tidszone eller layoutet af en tabel. Intet af det rummer dit navn, din e-mailadresse, din IP-adresse eller nogen
identifikator, og intet af det bruges til at spore dig, måle dig eller vise dig reklamer. Hvert element findes, så den
næste side, du åbner, bruger det samme valg, og du ikke skal træffe det igen.

Der gemmes intet, før du træffer et valg. En besøgende, der ikke ændrer noget, har ikke noget gemt.

### På dette websted

| Navn | Slags | Hvad det rummer | Gemmes i |
| --- | --- | --- | --- |
| `lily-theme` | Cookie og `localStorage` | `system` ("Følg denne enhed") eller navnet på et tema, f.eks. `dark` | Cookie: 400 dage. `localStorage`: indtil du rydder det |
| `lily-text-size` | `localStorage` | Den tekststørrelse, du valgte | Indtil du rydder det |
| `lily-time-zone` | `localStorage` | Den tidszone, du valgte | Indtil du rydder det |

Temaet gemmes i en cookie og i `localStorage`, så det stadig er der, hvis din browser rydder det ene. Webstedets sider
er statiske filer, og intet program på webstedet læser cookien. Fordi det er en cookie, sender din browser den til
GitHub Pages, som hoster webstedet, ved hver forespørgsel til webstedet.

Webstedets sprog og landestandard er en del af sideadressen, f.eks. `/de-de/`, så der gemmes intet for dem.

### I CRM-brugerfladen

| Navn | Slags | Hvad det rummer | Gemmes i |
| --- | --- | --- | --- |
| `crm-theme` | Cookie | `system` ("Følg denne enhed") eller navnet på et tema | 400 dage |
| `mxi.crm.locale` | `localStorage` | Det sprog og den landestandard, du valgte, f.eks. `de-de` | Indtil du rydder det |
| `lily-text-size` | `localStorage` | Den tekststørrelse, du valgte | Indtil du rydder det |
| `crm-grid-<table>` | `localStorage` | Én tabels kolonnebredder, skjulte kolonner og sortering | Indtil du rydder det |
| `crm-grid-order-<table>` | `localStorage` | Én tabels kolonnerækkefølge | Indtil du rydder det |
| `__Host-mxi_session` | Cookie | En tilfældig identifikator for din indloggede session | Indtil du logger ud, eller sessionen udløber |

CRM-serveren læser cookien `crm-theme`, så den kan sende hver side i dit tema, og siden ikke først vises i et andet.
CRM-systemet læser den aldrig til noget andet formål.

Sessionscookien er ikke et valg. Den sættes, når du logger ind, browserens scripts kan ikke læse den (`HttpOnly`),
og uden den kan du ikke forblive logget ind.

## Hvem der modtager det

Disse elementer bliver i din browser. Intet af det sendes til tredjepart, bortset fra webstedets temacookie, som din
browser sender til webstedets vært som beskrevet ovenfor. Cookies er førsteparts (`SameSite=Lax`) og `Secure` på en
`https`-side.

## Sådan fjerner du det

Hvis du vil ændre et valg, skal du træffe det igen. Hvis du vil fjerne alt, skal du rydde dette websteds data i
browserens indstillinger: den næste side bruger så standardværdierne igen, det vil sige at følge enhedens lyse eller
mørke indstilling, standardtekststørrelsen og sidens standardsprog.
