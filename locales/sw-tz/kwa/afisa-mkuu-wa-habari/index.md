# Kwa Afisa Mkuu wa Habari

> Muhtasari wa kihandisi, si ushauri wa kisheria, uthibitisho wala ukaguzi. Unaeleza
> kile ambacho hazina inaonyesha hadi 2026-10-11. Hakuna kilichomo hapa kinachodai idhini rasmi.

## Mtazamo wa habari na data

CRM hii inashikilia **data ya mahusiano**: waasiliani na akaunti kama vifuniko vya vitambulisho
vilivyoshikiliwa katika mifumo mingine, shughuli, mikataba, tiketi, historia ya idhini
na kumbukumbu ya ukaguzi. Taarifa za idadi ya watu zinabaki kwa makusudi katika rejista za utambulisho
([domain-model](../../../../spec/domain-model.md)). Ikiwa shirika lako linarekodi wagonjwa au
wananchi kama waasiliani, uainishaji wa data hiyo ni **wako** kuamua;
mpango umeandikwa kwa usomaji ulio mkali zaidi ([plan](../../../../spec/plan.md)).

## Data inakwenda wapi

| Hifadhi | Inashikilia | Ulinzi katika hazina |
|---|---|---|
| PostgreSQL | Rekodi zote, mnyororo wa ukaguzi, ruhusa za break-glass | Usimbaji fiche wa sehemu kwenye safu wima mbili; kichocheo cha ukaguzi cha kuongeza tu; usimbaji fiche wakati wa kuhifadhi ni wa msambazaji |
| Redis | Akiba ya majibu ya uchanganuzi (sekunde 30), vihesabu vya kikomo cha kasi, funguo za ruhusa za break-glass | Funguo kwa kila mpigaji; **inahitaji TLS, nenosiri na isifichuliwe hadharani** |
| Kumbukumbu/vipimo | Metadata ya maombi, vipimo vya Prometheus | Uhifadhi ni wa msambazaji |
| Mtoa huduma wa barua pepe (baadaye) | Maudhui ya ujumbe na wapokeaji | Haijajengwa (T39) |

## Uunganishaji na utambulisho

- Kuingia hutoka kwa huduma ya utambulisho inayotoa tokeni za PASETO, au Keycloak.
  Kwa hivyo wanaojiunga/wanaohama/wanaoondoka, MFA na sera ya vipindi ni **vya mtoa utambulisho
  wako**; CRM hutekeleza kile tokeni inachosema na hukataa zilizoisha muda.
- API imeandikwa katika OpenAPI ([gds-api-review](../../../../spec/compliance/gds-api-review.md)),
  ina matoleo kwa kichwa, imepangwa kwa kurasa kwa vichwa vya `Link`, na hurudisha makosa kama
  hati za matatizo za RFC 9457. Kanuni za mabadiliko na kuacha kutumika ziko katika
  [api-lifecycle](../../../../spec/compliance/api-lifecycle.md).
- SBOM (CycloneDX) hutengenezwa kwa huduma na UI katika CI; mtiririko wa toleo
  huzitia saini pamoja na taswira.

## Wajibu wa utawala wa habari unaobaki kwako

Ushahidi wa nakala rudufu na urejeshaji, ratiba za uhifadhi kwa kila jedwali, kutia nanga kichwa cha mnyororo wa
ukaguzi nje ya hifadhidata, uhifadhi wa funguo za sehemu kwenye KMS, makubaliano ya uchakataji wa data kwa
msambazaji yeyote, na tathmini ya athari ya ulinzi wa data
([dpia-template](../../../../spec/compliance/dpia-template.md)). Ramani ya udhibiti
([control-mapping](../../../../spec/compliance/control-mapping.md)) inaweka alama kila udhibiti kama
Implemented, Deployer, au Gap.
