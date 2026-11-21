# Til driftsdirektøren

> Teknisk orientering, ikke juridisk rådgivning, certificering eller revision. Den angiver,
> hvad repositoriet viser pr. 2026-10-11. Intet her gør krav på akkreditering.

## Hvad det kræver at køre dette

Tjenesten er én Rust-proces pr. replika plus PostgreSQL 18 og Redis; brugerfladen er en Node-proces. Begge leveres som
hærdede containerbilleder uden root, med basisbilleder fastlåst med digest. Driftshåndbogen er
[operations-runbook.md](../../../../spec/compliance/operations-runbook.md).

## Driftsforhold at planlægge efter

- **Fejler sikkert ved start.** I produktion nægter tjenesten at starte, hvis autentificering er slået fra, hvis
  revisionskæden er ubrugelig, eller hvis krævede nøgler og `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL` mangler. En
  forkert konfigureret installation stopper i stedet for at køre åben.
- **Sundhed og målinger.** `/_health/live`, `/_health/ready` og `/_health/startup` hastighedsbegrænses aldrig, så en
  spærring ikke kan genstarte en sund pod. Målinger findes på `/metrics.prom`.
- **Grænser mod misbrug** er slået til i produktion: 600 forespørgsler i minuttet pr. klient og en spærring efter 10
  mislykkede indloggeringer på 5 minutter. De deles mellem replikaer gennem Redis; hvis Redis fejler, falder de tilbage
  til tællinger pr. replika.
- **Nødadgang** (break-glass): enhver indlogget bruger kan tage en begrundet læseforhøjelse i op til 60 minutter. En
  anden skal gennemgå hver tildeling. **Proces, du selv skal køre:** giv alarm ved revisionsrækker med
  `break_glass_granted` og gennemgå antallet af ugennemgåede (administrationssiden `/break-glass/review`).
- **Fejl** har én form (RFC 9457 `application/problem+json`, hvor `error` og `description` bevares), så
  logfortolkning og alarmregler kan stole på `status` og `type`.
- **Vækst i revision.** Læsninger revideres, så revisionstabellen vokser med trafikken. Dimensionér opbevaringen til
  den. En klient, der poller, får den til at vokse hurtigt.

## Åbent driftsarbejde

| Punkt | Tilstand |
|---|---|
| Øvelser i sikkerhedskopiering og gendannelse, prøve på nøglerotation | Skrevet i håndbogen; **du** skal køre og dokumentere dem |
| Forankring af revisionskædens hoved uden for databasen | Din pligt; API'en rapporterer hovedets hash |
| Opdatering af fastlåste billeddigests | Dependabot åbner ugentlige pull requests; **du** skal gennemgå og flette dem, ellers bliver fastlåsningerne forældede |
| Verificering af en signeret udgivelse før udrulning | Kommandoerne står i håndbogen; udrul med digest og afvis et billede, der ikke kan verificeres. Utestet, indtil en første mærket udgivelse findes |
| Rigtig e-mailudbyder (T39) | Åben; ændrer billedet for hændelser og leverandører |
| Redis rummer indsigtsdata | Beskyt den som databasen: TLS, adgangskode, ingen offentlig eksponering |

## Kendte grænser, som vagtholdet bør vide

Udlogning ved inaktivitet sker i klienten; et tilbagekaldt token afvises med det samme, men en genindlæsning af
politikken viser sig i cachelagret indsigt efter op til 30 sekunder; break-glass kan ikke tilbagekaldes, kun udløbe.
