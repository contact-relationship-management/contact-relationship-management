# Für die Personalleitung (CPO)

> Technische Übersicht, keine Rechtsberatung, Zertifizierung oder Prüfung. Sie nennt,
> was das Repository zum Stand 2026-10-11 zeigt. Nichts hier erhebt den Anspruch einer Akkreditierung.

## Warum Menschen für dieses Produkt wichtig sind

Der Zugriff auf personenbezogene Daten wird über Rollen entschieden. Die Software setzt durch, was das Token einer Person
sagt; **wer welche Rolle erhält und wann sie entzogen wird, ist ein Personalprozess**, den dieses Repository nicht für
Sie leisten kann.

## Was die Software Ihren Prozessen bietet

- **Rollenbasierte Sichtbarkeit:** Einige Rollen sehen maskierte Werte bei Kontaktkanälen, Deal-Beträgen und
  Geldansichten; Manager, Admin und Service-Ebenen sehen mehr ([auth](../../../../spec/auth.md)).
- **Auditierte Lesezugriffe:** Das Lesen personenbezogener Daten hinterlässt eine Spur, die die Person nennt. Die
  Beschäftigten sollten dies vor dem Go-live erfahren.
- **Notfallzugriff:** Eine Person kann eine begründete Leseerhöhung von bis zu 60 Minuten nehmen; eine andere Person muss
  sie anschließend prüfen. Schulen Sie beide Seiten.
- **Abmeldung bei Inaktivität** nach 15 Minuten im Browser.
- **Barrierefreiheit:** eine öffentliche Erklärung und automatisierte Prüfungen; das manuelle Audit (T48) braucht noch
  Benutzer mit Behinderungen und Tester für assistive Technologien.
- **Sprachen:** Die Oberfläche ist in 8 Regionen übersetzt; die Übersetzungen wurden **nicht** von Muttersprachlern
  geprüft.

## Ihre Verantwortung

| Bereich | Maßnahme |
|---|---|
| Eintritt, Wechsel, Austritt | Rollen über Ihren Identitätsanbieter erteilen und entziehen; das CRM weist abgelaufene Token ab, verwaltet aber keine Konten |
| Schulung | Datenschutz, Einwilligungsregeln, Nutzung und Prüfung von Break-glass, barrierefreie Praxis |
| Richtlinien | Zulässige Nutzung, Hinweis zur Überwachung (Lesezugriffe werden auditiert), disziplinarischer Weg bei Missbrauch des Notfallzugriffs |
| Kapazität der Prüfer | Benennen, wer Break-glass-Freigaben prüft, und die Vertretung bei Abwesenheit |
| Personalgewinnung | Rollen, die das Programm braucht: Ansprechperson für den unabhängigen Tester, klinischer Sicherheitsbeauftragter bei NHS-Anwendungsbereich, Datenschutzbeauftragter, Beauftragter für Barrierefreiheit |
| Wohlergehen | Beschäftigte unterstützen, die sensible Fälle bearbeiten; das CRM ist kein Fallverwaltungssystem, kann aber sensible Beziehungsdaten halten |

## Offene Punkte, die Menschen brauchen

Manuelles Barrierefreiheits-Audit mit Benutzern assistiver Technologien (T48), Prüfung der Übersetzungen durch
Muttersprachler und die Zuständigkeit für die oben genannten Pflichten zu Prüfung und Alarmierung. Siehe
[tasks.md](../../../../spec/tasks.md).
