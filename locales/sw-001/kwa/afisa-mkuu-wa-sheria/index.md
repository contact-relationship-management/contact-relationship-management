# Kwa Afisa Mkuu wa Sheria

> Muhtasari wa kihandisi, si ushauri wa kisheria, uthibitisho wala ukaguzi. Unaeleza
> kile ambacho hazina inaonyesha hadi 2026-10-11. Hakuna kilichomo hapa kinachodai idhini rasmi.

## Msimamo wa kisheria kwa ufupi

Hazina **haijawa tayari kwa uzalishaji** na inashikilia data bandia. Haidai
uzingatiaji wa utaratibu wowote. Inatekeleza vipengele vya kiufundi vinavyosaidia
kadhaa, na inaorodhesha kile msambazaji lazima aongeze ([regulatory](../../../../spec/regulatory.md)).

## Mifumo ya sheria inayoguswa

| Mfumo | Kile programu inachosaidia | Kile lazima utoe |
|---|---|---|
| GDPR ya Uingereza/EU | Historia ya idhini, usafirishaji wa ufikiaji wa data, ufutaji (hukataliwa wakati wajibu wazi upo), ufagiaji wa uhifadhi, upunguzaji wa data, ukaguzi wa usomaji | Msingi halali kwa kila madhumuni, DPIA ([kiolezo](../../../../spec/compliance/dpia-template.md)), rekodi za uchakataji, ratiba ya uhifadhi, majukumu ya mdhibiti/mchakataji |
| PECR / e-privacy | Utumaji unaotegemea idhini, kujiondoa mara moja | Utambulisho wa mtumaji na maamuzi ya soft-opt-in kwa mtumaji halisi |
| HIPAA (ikiwa data ya afya ya Marekani) | Vipengele vya udhibiti wa ukaguzi, udhibiti wa ufikiaji, uadilifu, usalama wa usafirishaji | Makubaliano ya mshirika wa kibiashara, uchambuzi wa hatari, sera, mafunzo ya wafanyakazi. Programu haikufanyi uzingatie |
| NHS DTAC / DCB0129/0160 | Rasimu za ushahidi ([dtac-supporting](../../../../spec/compliance/dtac-supporting.md)) | Afisa wa usalama wa kitabibu aliyetajwa, kumbukumbu ya hatari, vyeti. Tathmini inasema DTAC huenda haitumiki |
| Ufikivu (WCAG 2.2 AA, sekta ya umma) | Ukaguzi wa kiotomatiki na taarifa ya uaminifu | Ukaguzi wa mikono (T48) kabla ya dai lolote la uzingatiaji |
| Usalama wa serikali (NIST 800-53, ISO 27001, Cyber Essentials) | Ramani ya udhibiti ([control-mapping](../../../../spec/compliance/control-mapping.md)) | Uthibitisho na uhakikisho huru |

## Mikataba utakayohitaji ambayo bado haipo

1. Makubaliano ya uchakataji wa data na masharti ya uhamishaji wa kimataifa na mtoa huduma wa barua pepe (T39).
2. Hati ya kazi na masharti ya usiri na kampuni ya jaribio la kuvamia (T42).
3. Masharti ya upangishaji na wachakataji wasaidizi kwa PostgreSQL, Redis na kumbukumbu.
4. Leseni: tazama [LICENSE.md](../../../../LICENSE.md); hazina pia ina
   taarifa za usalama, utawala na matumizi ya AI.

## Mambo mahususi ya kisheria ya kuangalia

- **Ufutaji** hufanya mwasiliani asitambulike na hukataliwa wakati mkataba wazi, tiketi
  au usajili hai wa kukuza wateja upo; mnyororo wa ukaguzi huhifadhi rekodi za vitendo,
  kwa hivyo amua katika DPIA yako jinsi hilo linavyoingiliana na maombi ya ufutaji.
- Usomaji wa data iliyofichwa kwa **break-glass** ni halali tu kwa msingi wako uliorekodiwa;
  sehemu ya sababu ni ushahidi, si mamlaka.
- **Utoaji alama** wa wateja watarajiwa: ikiwa utaathiri maamuzi muhimu kuhusu
  watu, kanuni za maamuzi ya kiotomatiki zinatumika ([regulatory](../../../../spec/regulatory.md)).
- **Ahadi za API:** [api-lifecycle](../../../../spec/compliance/api-lifecycle.md)
  inaweka kipindi cha notisi cha siku 180 kabla mabadiliko ya API hayajaondolewa. Hilo ni
  chaguo la mtunzaji; ikiwa wateja wataitegemea, amua kama linapaswa kuwa
  kwenye mkataba.
- **Madai ya hadharani:** usidai uzingatiaji, uthibitisho wala idhini.
