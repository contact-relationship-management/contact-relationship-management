# Integritetsmeddelande: vad som sparas i din webbläsare

Det här meddelandet gäller två saker: den här webbplatsen (`contact-relationship-management.github.io`) och
användargränssnittet för Contact Relationship Management så som det här arkivet levererar det. Det förtecknar allt som
någondera sparar i din webbläsare, och varför.

En organisation som driver CRM-systemet behandlar personuppgifter om sina kontakter. Den behandlingen omfattas av
organisationens eget integritetsmeddelande, inte det här ([spec/regulatory.md](../../../spec/regulatory.md)).

## Vad som sparas, och varför

Varje post innehåller ett val som du har gjort: ett tema, ett språk och en region, en textstorlek, en tidszon eller
layouten på en tabell. Ingen av dem innehåller ditt namn, din e-postadress, din IP-adress eller någon identifierare, och
ingen används för att spåra dig, mäta dig eller visa reklam för dig. Var och en finns för att nästa sida du öppnar ska
använda samma val, så att du inte behöver göra det igen.

Ingenting sparas förrän du gör ett val. En besökare som inte ändrar något har ingenting sparat.

### På den här webbplatsen

| Namn | Typ | Vad den innehåller | Sparas i |
| --- | --- | --- | --- |
| `lily-theme` | Kaka och `localStorage` | `system` ("Följ den här enheten") eller namnet på ett tema, till exempel `dark` | Kaka: 400 dagar. `localStorage`: tills du rensar den |
| `lily-text-size` | `localStorage` | Den textstorlek du valde | Tills du rensar den |
| `lily-time-zone` | `localStorage` | Den tidszon du valde | Tills du rensar den |

Temat sparas i en kaka och i `localStorage`, så att det finns kvar om webbläsaren rensar den ena. Webbplatsens sidor är
statiska filer, och inget program på webbplatsen läser kakan. Eftersom det är en kaka skickar webbläsaren den till
GitHub Pages, som är webbplatsens värd, med varje begäran till webbplatsen.

Webbplatsens språk och region ingår i sidadressen, till exempel `/de-de/`, så ingenting sparas för dem.

### I CRM-användargränssnittet

| Namn | Typ | Vad den innehåller | Sparas i |
| --- | --- | --- | --- |
| `crm-theme` | Kaka | `system` ("Följ den här enheten") eller namnet på ett tema | 400 dagar |
| `mxi.crm.locale` | `localStorage` | Det språk och den region du valde, till exempel `de-de` | Tills du rensar den |
| `lily-text-size` | `localStorage` | Den textstorlek du valde | Tills du rensar den |
| `crm-grid-<table>` | `localStorage` | En tabells kolumnbredder, dolda kolumner och sorteringsordning | Tills du rensar den |
| `crm-grid-order-<table>` | `localStorage` | En tabells kolumnordning | Tills du rensar den |
| `__Host-mxi_session` | Kaka | En slumpmässig identifierare för din inloggade session | Tills du loggar ut eller sessionen upphör |

CRM-servern läser kakan `crm-theme`, så att den kan skicka varje sida i ditt tema och sidan inte först visas i ett annat.
CRM-systemet läser den aldrig för något annat ändamål.

Sessionskakan är inte ett val. Den sätts när du loggar in, webbläsarens skript kan inte läsa den (`HttpOnly`), och utan
den kan du inte förbli inloggad.

## Vem som tar emot den

De här posterna stannar i din webbläsare. Ingenting i dem skickas till tredje part, förutom webbplatsens temakaka, som
din webbläsare skickar till webbplatsens värd enligt beskrivningen ovan. Kakorna är förstapartskakor (`SameSite=Lax`) och
`Secure` på en `https`-sida.

## Så tar du bort dem

Gör valet igen för att ändra det. Rensa den här webbplatsens data i webbläsarens inställningar för att ta bort allt:
nästa sida använder då standardvärdena igen, vilket innebär att den följer enhetens ljusa eller mörka inställning,
standardtextstorleken och sidans standardspråk.
