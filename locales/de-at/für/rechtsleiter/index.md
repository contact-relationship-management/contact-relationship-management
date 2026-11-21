# Für die Rechtsleitung (CLO)

> Technische Übersicht, keine Rechtsberatung, Zertifizierung oder Prüfung. Sie nennt,
> was das Repository zum Stand 2026-10-11 zeigt. Nichts hier erhebt den Anspruch einer Akkreditierung.

## Rechtliche Lage in Kürze

Das Repository ist **nicht produktionsreif** und hält synthetische Daten. Es erhebt keinen Anspruch auf Konformität mit
irgendeiner Regelung. Es setzt technische Funktionen um, die mehrere davon unterstützen, und nennt, was ein Betreiber
ergänzen muss ([regulatory](../../../../spec/regulatory.md)).

## Berührte Regelungen

| Regelung | Was die Software unterstützt | Was Sie liefern müssen |
|---|---|---|
| DSGVO / UK GDPR | Einwilligungsverlauf, Export für Auskunft, Löschung (abgelehnt, solange offene Verpflichtungen bestehen), Aufbewahrungsprüfung, Datenminimierung, Audit von Lesezugriffen | Rechtsgrundlage je Zweck, DPIA ([Vorlage](../../../../spec/compliance/dpia-template.md)), Verzeichnis der Verarbeitungstätigkeiten, Aufbewahrungsfristen, Rollen als Verantwortlicher und Auftragsverarbeiter |
| PECR / E-Privacy | Versand nur mit Einwilligung, sofortige Abmeldung | Absenderidentifikation und Entscheidungen zum Soft-Opt-in für einen echten Absender |
| HIPAA (bei US-Gesundheitsdaten) | Audit-Kontrollen, Zugriffskontrolle, Integrität, Funktionen zur Übertragungssicherheit | Eine Business-Associate-Vereinbarung, Risikoanalyse, Richtlinien, Schulung der Mitarbeitenden. Die Software macht Sie nicht konform |
| NHS DTAC / DCB0129/0160 | Entwürfe für Nachweise ([dtac-supporting](../../../../spec/compliance/dtac-supporting.md)) | Benannter klinischer Sicherheitsbeauftragter, Gefährdungsprotokoll, Zertifizierungen. Die Bewertung sagt, dass DTAC wahrscheinlich nicht gilt |
| Barrierefreiheit (WCAG 2.2 AA, öffentlicher Sektor) | Automatisierte Prüfungen und eine ehrliche Erklärung | Ein manuelles Audit (T48), bevor irgendeine Konformität behauptet wird |
| Staatliche Sicherheit (NIST 800-53, ISO 27001, Cyber Essentials) | Kontrollzuordnung ([control-mapping](../../../../spec/compliance/control-mapping.md)) | Zertifizierung und unabhängige Absicherung |

## Verträge, die Sie brauchen werden und die es noch nicht gibt

1. Auftragsverarbeitungsvereinbarung und Bedingungen für internationale Übermittlungen mit dem E-Mail-Anbieter (T39).
2. Leistungsbeschreibung und Vertraulichkeitsbedingungen mit der Firma für den Penetrationstest (T42).
3. Hosting- und Unterauftragsverarbeiter-Bedingungen für PostgreSQL, Redis und Logs.
4. Lizenz: siehe [LICENSE.md](../../../../LICENSE.md); das Repository enthält außerdem Erklärungen zu Sicherheit,
   Governance und KI-Nutzung.

## Besondere rechtliche Beobachtungspunkte

- **Löschung** anonymisiert den Kontakt und wird abgelehnt, solange ein offener Deal, ein offenes Ticket oder eine
  aktive Nurture-Einschreibung besteht; die Audit-Kette führt Aufzeichnungen über Handlungen, klären Sie also in Ihrer
  DPIA, wie das mit Löschanfragen zusammenwirkt.
- **Break-glass**-Lesezugriffe auf maskierte Daten sind nur auf Ihrer dokumentierten Grundlage rechtmäßig; das Feld
  für den Grund ist ein Nachweis, keine Befugnis.
- **Scoring** von Leads: Wenn es jemals wesentliche Entscheidungen über Personen beeinflusst, gelten Regeln für
  automatisierte Entscheidungen ([regulatory](../../../../spec/regulatory.md)).
- **API-Zusagen:** [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) legt eine Ankündigungsfrist von
  180 Tagen fest, bevor eine API-Änderung entfernt wird. Das ist die Entscheidung der Maintainer; wenn sich Kunden darauf
  verlassen werden, entscheiden Sie, ob sie in einen Vertrag gehört.
- **Öffentliche Aussagen:** Behaupten Sie keine Konformität, Zertifizierung oder Zulassung.
