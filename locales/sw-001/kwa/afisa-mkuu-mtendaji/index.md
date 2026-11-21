# Kwa Afisa Mkuu Mtendaji

> Muhtasari wa kihandisi, si ushauri wa kisheria, uthibitisho wala ukaguzi. Unaeleza
> kile ambacho hazina inaonyesha hadi 2026-10-11. Hakuna kilichomo hapa kinachodai idhini rasmi.

## Uamuzi katika aya moja

Hii ni CRM ya kiwango cha onyesho yenye msingi imara usio wa kawaida wa usalama na faragha.
**Haijawa tayari** kushikilia data halisi ya wagonjwa au wananchi, na hazina yenyewe inasema hivyo
([plan.md](../../../../spec/plan.md)). Kuipitisha ni uamuzi wa kufadhili kazi ya nje iliyobaki, si
kuiwasha.

## Kilichopo

- CRM inayofanya kazi katika mauzo, masoko, huduma kwa wateja na uchanganuzi, yenye API ya Rust
  na kiolesura cha wavuti, iliyojengwa kwa maelezo ya kiufundi yaliyoandikwa.
- Masoko yanayotanguliza idhini, udhibiti wa ufikiaji unaoficha thamani nyeti kwa majukumu
  yasiyopaswa kuziona, njia ya ukaguzi inayoonyesha uchezaji, ufikiaji wa data na ufutaji kwa maombi
  ya faragha, na ufikiaji wa dharura ("break-glass") wenye sababu, muda mfupi na unaopitiwa.
- Vipengele 54 kati ya 58 vya kazi vinavyofuatiliwa vimekamilika ([tasks.md](../../../../spec/tasks.md)).

## Kinachozuia matumizi halisi

| Kizuizi | Kwa nini kinajalisha | Nani anaweza kukirekebisha |
|---|---|---|
| Hakuna utumaji halisi wa barua pepe (T39) | Masoko yanaigizwa; mtoa huduma halisi anahitaji mikataba na makubaliano ya uchakataji wa data | Wewe unachagua muuzaji; idara ya sheria inatia saini |
| Hakuna jaribio huru la usalama (T42) | Hospitali na wanunuzi wa serikali kwa kawaida wanalihitaji; majaribio yetu wenyewe si huru | Kampuni ya nje |
| Hakuna ukaguzi wa mikono wa ufikivu (T48) | Taasisi za umma lazima zichapishe taarifa ya ufikivu wanayoweza kuisimamia | Watu wenye teknolojia saidizi |
| Utiaji saini wa toleo umejengwa lakini haujaendeshwa kwenye toleo halisi, na hakuna mkaguzi aliyekagua rasimu ya viwango vya API (T49) | Orodha za ukaguzi za manunuzi huuliza vyote viwili | Uhandisi (kuweka alama ya toleo la kwanza); mkaguzi wa nje |

## Maamuzi ambayo ni yako

1. **Soko gani kwanza.** Tathmini ya NHS Digital Technology Assessment Criteria
   ([docs](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md))
   inahitimisha kwamba bidhaa huenda iko nje ya wigo wa DTAC na ingeshindwa katika sehemu
   zinazotumika kwa kukosa ushahidi wa kishirika (afisa wa usalama wa kitabibu aliyetajwa,
   vyeti). Chagua njia kabla ya kutumia gharama juu yake.
2. **Kama utafadhili uhakikisho huru** (T42, T48). Bila huo, dai la kweli ni
   "imejengwa kwa kiwango cha juu, haijajaribiwa na watu wa nje".
3. **Nani anamiliki hatari** pindi data halisi inaposhikiliwa. Tazama muhtasari wa [CRO](../afisa-mkuu-wa-hatari/index.md)
   na [CLO](../afisa-mkuu-wa-sheria/index.md).

## Cha kutosema nje

Usidai uzingatiaji, uthibitisho, "inazingatia HIPAA" wala "imeidhinishwa na NHS".
Kifurushi cha ushahidi ([compliance](../../../../spec/compliance/index.md)) kinaunga mkono kauli finyu zaidi
na ya kweli: udhibiti umepangwa, ule uliotekelezwa umejaribiwa, mapengo yametajwa.
