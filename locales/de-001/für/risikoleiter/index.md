# Für die Risikoleitung (CRO)

> Technische Übersicht, keine Rechtsberatung, Zertifizierung oder Prüfung. Sie nennt,
> was das Repository zum Stand 2026-10-11 zeigt. Nichts hier erhebt den Anspruch einer Akkreditierung.

## Risikolage

**Insgesamt: nicht bereit für regulierte Daten.** Die Software ist für ihre Phase gut kontrolliert, aber die wichtigsten
Restrisiken sind durch Tests und Prozesse nicht gemindert, die nur die einführende Organisation oder externe Parteien
liefern können.

## Wichtigste Risiken

| # | Risiko | Grundlage der Einstufung | Minderung im Repository | Restrisiko |
|---|---|---|---|---|
| 1 | Unentdeckte Schwachstellen | Kein unabhängiger Penetrationstest (T42) | Eigene Tests, Tore für die Lieferkette, strenge Header/CSP | **Hoch**, bis getestet |
| 2 | Offenlegung sensibler Felder gegenüber der falschen Rolle | Komplexe Zugriffsregeln | Rollenbasierte Maskierung bei Datensätzen, Aggregaten und Insights; ein Test der Persona-Matrix; ein Leck in den Kontodetails wurde gefunden und behoben (T47) | Mittel: Die Maskierung ist getestet, aber nicht unabhängig verifiziert |
| 3 | Missbrauch des Notfallzugriffs durch Insider | Break-glass erhöht Lesezugriffe | Schriftlicher Grund, Grenze von 60 Minuten, nur lesend, verpflichtende Prüfung durch eine zweite Person, auditiert | Mittel: kein Alarm angebunden; kein Widerruf |
| 4 | Audit-Pfad verändert | Erfordert Datenbankzugriff | Hash-Kette plus Trigger, der nur anfügt; meldet den ersten Bruch | Niedrig bis mittel: Der Kopf-Hash muss außerhalb der Datenbank verankert werden |
| 5 | Unrechtmäßiges Marketing | Fehler bei der Einwilligung | Der Sendeweg setzt die Einwilligung durch, nicht die Oberfläche; unveränderlicher Einwilligungsverlauf | Mittel, bis ein echter Anbieter eingebunden und geprüft ist (T39) |
| 6 | Verfügbarkeit / Missbrauch | Öffentliche API | Ratenbegrenzung und Sperre, über Redis geteilt | Mittel: Grenzen fallen auf Replikate zurück, wenn Redis ausfällt |
| 7 | Verstoß gegen die Barrierefreiheit | Pflicht des öffentlichen Sektors | Automatisierte axe-Scans; ehrliche Erklärung zur Barrierefreiheit | **Nicht quantifiziert**, bis zum manuellen Audit (T48) |
| 8 | Lieferkette | Abhängigkeiten und Images | `cargo deny/audit`, `pnpm audit`, SBOM, von Dependabot aktualisierte Digest-Festlegungen, ein schlüsselloser signierter Release-Workflow | Mittel: Die Signierung lief noch nicht auf einem echten Release (T49) |
| 9 | Doppelte Datensätze durch Wiederholungen | Idempotenzschlüssel sind in der API optional (T52) | Wiedergabe für Clients, die den Schlüssel senden; die mitgelieferte Oberfläche tut es immer | Niedrig |
| 10 | Falsche Klassifizierung von Daten | Der Betreiber entscheidet, was ein „Kontakt“ ist | Hinweis zum Umfang und DPIA-Vorlage | Hängt von Ihnen ab |

## Lücken, die ihrer Natur nach offen bleiben

T39 (echter E-Mail-Anbieter), T42 (unabhängige Absicherung), T48 (manuelles Barrierefreiheits-Audit), T49 (ein erstes
signiertes Release, eine Prüfung des API-Entwurfs durch eine Prüferin oder einen Prüfer). Siehe
[tasks.md](../../../../spec/tasks.md) und [plan.md](../../../../spec/plan.md).

## Empfohlene Risikomaßnahmen

Erfassen Sie die Risiken 1, 7 und 10 förmlich; verlangen Sie T42 vor echten Daten; bestimmen Sie eine verantwortliche
Person für Prüfung und Alarmierung bei Break-glass; legen Sie den Rhythmus für die Verankerung des Audit-Kopfes fest.
