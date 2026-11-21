# Notisi ya faragha: kinachohifadhiwa kwenye kivinjari chako

Notisi hii inahusu mambo mawili: tovuti hii (`contact-relationship-management.github.io`), na kiolesura cha mtumiaji cha
Contact Relationship Management kama hazina hii inavyokisafirisha. Inaorodhesha kila kitu ambacho kimojawapo huhifadhi kwenye
kivinjari chako, na kwa nini.

Shirika linaloendesha CRM huchakata data binafsi kuhusu waasiliani wake. Uchakataji huo unashughulikiwa na notisi ya faragha
ya shirika hilo lenyewe, si hii ([spec/regulatory.md](../../../spec/regulatory.md)).

## Kinachohifadhiwa, na kwa nini

Kila kipengele kinashikilia chaguo ulilofanya: mandhari, lugha na eneo, ukubwa wa maandishi, saa za eneo, au mpangilio wa
jedwali. Hakuna kati yake kinachoshikilia jina lako, anwani yako ya barua pepe, anwani yako ya IP wala kitambulisho chochote, na
hakuna kinachotumiwa kukufuatilia, kukupima au kukuonyesha matangazo. Kila kimoja kipo ili ukurasa unaofuata utakaofungua utumie
chaguo lilelile, na usilazimike kulifanya tena.

Hakuna kinachohifadhiwa hadi ufanye chaguo. Mgeni asiyebadilisha chochote hana kitu kilichohifadhiwa.

### Kwenye tovuti hii

| Jina | Aina | Kinachoshikiliwa | Muda wa kuhifadhiwa |
| --- | --- | --- | --- |
| `lily-theme` | Kuki na `localStorage` | `system` ("Fuata kifaa hiki") au jina la mandhari, kama vile `dark` | Kuki: siku 400. `localStorage`: hadi uifute |
| `lily-text-size` | `localStorage` | Ukubwa wa maandishi uliouchagua | Hadi uifute |
| `lily-time-zone` | `localStorage` | Saa za eneo ulizochagua | Hadi uifute |

Mandhari huhifadhiwa kwenye kuki na kwenye `localStorage`, ili ibaki hata kivinjari chako kikifuta mojawapo. Kurasa za tovuti
ni faili tuli, na hakuna programu kwenye tovuti inayosoma kuki hiyo. Kwa kuwa ni kuki, kivinjari chako huituma kwa GitHub Pages,
inayopangisha tovuti, pamoja na kila ombi kwa tovuti.

Lugha na eneo la tovuti ni sehemu ya anwani ya ukurasa, kama vile `/de-de/`, kwa hivyo hakuna kinachohifadhiwa kwa ajili yake.

### Kwenye kiolesura cha mtumiaji cha CRM

| Jina | Aina | Kinachoshikiliwa | Muda wa kuhifadhiwa |
| --- | --- | --- | --- |
| `crm-theme` | Kuki | `system` ("Fuata kifaa hiki") au jina la mandhari | Siku 400 |
| `mxi.crm.locale` | `localStorage` | Lugha na eneo ulilochagua, kama vile `de-de` | Hadi uifute |
| `lily-text-size` | `localStorage` | Ukubwa wa maandishi uliouchagua | Hadi uifute |
| `crm-grid-<table>` | `localStorage` | Upana wa safu wima, safu wima zilizofichwa na mpangilio wa kupanga wa jedwali moja | Hadi uifute |
| `crm-grid-order-<table>` | `localStorage` | Mpangilio wa safu wima za jedwali moja | Hadi uifute |
| `__Host-mxi_session` | Kuki | Kitambulisho cha nasibu cha kipindi chako cha kuingia | Hadi utoke au kipindi kiishe |

Seva ya CRM husoma kuki ya `crm-theme`, ili iweze kutuma kila ukurasa katika mandhari yako na ukurasa usionekane kwanza katika
mandhari nyingine. CRM haisomi kuki hiyo kwa kusudi lingine lolote.

Kuki ya kipindi si chaguo. Huwekwa unapoingia, hati za kivinjari haziwezi kuisoma (`HttpOnly`), na bila hiyo huwezi kuendelea
kuwa umeingia.

## Nani anapokea

Vipengele hivi hubaki kwenye kivinjari chako. Hakuna kilichomo kinachotumwa kwa mtu wa tatu, isipokuwa kuki ya mandhari ya tovuti,
ambayo kivinjari chako huituma kwa mpangishi wa tovuti kama ilivyoelezwa hapo juu. Kuki ni za mhusika wa kwanza
(`SameSite=Lax`), na `Secure` kwenye ukurasa wa `https`.

## Jinsi ya kuviondoa

Ili kubadilisha chaguo, lifanye tena. Ili kuondoa kila kitu, futa data ya tovuti hii katika mipangilio ya kivinjari chako:
ukurasa unaofuata utatumia chaguo-msingi tena, yaani kufuata mpangilio wa mwanga au giza wa kifaa chako, ukubwa wa kawaida wa
maandishi na lugha-msingi ya ukurasa.
