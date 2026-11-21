# För driftchef

> Teknisk sammanfattning, inte juridisk rådgivning, certifiering eller revision. Den anger
> vad arkivet visar per 2026-10-11. Inget här gör anspråk på ackreditering.

## Vad det krävs för att driva detta

Tjänsten är en Rust-process per replika plus PostgreSQL 18 och Redis; front end
är en Node-process. Båda levereras som härdade containeravbilder utan root,
med basavbilder låsta med digest. Driftsmanualen är
[operations-runbook.md](../../../../spec/compliance/operations-runbook.md).

## Driftfakta att planera efter

- **Misslyckas säkert vid start.** I produktion vägrar tjänsten starta om
  autentisering är avstängd, om granskningskedjan är oanvändbar eller om nödvändiga nycklar och
  `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL` saknas. En felkonfigurerad driftsättning stoppar i stället för att köra öppet.
- **Hälsa och mått.** `/_health/live`, `/_health/ready`, `/_health/startup` hastighetsbegränsas aldrig,
  så en spärr kan inte starta om en frisk pod. Mått på
  `/metrics.prom`.
- **Missbruksgränser** är på i produktion: 600 begäranden per minut per klient och en
  spärr efter 10 misslyckade inloggningar på 5 minuter. De delas mellan
  repliker via Redis; om Redis fallerar faller de tillbaka på räkning per replika.
- **Nödåtkomst** (break-glass): vilken inloggad användare som helst kan ta en motiverad
  läsförhöjning i upp till 60 minuter. Någon annan måste granska varje beviljande.
  **Process du måste köra:** larma på granskningsrader för `break_glass_granted` och
  granska antalet ogranskade (adminsidan `/break-glass/review`).
- **Fel** har en enda form (RFC 9457 `application/problem+json`, med `error` och
  `description` bevarade), så logganalys och larmregler kan lita på `status` och `type`.
- **Tillväxt i granskningen.** Läsningar granskas, så granskningstabellen växer med trafiken.
  Dimensionera gallringen för den. En klient som pollar får den att växa snabbt.

## Öppet driftarbete

| Punkt | Status |
|---|---|
| Övningar i säkerhetskopiering/återställning, repetition av nyckelrotation | Skrivna i manualen; **du** måste köra och belägga dem |
| Förankring av granskningskedjans huvud utanför databasen | Din skyldighet; API:et rapporterar huvudets hash |
| Uppdatering av låsta avbildsdigests | Dependabot öppnar veckovisa pull requests; **du** måste granska och slå ihop dem, annars blir låsningarna inaktuella |
| Verifiering av en signerad release före driftsättning | Kommandon finns i manualen; driftsätt via digest och vägra en avbild som misslyckas med verifieringen. Otestat tills en första taggad release finns |
| Riktig e-postleverantör (T39) | Öppen; ändrar bilden för incidenter och leverantörer |
| Redis innehåller insiktsdata | Skydda den som databasen: TLS, lösenord, ingen offentlig exponering |

## Kända begränsningar värda att berätta för beredskapsteamet

Utloggning vid inaktivitet sker på klientsidan; ett återkallat token vägras direkt, men en omladdning
av policy syns i cachade insikter i upp till 30 sekunder; break-glass kan inte
återkallas, bara löpa ut.
