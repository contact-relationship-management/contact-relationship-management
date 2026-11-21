# Für die Geschäftsführung (CEO)

> Technische Übersicht, keine Rechtsberatung, Zertifizierung oder Prüfung. Sie nennt,
> was das Repository zum Stand 2026-10-11 zeigt. Nichts hier erhebt den Anspruch einer Akkreditierung.

## Die Entscheidung in einem Absatz

Dies ist ein CRM auf Demo-Niveau mit einem ungewöhnlich starken Fundament für Sicherheit und Datenschutz. Es ist **nicht
bereit**, echte Patienten- oder Bürgerdaten zu halten, und das Repository sagt dies selbst
([plan.md](../../../../spec/plan.md)). Die Einführung ist die Entscheidung, die verbleibende externe Arbeit zu
finanzieren, nicht, es einzuschalten.

## Was vorhanden ist

- Ein funktionierendes CRM für Vertrieb, Marketing, Service und Analyse, mit einer Rust-API und einem Web-Front-End,
  gebaut nach einer schriftlichen Spezifikation.
- Marketing, das mit der Einwilligung beginnt, Zugriffskontrolle, die sensible Werte vor Rollen verbirgt, die sie nicht
  sehen sollen, ein Audit-Pfad, der Manipulation sichtbar macht, Auskunft und Löschung für Datenschutzanfragen sowie
  ein Notfallzugriff („Break-glass“), der begründet, zeitlich begrenzt und geprüft ist.
- 54 von 58 verfolgten Arbeitspunkten sind erledigt ([tasks.md](../../../../spec/tasks.md)).

## Was zwischen diesem Stand und dem echten Einsatz steht

| Hindernis | Warum es wichtig ist | Wer es beheben kann |
|---|---|---|
| Keine echte E-Mail-Zustellung (T39) | Marketing wird simuliert; ein echter Anbieter braucht Verträge und eine Auftragsverarbeitungsvereinbarung | Sie wählen den Anbieter; die Rechtsabteilung unterzeichnet |
| Kein unabhängiger Sicherheitstest (T42) | Krankenhäuser und staatliche Käufer verlangen in der Regel einen; unsere eigenen Tests sind nicht unabhängig | Eine externe Firma |
| Kein manuelles Barrierefreiheits-Audit (T48) | Öffentliche Stellen müssen eine Erklärung zur Barrierefreiheit veröffentlichen, für die sie einstehen können | Menschen mit assistiven Technologien |
| Die Signierung von Releases ist gebaut, lief aber noch nicht auf einem echten Release, und niemand hat den Entwurf der API-Standards geprüft (T49) | Beschaffungs-Checklisten verlangen beides | Entwicklung (ein erstes Release taggen); eine externe Prüfung |

## Entscheidungen, die bei Ihnen liegen

1. **Welcher Markt zuerst.** Die Bewertung nach den Digital Technology Assessment Criteria des NHS
   ([docs](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md))
   kommt zu dem Schluss, dass das Produkt wahrscheinlich außerhalb des DTAC-Anwendungsbereichs liegt und die
   zutreffenden Abschnitte mangels organisatorischer Nachweise (benannter klinischer Sicherheitsbeauftragter,
   Zertifizierungen) nicht bestehen würde. Wählen Sie den Weg, bevor Sie Geld dafür ausgeben.
2. **Ob unabhängige Absicherung finanziert wird** (T42, T48). Ohne sie lautet die ehrliche Aussage: „nach hohem Standard
   gebaut, von Außenstehenden ungetestet“.
3. **Wer das Risiko trägt**, sobald echte Daten gehalten werden. Siehe die Übersichten für die
   [Risikoleitung](../risikoleiter/index.md) und die [Rechtsleitung](../rechtsleiter/index.md).

## Was nach außen nicht gesagt werden sollte

Behaupten Sie keine Konformität, keine Zertifizierung, keine „HIPAA-Konformität“ und keine „NHS-Zulassung“. Das
Nachweispaket ([compliance](../../../../spec/compliance/index.md)) stützt eine engere, wahre Aussage: Kontrollen sind
zugeordnet, umgesetzte sind getestet, Lücken sind benannt.
