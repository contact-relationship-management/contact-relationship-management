# Kwa Afisa Mkuu wa Usalama

> Muhtasari wa kihandisi, si ushauri wa kisheria, uthibitisho wala ukaguzi. Unaeleza
> kile ambacho hazina inaonyesha hadi 2026-10-11. Hakuna kilichomo hapa kinachodai idhini rasmi.

## Msimamo wa usalama katika aya moja

Huduma **hushindwa kwa usalama**, hudhibiti ufikiaji kwa jukumu na kwa rekodi, huficha
thamani nyeti kwa majukumu yasiyopaswa kuziona, na huweka njia ya ukaguzi inayoonyesha
uchezaji. Udhibiti huo umetekelezwa na kujaribiwa na waandishi. **Haujajaribiwa
na mtu yeyote huru** (T42), kwa hivyo chukulia kila kauli iliyo hapa chini
kama "imejengwa na kujaribiwa na sisi wenyewe", si "imethibitishwa".

## Udhibiti na pa kuukagua

| Eneo | Kilichopo | Mahali pa kuangalia |
|---|---|---|
| Uthibitishaji | PASETO v4.public huthibitishwa nje ya mtandao, Keycloak ya hiari; uzalishaji hukataa kuanza ikiwa uthibitishaji umezimwa | [auth](../../../../spec/auth.md) |
| Idhini | Inayotegemea sifa: kusoma, kuandika, kufuta, kuharibu; wajibu wa `mask`; ukaguzi wa ngazi ya rekodi; ukaguzi, uhifadhi na mapitio ya break-glass kwa msimamizi pekee | [auth](../../../../spec/auth.md), jaribio la matriki ya wahusika |
| Kufichuliwa kwa data | Ufichaji kwenye rekodi, jumla na uchanganuzi; uvujaji wa maelezo ya akaunti ulipatikana na kurekebishwa (T47) | [tasks](../../../../spec/tasks.md) |
| Ufikiaji wa dharura | Sababu iliyoandikwa, kikomo cha dakika 60, usomaji tu, ukaguzi wa lazima wa mtu wa pili, hukaguliwa, hushirikiwa kupitia Redis | kazi za break-glass T44 |
| Ukaguzi | Mnyororo wa hash wa SHA-256 katika muamala ule ule, kichocheo cha kuongeza tu, sehemu ya mwisho ya uthibitishaji; usomaji hukaguliwa | [audit](../../../../spec/audit.md) |
| Upinzani dhidi ya matumizi mabaya | Kikomo cha kasi (600 kwa dakika) na kufungiwa (kushindwa mara 10 ndani ya dakika 5), vinashirikiwa kupitia Redis; vipimo vya afya vimeondolewa | T43 |
| Kivinjari na usafirishaji | CSP kali yenye hash, kukataa fremu, `nosniff`, sera ya referrer, CORS iliyofungwa, vichwa salama; hakuna tokeni kwenye JavaScript ya kivinjari (BFF) | UI `hooks.server.ts` |
| Ulinzi wa data | AES-256-GCM kwenye safu wima mbili na kuzungusha; funguo za ndani, hakuna KMS | T41 |
| Mnyororo wa ugavi | `cargo audit`, `cargo deny`, `pnpm audit`, SBOM, taswira zilizoimarishwa zisizo za root zilizobanwa kwa digest, masasisho ya kila wiki ya Dependabot, mtiririko wa toleo unaotia saini taswira na SBOM bila ufunguo | [control-mapping](../../../../spec/compliance/control-mapping.md), [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Makosa | Kila kosa ni hati moja ya tatizo ya RFC 9457; `401` hutaja mpango unaokubaliwa | [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) |
| Usalama wa kumbukumbu | `forbid(unsafe_code)`, clippy pedantic | `src/lib.rs` |

## Udhaifu unaojulikana (umetajwa, haujafichwa)

- **Hakuna jaribio huru la kuvamia wala zoezi la kuunda muundo wa vitisho** (T42).
  Muundo wa vitisho katika [dtac-supporting](../../../../spec/compliance/dtac-supporting.md)
  ni muhtasari, si ulio na muundo rasmi.
- **Utiaji saini wa toleo haujaendeshwa kwenye toleo halisi** (T49). Mtiririko na
  amri za uthibitishaji zipo; hadi alama ya toleo itumwe hazijajaribiwa. Actions
  zimebanwa kwa hash ya commit.
- **Usimbaji fiche wa sehemu** unashughulikia safu wima mbili kwa funguo za ndani, hakuna KMS.
- **Uthibitishaji upya wa hatua ya juu** kabla ya vitendo vya kuharibu haujajengwa.
- **Redis inashikilia data binafsi** (uchanganuzi ulio kwenye akiba, funguo za ruhusa) na lazima ilindwe
  kama hifadhidata: TLS, nenosiri, isifichuliwe hadharani.
- **Break-glass** haiwezi kufutwa, inaisha muda tu, na hakuna tahadhari iliyounganishwa.
- **Kikomo cha kasi na kufungiwa** hurudi kwenye hesabu kwa kila nakala Redis ikishindwa; kapu
  la wasiothibitishwa ni kwa kila anwani, na nyuma ya proksi isiyoorodheshwa kwenye
  `TRUSTED_PROXIES` hiyo ni ya proksi, kwa hivyo mshambuliaji anaweza kuwafungia nje wapigaji
  wengine wasiothibitishwa. Mpigaji mwenye tokeni halali huhesabiwa kwa mhusika wake.
- **Upakiaji upya wa sera** huonekana katika uchanganuzi ulio kwenye akiba hadi sekunde 30.
- **Kutoka kiotomatiki kwa kutofanya kazi** hufanyika upande wa mteja.
- **Funguo za idempotency** ni hiari kwenye API (T52): mteja asiyetuma
  `Idempotency-Key` hajalindwa dhidi ya uundaji uliorudiwa unaorudufu rekodi.
  UI iliyojumuishwa hutuma moja.

## Kile msambazaji lazima afanye

1. Weka uthibitishaji umewashwa; toa funguo za uzalishaji na `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL`.
2. Toa tahadhari kwa `break_glass_granted` na kagua ruhusa haraka.
3. Tia nanga kichwa cha mnyororo wa ukaguzi nje ya hifadhidata na kithibitishe
   (`GET /api/audits/verify`) kwa ratiba.
4. Linda PostgreSQL na Redis (mtandao, TLS, vitambulisho, nakala rudufu, usimbaji fiche
   wakati wa kuhifadhi); zungusha funguo za sehemu kwa kazi iliyotolewa.
5. Unganisha masasisho ya Dependabot; endesha tena hatua ya mnyororo wa ugavi; thibitisha toleo
   lililotiwa saini kabla ya kulisambaza.
6. Fuata mwongozo wa mwitikio wa matukio
   ([operations-runbook](../../../../spec/compliance/operations-runbook.md)).

## Kushughulikia udhaifu

Ripoti hupitia [SECURITY.md](../../../../SECURITY.md). Agiza jaribio huru
(T42) dhidi ya ujenzi wa staging kabla ya data yoyote halisi, na jaribu upya baada ya mabadiliko makubwa.

## Hatua za usalama zinazopendekezwa, kwa mpangilio

1. Bainisha wigo na uagize jaribio huru la kuvamia (T42).
2. Tuma alama ya toleo la kwanza ili kuthibitisha utiaji saini na uthibitishaji (T49).
3. Unganisha tahadhari kwa break-glass na kwa kushindwa kwa uthibitishaji wa mnyororo wa ukaguzi.
4. Weka Redis nyuma ya TLS na vitambulisho katika kila mazingira.
5. Funguo za sehemu zinazoungwa mkono na KMS na ufunikaji mpana zaidi wa sehemu (salio la T41).

Tazama pia [muhtasari wa CTO](../afisa-mkuu-wa-teknolojia/index.md) na
[muhtasari wa CRO](../afisa-mkuu-wa-hatari/index.md).
