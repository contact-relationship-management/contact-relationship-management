# Weg in den Produktivbetrieb

> Contact Relationship Management ist nicht produktionsreif. Diese Seite sagt, was Produktivbetrieb hier bedeutet und
> wie weit jeder Teil gekommen ist.

Die Software arbeitet mit synthetischen Daten. E-Mail wird simuliert, sofern kein echter Mailmodus konfiguriert ist.
Geben Sie keine echten personenbezogenen Daten ein.

## Was Produktivbetrieb bedeutet

Ein produktiver Einsatz hält die Daten echter Menschen und sendet echte Nachrichten an sie. Für dieses Projekt braucht
das alles Folgende:

- **Echte Anmeldung.** Jede Anfrage wird authentifiziert, und die Zugriffsregeln entsprechen den Rollen der einsetzenden
  Organisation.
- **Rechtmäßiger Versand.** Jede Nachricht hat eine dokumentierte Einwilligung und eine Rechtsgrundlage nach dem Recht
  jedes Ortes, an den sie gesendet wird, und läuft über einen echten E-Mail-Anbieter, der Rückläufer, Beschwerden und
  Abmeldungen behandelt.
- **Unabhängige Absicherung.** Personen außerhalb des Projekts haben Sicherheit und Barrierefreiheit getestet, und die
  Befunde sind behoben oder akzeptiert.
- **Betrieb.** Wiederhergestellte Sicherungen, Überwachung, ein Vorfallprozess und jemand in Bereitschaft.
- **Nachweis der Stabilität.** Lange Läufe unter realistischer Last, auf Hardware wie der des Einsatzes, ohne Fehler
  oder Verlangsamung.

Die Produktionsreife wird je Einsatz entschieden. Ein Betreiber kann einige Tore nur für seine eigene Organisation und
seine eigenen Empfänger schließen. Die technische Bewertung hinter dieser Seite ist [spec/plan.md](../../../spec/plan.md).
Die offenen Tore stehen in [spec/tasks.md](../../../spec/tasks.md).

## Reifegrad

Jeder Prozentwert ist eine am 2026-11-13 aus der Aufgabenliste gemachte Schätzung, keine Messung.

- **Authentifizierung und Zugriffskontrolle** (70%): Anmeldung über PASETO-Token oder Keycloak/OIDC, attributbasierte
  Zugriffsregeln mit Prüfungen auf Datensatzebene und ein Produktionsmodus, der ohne Authentifizierung nicht startet, sind
  gebaut. Kein echter Einsatz hat sie mit eigenen Attributen eingeschaltet, und die Durchsetzungsmatrix deckt noch nicht
  jeden Handler ab (CRM-G1).
- **E-Mail-Zustellung** (40%): Standard ist ein simulierter Sender, und ein SMTP-Sender existiert. Es gibt keinen Adapter
  für einen E-Mail-Anbieter mit Webhooks für Rückläufer und Beschwerden und keine Sperrliste (CRM-T39).
- **Datenschutz und E-Marketing-Recht** (15%): Einwilligungsprotokolle, Auskunft, Löschung und Aufbewahrung sind gebaut.
  Für dieses Projekt wurde das Recht keiner Region gelesen, und die Prüfung des echten Sendewegs ist offen (CRM-G2,
  [regulatory](../../../spec/regulatory.md)).
- **Unabhängige Sicherheitsabsicherung** (10%): Der Umfang eines Penetrationstests ist geschrieben. Es wurde kein
  Penetrationstest, keine förmliche Risikoanalyse und keine Akkreditierung durchgeführt (CRM-T42). Die KI-Sicherheitsaudits
  sind keine unabhängige Absicherung.
- **Barrierefreiheit** (50%): Automatisierte axe-core-Prüfungen gegen WCAG 2.2 A und AA bestehen auf jeder Seitenroute.
  Es wurde kein manuelles Audit mit reiner Tastaturbedienung, Screenreadern oder 400 % Zoom durchgeführt (CRM-T48).
- **Übersetzungen** (30%): Die Oberfläche ist in acht Sprachen verfügbar. Zeichenketten in anderen Sprachen als Englisch
  sind maschinell geschrieben, und keine sprechende Person hat sie geprüft.
- **Betrieb** (30%): Ein Betriebshandbuch behandelt Sicherung und Wiederherstellung, Schlüsselverwaltung, Verschlüsselung
  im Ruhezustand, Vorfallreaktion und Aufbewahrung von Protokollen. Es wurde keine Wiederherstellung getestet, und die
  Go-live-Checkliste des Handbuchs ist offen.
- **Leistung und Stabilität** (20%): Lasttests und ein Langzeit-Stabilitätstest existieren. Der Stabilitätstest lief zwei
  Minuten gegen den Demo-Stack; es gibt keinen langen Lauf und keine Basislinie auf produktionsähnlicher Hardware.
- **Integrität der Releases** (85%): Release-Images sind für mehrere Architekturen gebaut, signiert und beglaubigt, und
  ihre Signaturen werden verifiziert. Actions und Images sind per Digest festgelegt, und der Verlauf wird auf Geheimnisse
  geprüft. Die amd64-Hälfte eines geladenen Release-Images wurde nicht geprüft, und es gibt noch keinen zweiten Code-Host
  (CRM-T192).
- **Kontinuierliche Integration** (100%): Ein Skript führt jede Prüfung auf GitHub Actions und Woodpecker aus. Beide
  Rust-Codebasen und die Oberfläche haben Untergrenzen von 95 % der Zeilen; Tore für Browser, Lint, Lieferkette,
  Mutationen geänderter Zeilen und wöchentliche vollständige Mutationen sind vorhanden (CRM-T94).
- **Demo-Schutzmaßnahmen** (100%): Jeder Besucher des Rundgangs erhält eine eigene Kopie der Demodaten, begrenzt auf
  200 Kopien. Der Rundgang ist standardmäßig aus; mit `CONTACT_RELATIONSHIP_MANAGEMENT_DEMO_TOUR=on` wird er angeboten
  (CRM-T92).

## Wie diese Seite gepflegt wird

Wenn ein Tor schließt, ändern Sie die Schätzung und ihren Nachweis im selben Commit wie die Arbeit und verlinken Sie die
Aufgabe.
