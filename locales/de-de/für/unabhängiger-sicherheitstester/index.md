# Unabhängiger Sicherheitstester

> Technische Übersicht, keine Rechtsberatung, Zertifizierung oder Prüfung. Sie nennt,
> was das Repository zum Stand 2026-10-11 zeigt. Nichts hier erhebt den Anspruch einer Akkreditierung.

## Warum es diese Seite gibt

Jede Sicherheitskontrolle in diesem Projekt wurde von ihren Autoren gebaut und getestet (CRM-T42). Das zeigt, dass die
Kontrollen so funktionieren, wie die Autoren sie verstanden haben. Es zeigt nicht, dass nichts fehlt. Krankenhäuser und
staatliche Käufer verlangen in der Regel einen Test durch Unabhängige, bevor sie echte Daten halten, und die ehrliche
Aussage lautet bis dahin „nach hohem Standard gebaut, von Außenstehenden ungetestet“
([Übersicht für die Geschäftsführung](../geschäftsführer/index.md),
[Übersicht zur Sicherheit](../sicherheitsleiter/index.md)).

Diese Seite ist das, was man einer Testfirma übergibt, und das, was man zurückerwarten darf.

## Was Bestehen bedeutet und was nicht

Ein sauberer Bericht sagt, dass ein benanntes Team in einer benannten Zeit mit einem benannten Umfang keine ernsten
Probleme gefunden hat. Er ist **keine** Zertifizierung, keine Akkreditierung und keine Garantie. Beschreiben Sie das
System nicht als „sicher“, „zertifiziert“ oder „nach einem Standard penetrationsgetestet“, über das hinaus, was der
Bericht sagt ([Übersicht zu GRC](../governance-risiko-compliance/index.md)).

## Den Tester auswählen

- Unabhängig von den Autoren und von allen, die den Einsatz konfiguriert haben.
- Mit benannten, erfahrenen Testern besetzt, mit einschlägiger Akkreditierung für Ihren Markt, wo eine gilt (zum Beispiel
  CREST oder das NCSC-CHECK-Schema für Arbeiten für die britische Regierung). Fragen Sie, welche Akkreditierung die
  Personen abdeckt, die die Arbeit leisten, nicht nur die Firma.
- Vereinbart schriftlich Umfang, Einsatzregeln, Vertraulichkeit, Umgang mit Daten und Haftung, bevor irgendein Test
  beginnt. Den Vertrag vereinbart die Rechtsabteilung ([Übersicht für die Rechtsleitung](../rechtsleiter/index.md)).

## Was zu testen ist

