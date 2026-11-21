# Vej til produktion

> Contact Relationship Management er ikke klar til produktion. Denne side siger, hvad produktion betyder her, og hvor
> langt hver del er nået.

Softwaren kører på syntetiske data. E-mail er simuleret, medmindre en rigtig mailtilstand er konfigureret. Læg ikke
rigtige personoplysninger i den.

## Hvad produktion betyder

En produktionsinstallation rummer rigtige menneskers data og sender rigtige beskeder til dem. For dette projekt kræver
det alt det følgende:

- **Rigtig indlogning.** Hver forespørgsel autentificeres, og adgangsreglerne svarer til roller i den organisation, der
  tager systemet i brug.
- **Lovlig udsendelse.** Hver besked har registreret samtykke og et lovligt grundlag efter lovgivningen på alle de
  steder, den sendes til, og går gennem en rigtig e-mailudbyder, der håndterer afviste mails, klager og afmeldinger.
- **Uafhængig sikring.** Folk uden for projektet har testet sikkerheden og tilgængeligheden, og fundene er rettet eller
  accepteret.
- **Drift.** Sikkerhedskopier, der er gendannet, overvågning, en hændelsesproces og nogen i vagt.
- **Bevis for stabilitet.** Lange kørsler under realistisk belastning på hardware som installationens, uden fejl eller
  nedsat hastighed.

Produktionsparathed afgøres for hver installation. En driftsansvarlig kan lukke nogle porte kun for sin egen
organisation og sine egne modtagere. Den tekniske vurdering bag denne side er [spec/plan.md](../../../spec/plan.md). De
åbne porte står i [spec/tasks.md](../../../spec/tasks.md).

## Parathed

Hver procent er et skøn lavet den 2026-11-13 ud fra opgavelisten, ikke en måling.

- **Autentificering og adgangskontrol** (70%): Indlogning med PASETO-tokens eller Keycloak/OIDC, attributbaserede
  adgangsregler med kontrol på postniveau og en produktionstilstand, der nægter at starte uden autentificering, er
  bygget. Ingen rigtig installation har slået dem til med egne attributter, og håndhævelsesmatrixen dækker endnu ikke
  alle handlere (CRM-G1).
- **E-mailudsendelse** (40%): En simuleret afsender er standard, og en SMTP-afsender findes. Der er ingen adapter til en
  e-mailudbyder med webhooks for afviste mails og klager og ingen spærreliste (CRM-T39).
- **Databeskyttelse og lov om e-marketing** (15%): Samtykkeregistre, indsigt, sletning og opbevaring er bygget. Ingen
  regions lovgivning er læst til dette projekt, og gennemgangen af den rigtige udsendelsessti er åben (CRM-G2,
  [regulatory](../../../spec/regulatory.md)).
- **Uafhængig sikkerhedssikring** (10%): Et omfang for en penetrationstest er skrevet. Der er ikke udført nogen
  penetrationstest, formel risikoanalyse eller akkreditering (CRM-T42). AI-sikkerhedsrevisionerne er ikke uafhængig
  sikring.
- **Tilgængelighed** (50%): Automatiske axe-core-kontroller mod WCAG 2.2 A og AA består på hver siderute. Der er ikke
  udført nogen manuel revision med kun tastatur, skærmlæsere eller 400 % zoom (CRM-T48).
- **Oversættelser** (30%): Brugerfladen findes på otte sprog. Tekster på andre sprog end engelsk er maskinskrevne, og
  ingen sprogbruger har gennemgået dem.
- **Drift** (30%): En driftshåndbog dækker sikkerhedskopiering og gendannelse, nøgleadministration, kryptering i hvile,
  hændelsesrespons og opbevaring af logfiler. Ingen gendannelse er testet, og håndbogens tjekliste for idriftsættelse er
  åben.
- **Ydeevne og stabilitet** (20%): Belastningstest og en langvarig stabilitetstest findes. Stabilitetstesten har kørt i
  to minutter mod demostakken; der findes ingen lang kørsel og ingen basislinje på hardware som i produktion.
- **Udgivelsens integritet** (85%): Udgivelsesbilleder er til flere arkitekturer, signeret og attesteret, og deres
  signaturer kan verificeres. Actions og billeder er fastlåst med digest, og historikken scannes for hemmeligheder.
  Amd64-halvdelen af et hentet udgivelsesbillede er ikke kontrolleret, og der er endnu ikke en anden kodevært (CRM-T192).
- **Kontinuerlig integration** (100%): Ét script kører hver kontrol på GitHub Actions og Woodpecker. Begge
  Rust-kodebaser og brugerfladen har linjegulv på 95 %; porte for browser, lint, forsyningskæde, mutation af ændrede
  linjer og ugentlig fuld mutation er på plads (CRM-T94).
- **Demobeskyttelse** (100%): Hver besøgende på rundvisningen får en separat kopi af demodata, begrænset til 200
  kopier. Rundvisningen er slået fra som standard; sæt `CONTACT_RELATIONSHIP_MANAGEMENT_DEMO_TOUR=on` for at tilbyde den
  (CRM-T92).

## Hvordan denne side vedligeholdes

Når en port lukker, skal du ændre skønnet og dets bevis i samme commit som arbejdet og linke opgaven.
