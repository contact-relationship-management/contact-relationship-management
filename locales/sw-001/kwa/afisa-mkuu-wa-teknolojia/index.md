# Kwa Afisa Mkuu wa Teknolojia

> Muhtasari wa kihandisi, si ushauri wa kisheria, uthibitisho wala ukaguzi. Unaeleza
> kile ambacho hazina inaonyesha hadi 2026-10-11. Hakuna kilichomo hapa kinachodai idhini rasmi.

## Usanifu

- **Huduma:** Rust kwenye Axum na axa (sqlx 0.9), PostgreSQL 18, Redis kwa akiba,
  vihesabu vinavyoshirikiwa na usambazaji wa break-glass. `forbid(unsafe_code)`, clippy
  pedantic. Chanzo cha ukweli wa tabia ni maelezo ya kiufundi
  ([index](../../../../spec/index.md), [architecture](../../../../spec/architecture.md)).
- **UI:** SvelteKit 3 / Svelte 5, proksi ya backend-for-frontend ili tokeni isifike kamwe
  kwenye JavaScript ya kivinjari, CSP kali yenye hash.
- **Uthibitishaji:** PASETO v4.public huthibitishwa nje ya mtandao, Keycloak ya hiari; udhibiti wa
  ufikiaji unaotegemea sifa (vitendo vya kusoma/kuandika/kufuta/kuharibu, wajibu wa `mask`,
  ukaguzi wa ngazi ya rekodi) kutoka injini inayoshirikiwa.
- **Ukaguzi:** mnyororo wa hash wa SHA-256 unaoandikwa katika muamala ule ule na mabadiliko,
  kichocheo cha kuongeza tu, kinachoweza kuthibitishwa kwenye `GET /api/audits/verify`.
- **Ulinzi wa data:** usimbaji fiche wa sehemu wa AES-256-GCM na kuzungusha funguo kwenye
  safu wima mbili; funguo za ndani, hakuna KMS.

## Ushahidi wa ubora

Majaribio ya kitengo (185), majaribio ya hifadhidata (56 yakijumuisha utekelezaji, ufichaji, ukaguzi,
mgawanyo wa kurasa, akiba, idempotency, ukaguzi wa skima), majaribio ya kitengo ya sehemu ya mbele (106) na
Playwright (98) ikijumuisha uchanganuzi wa ufikivu wa axe. CI huendesha fmt, clippy, majaribio, ujenzi/lint wa sehemu ya mbele na
hatua ya mnyororo wa ugavi (`cargo audit`, `cargo deny`, `pnpm audit`, SBOM za CycloneDX)
kwenye GitHub Actions na Woodpecker. Amri moja huyarudia: `scripts/ci-check.sh`.

## Deni la kiufundi na mipaka ya uaminifu

- Mgawanyo wa kurasa hutumia kielekezi cha offset (safu zinaweza kurukwa au kurudiwa data ikibadilika
  katikati ya kupitia); hakuna jumla ya hesabu kwa makusudi.
- Funguo za idempotency ni hiari kwenye API (T52): mteja lazima atume
  `Idempotency-Key` kwenye uundaji ili kulindwa dhidi ya kujaribu upya kunakorudufu
  rekodi. UI iliyojumuishwa hutuma moja kwa kila uundaji.
- Skima za OpenAPI zimeandikwa kwa mkono. Zinashughulikia maombi, orodha, usomaji wa maelezo,
  dashibodi, mionekano ya uchanganuzi, ukaguzi, faragha na break-glass; mabadiliko ya hali
  na upsert zinazorudisha rekodi iliyosasishwa hazina aina. Jaribio hushindwa wakati
  njia na hati zinapotofautiana, na mengine hukagua majibu halisi, yale yaliyofichwa
  yakijumuishwa, dhidi ya skima
  ([gds-api-review](../../../../spec/compliance/gds-api-review.md)).
- Makosa ni hati za matatizo za RFC 9457 (T54) na vipengele vya awali vya `error` na
  `description` vimehifadhiwa; makosa kutoka tabaka nje ya kipanga njia (kikomo cha ukubwa,
  muda kuisha) hayabadilishwi. Sera ya matoleo na kuacha kutumika ni
  [api-lifecycle](../../../../spec/compliance/api-lifecycle.md); vichwa vya
  `Deprecation` na `Sunset` vimebainishwa lakini havijajengwa.
- Usimbaji fiche wa sehemu unashughulikia safu wima mbili; hakuna uunganishaji wa KMS.
- Hali ya kikomo cha kasi na akiba zinahitaji Redis ili kuwa sahihi kwa nakala nyingi.
- Mionekano ya uchanganuzi imefichwa kwa jukumu lakini huhifadhiwa kwenye akiba kwa kila tokeni ya mpigaji kwa sekunde 30.
- Uthibitishaji upya wa hatua ya juu kabla ya vitendo vya kuharibu unahitaji kazi kwa mtoa utambulisho.

## Mfuatano wa kiufundi unaopendekezwa

1. Funguo za sehemu zinazoungwa mkono na KMS na ufunikaji mpana zaidi wa sehemu (salio la T41).
2. Weka alama toleo la kwanza ili kuthibitisha utiaji saini bila ufunguo na uthibitishaji (T49), na
   mkaguzi akague rasimu ya viwango vya API.
3. Jenga vichwa vya `Deprecation` na `Sunset` pamoja na kuacha kutumika kwa kwanza.
4. Agiza jaribio huru (T42) dhidi ya ujenzi wa staging.
