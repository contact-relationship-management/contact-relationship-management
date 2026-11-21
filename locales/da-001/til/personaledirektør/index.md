# Til personaledirektøren

> Teknisk orientering, ikke juridisk rådgivning, certificering eller revision. Den angiver,
> hvad repositoriet viser pr. 2026-10-11. Intet her gør krav på akkreditering.

## Hvorfor mennesker har betydning for dette produkt

Adgang til personoplysninger afgøres af rolle. Softwaren håndhæver, hvad en persons token siger; **hvem der får hvilken
rolle, og hvornår den fjernes, er en personaleproces**, som dette repositorium ikke kan udføre for dig.

## Hvad softwaren giver dine processer

- **Rollebaseret synlighed:** nogle roller ser maskerede værdier for kontaktkanaler, dealbeløb og pengeoversigter;
  ledere, administratorer og servicetrin ser mere ([auth](../../../../spec/auth.md)).
- **Reviderede læsninger:** at læse personoplysninger efterlader et spor med personens navn. Medarbejdere bør have det at
  vide før idriftsættelsen.
- **Nødadgang:** en person kan tage en begrundet læseforhøjelse på op til 60 minutter; en anden person skal
  gennemgå den bagefter. Uddan begge sider.
- **Udlogning ved inaktivitet** efter 15 minutter i browseren.
- **Tilgængelighed:** en offentlig erklæring og automatiske kontroller; den manuelle revision (T48) kræver stadig
  brugere med handicap og testere af hjælpeteknologi.
- **Sprog:** brugerfladen er oversat til 8 landestandarder; oversættelserne er **ikke** gennemgået af
  modersmålstalende.

## Dit ansvar

| Område | Handling |
|---|---|
| Ansættelse, flytning, fratrædelse | Tildel og fjern roller gennem din identitetsudbyder; CRM-systemet afviser udløbne tokens, men administrerer ikke konti |
| Uddannelse | Databeskyttelse, samtykkeregler, brug og gennemgang af break-glass, tilgængelig praksis |
| Politikker | Acceptabel brug, meddelelse om overvågning (læsninger revideres), disciplinær vej ved misbrug af nødadgang |
| Kapacitet til gennemgang | Udpeg, hvem der gennemgår break-glass-tildelinger, og hvem der dækker, når de er væk |
| Rekruttering | Roller, programmet har brug for: kontaktperson for den uafhængige tester, klinisk sikkerhedsansvarlig hvis NHS er omfattet, databeskyttelsesansvarlig, tilgængelighedsansvarlig |
| Trivsel | Støtte til medarbejdere, der håndterer følsomme sager; CRM-systemet er ikke et system til sagsstyring, men kan rumme følsomme relationsdata |

## Åbne punkter, der kræver mennesker

Manuel tilgængelighedsrevision med brugere af hjælpeteknologi (T48), gennemgang af oversættelser ved modersmålstalende
og ejerskab af ovenstående pligter til gennemgang og alarmering. Se [tasks.md](../../../../spec/tasks.md).