| Bereich | Was die Tester versuchen sollten | Wo das Design beschrieben ist |
|---|---|---|
| Authentifizierung | Gefälschte, abgelaufene, wiedergegebene und für den falschen Empfänger ausgestellte Token; Rückstufung auf nicht authentifiziert; die Prüfung beim Start, die den Lauf ohne Authentifizierung verweigert | [auth](../../../../spec/auth.md) |
| Autorisierung | Datensätze oder Aktionen erreichen, die eine Rolle nicht erreichen soll: horizontale und vertikale Eskalation, Prüfungen auf Datensatzebene, zerstörerische Aktionen, Lesezugriffe nur für Admins (Audit, Aufbewahrung, Break-glass-Prüfung) | [auth](../../../../spec/auth.md) |
| Offenlegung von Daten | Maskierte Werte, die über Listen, gebündelte Detaillesezugriffe, Insights, Aggregate, Fehlermeldungen, Caching oder den Cache-Schlüssel durchsickern | [tasks](../../../../spec/tasks.md) (T32, T47) |
| Notfallzugriff | Break-glass nehmen, verlängern oder missbrauchen; die eigene Freigabe prüfen; zweimal prüfen | [auth](../../../../spec/auth.md) (T44) |
| Audit | Verlauf ändern, löschen oder umordnen; die Kette umgehen; Lesezugriffe verbergen | [audit](../../../../spec/audit.md) |
| Missbrauchsresistenz | Umgehung von Ratenbegrenzung und Sperre, andere Benutzer aussperren, Fälschen von Headern für den Client-Schlüssel | T43 |
| Verarbeitung von Eingaben | Injection (SQL, JSON, Header), überdimensionierte Bodys, fehlerhafte Cursor und Idempotenzschlüssel, Wiedergabe oder Kollision von Schlüsseln zwischen Aufrufern | T51, T52 |
| Web-Front-End | Cross-Site-Scripting, Request Forgery, Sitzungsbehandlung, der Backend-for-Frontend-Proxy, Content Security Policy, Clickjacking | UI `hooks.server.ts` |
| Infrastruktur wie ausgeliefert | Container-Images, offene Ports, Erreichbarkeit von Redis und PostgreSQL, Umgang mit Geheimnissen, Standardwerte der Konfiguration | [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Lieferkette | Herkunft von Abhängigkeiten und Images, die SBOMs, nicht signierte Builds | [control-mapping](../../../../spec/compliance/control-mapping.md) |
| Geschäftslogik | Umgehung der Einwilligung im Sendeweg, Löschung, die Daten übrig lässt, Manipulation des Scorings, doppelte Anlage | [regulatory](../../../../spec/regulatory.md) |

**Außerhalb des Umfangs, sofern Sie nichts anderes vereinbaren:** der Identitätsdienst, der die Token ausstellt (er liegt
außerhalb dieses Repositorys), die Hosting-Plattform und Denial-of-Service in großem Umfang.

## Was den Testern zu geben ist

1. Ein **Staging-Build**, der dem entspricht, was in Produktion laufen wird: Produktionskonfiguration, Authentifizierung
   an, TLS, ein echtes Redis und PostgreSQL. Testen Sie nicht die Standardwerte der Demo.
2. **Nur synthetische Daten.** Die Seed-Aufgabe erzeugt einen Kundenbestand ohne echte Menschen. Echte Patienten- oder
   Bürgerdaten dürfen niemals in einer Testumgebung sein.
3. **Ein Konto je Rolle**, damit die Autorisierung von jeder Seite getestet werden kann. Die Referenzrichtlinie ist
   `config/abac-policy.reference.json`.
4. Die oben verlinkten Dokumente, das OpenAPI-Dokument unter `/api-docs/openapi.json` und Lesezugriff auf das
   Repository, damit der Test Black-Box-Prüfung und Code-Review verbinden kann.
5. Die **bekannten Schwächen**, damit die Tester ihre Zeit auf das verwenden, was nicht bereits aufgeschrieben ist:
   - Die Signierung von Releases ist gebaut, lief aber noch nicht auf einem echten Release (T49);
   - die Feldverschlüsselung deckt zwei Spalten mit lokalen Schlüsseln ab, kein KMS;
   - die erneute Authentifizierung ist nicht gebaut;
   - Break-glass kann nicht widerrufen werden, nur ablaufen, und es ist kein Alarm angebunden;
   - das Neuladen der Richtlinie zeigt sich in zwischengespeicherten Insights bis zu 30 Sekunden später;
   - die Abmeldung bei Inaktivität läuft im Client;
   - der Zustand der Ratenbegrenzung fällt auf Replikate zurück, wenn Redis ausfällt.
   Bitten Sie sie, jede zu bestätigen oder zu widerlegen, nicht, sie zu überspringen.

## Einsatzregeln, die schriftlich zu vereinbaren sind

- Benannte Tester, Termine und Quelladressen; eine 24-Stunden-Kontaktperson auf Ihrer Seite.
- Was versucht werden darf (zum Beispiel keine Zerstörung des Datenspeichers des Tests ohne Ankündigung) und was beim
  Nachweis der Offenlegung aufhören muss.
- Wer bei einem kritischen Befund sofort informiert wird und wie, noch vor dem Abschlussbericht.
- Wie der Bericht, die Nachweise und alle Notizen der Tester aufbewahrt und vernichtet werden.

## Was Sie zurückerhalten sollten

- Einen Bericht mit Umfang, Methode, Terminen, Namen der Tester und Akkreditierung.
- Jeden Befund mit Schweregrad, Nachweis, Schritten zur Reproduktion, betroffener Komponente und empfohlener Behebung.
- Eine klare Aussage dazu, was **nicht** getestet wurde.
- Einen erneuten Test der behobenen Befunde, im Auftrag enthalten oder mit Preis.

## Nach dem Bericht

1. Jeden Befund in `spec/tasks.md` als Aufgabe mit verantwortlicher Person und Datum einordnen.
2. Kritische und hohe Befunde vor echten Daten beheben und die Behebungen dann von den Testern bestätigen lassen.
3. Für jede Behebung einen Regressionstest hinzufügen. Die Regel des Projekts lautet, keinen Test und kein Tor
   abzuschwächen, damit etwas besteht.
4. Das Ergebnis ehrlich festhalten: Datum des Berichts, Umfang, die Befunde und ihr Status. Den Reifeplan
   ([plan](../../../../spec/plan.md)) und das Compliance-Paket ([compliance](../../../../spec/compliance/index.md))
   aktualisieren.
5. Nach wesentlichen Änderungen erneut testen und nach einem Zeitplan, den Ihre risikoverantwortliche Person festlegt.
   Ein Test ist eine Momentaufnahme, kein Zustand.

## Reihenfolge der Arbeit

1. Zuerst schließen, was billig und bekannt ist, damit die Tester das Budget nicht damit verbrauchen, es neu zu
   entdecken: ein erstes signiertes Release (T49), Alarmierung bei Break-glass und bei fehlgeschlagener Verifikation des
   Audits, Redis hinter TLS und Zugangsdaten.
2. Den Staging-Build, die Konten und die Seed-Daten vorbereiten.
3. Umfang, Vertrag und Einsatzregeln vereinbaren.
4. Testen; kritische Befunde sofort erhalten.
5. Beheben, erneut testen und festhalten.
6. Erst dann externe Aussagen aktualisieren, und nur so weit, wie der Bericht sie stützt.

## Entscheidungen für Sie

1. Wer beauftragt und wer die Befunde verantwortet (siehe die
   [Übersicht für die Risikoleitung](../risikoleiter/index.md)).
2. Welche Akkreditierung Ihr Markt von den Testern erwartet.
3. Budget und Termin, bestimmt davon, was eine Verzögerung kostet, nicht davon, was bequem ist.
4. Ob einmal vor dem Start oder nach einem wiederkehrenden Zeitplan getestet wird.
