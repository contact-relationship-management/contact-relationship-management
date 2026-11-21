# Contact Relationship Management

Open-Source-CRM-Software für Vertrieb, Marketing und Kundenservice.

Contact Relationship Management verwaltet Beziehungen zu Kunden und Interessenten in Vertrieb, Marketing und Service. Die Software führt für jeden Kontakt genau einen Datensatz und verlangt eine dokumentierte Einwilligung, bevor eine Nachricht versendet wird. Sie ist nicht produktionsreif: Sie arbeitet mit synthetischen Daten.

## Vorteile

- **Ein Kontaktdatensatz.** Vertrieb, Marketing, Service und Analyse nutzen denselben Datensatz.
- **Dokumentierte Einwilligung.** An einen Kontakt wird nichts gesendet, solange keine Einwilligung dokumentiert ist. Jede E-Mail enthält einen signierten Abmeldelink.
- **Weniger personenbezogene Daten.** Das CRM speichert einen Verweis auf eine Person in einem Identitätsdienst, nicht eine Kopie von Name oder Adresse.
- **Nachvollziehbare Ergebnisse.** Lead-Scores werden aus lesbaren Regeln berechnet. Prognosen werden aus gewichteten Phasen der Pipeline berechnet.
- **Internationales Design.** Namen, Adressen, Kalender und Schreibrichtungen sind nicht an das Format eines einzelnen Landes gebunden. Die Oberfläche ist in acht Sprachen verfügbar.
- **Öffentliche Spezifikation.** Spezifikation, Code und Tests sind öffentlich. Die Dokumentation sagt, was gebaut ist und was nicht.

## Funktionen

- **Vertrieb.** Regelbasierte Lead-Bewertung, Deal-Pipelines als Board und nach Phasen gewichtete Prognosen.
- **Marketing.** Segmentierte Kampagnen und Nurture-Sequenzen. Der Versand wird simuliert, sofern kein Mailmodus konfiguriert ist.
- **Service.** Tickets mit Prioritäten und Fristen nach Service-Level sowie Artikel einer Wissensdatenbank.
- **Analyse.** Dashboards für Abschlussquote, Pipeline nach Phase, Kampagnenertrag, Service-Level-Zustand und Kundenwert über die gesamte Kundenbeziehung.
- **Datenqualität.** Erkennung von Dubletten nach Wahrscheinlichkeit, mit Zusammenführen und Rückgängigmachen.
- **Governance.** Audit-Log, rollenbasierter Zugriff und Unterstützung bei Auskunfts- und Löschungsanfragen.

## Erste Schritte

1. **Code beschaffen.** Das Repository klonen. Node.js und pnpm sind erforderlich.
2. **Anwendung starten.** Im Svelte-Verzeichnis `pnpm install` und dann `pnpm dev` ausführen.
3. **Anmelden.** `example@example.com` verwenden. Es ist kein weiterer Dienst erforderlich.
4. **Rundgang ansehen.** Der Rundgang behandelt Kontakte, Leads, Deals, Kampagnen, Tickets und die Sicht der Geschäftsleitung anhand fiktiver Daten.

Das [Tutorial](../../docs/tutorial-first-tour.md) beschreibt jeden Schritt. [Installation und Nutzung](../../INSTALL.md) behandelt den vollständigen Dienst.

## Vergleich

| Projekt | Kategorie | Unterschied |
| --- | --- | --- |
| [Twenty](../../docs/compare/twenty.md) | Open-Source-CRM | Vollständiges Produkt. Dieses Projekt ist eine kleinere, spezifikationsgetriebene Demonstration. |
| [EspoCRM](../../docs/compare/espocrm.md) | Open-Source-CRM | Größerer Funktionsumfang. Dieses Projekt legt den Schwerpunkt auf nachvollziehbare Regeln. |
| [SuiteCRM](../../docs/compare/suitecrm.md) | Open-Source-CRM | Ausgereift und umfangreich. Dieses Projekt ist eine kompakte Referenz. |
| [Salesforce](../../docs/compare/salesforce.md), [HubSpot](../../docs/compare/hubspot.md) | Kommerzielles SaaS-CRM | Dieses Projekt bildet dieselbe Praxis nach. Es werden weder Code noch Daten geteilt. |

Dieses Projekt ist eine Demonstration und steht nicht im Wettbewerb mit diesen Produkten. Siehe [alle Vergleiche](../../COMPARISONS.md).

## Rollenübersichten

Die [Rollenübersichten](für/) bieten für jede leitende Rolle eine Seite: was die Software leistet, was sie nicht leistet und welche Entscheidungen zu dieser Rolle gehören.

## Status

Diese Software ist nicht produktionsreif ([Weg in den Produktivbetrieb](weg-in-den-produktivbetrieb/index.md)). Nur synthetische Daten verwenden. Es wird keine echte E-Mail versendet, sofern kein echter Mailmodus konfiguriert ist. Nichts auf dieser Website ist Rechtsberatung.
