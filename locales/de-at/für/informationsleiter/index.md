# Für die Informationsleitung (CIO)

> Technische Übersicht, keine Rechtsberatung, Zertifizierung oder Prüfung. Sie nennt,
> was das Repository zum Stand 2026-10-11 zeigt. Nichts hier erhebt den Anspruch einer Akkreditierung.

## Sicht auf Informationen und Daten

Dieses CRM hält **Beziehungsdaten**: Kontakte und Konten als Hüllen um Kennungen, die in anderen Systemen gehalten
werden, Aktivitäten, Deals, Tickets, Einwilligungsverlauf und ein Audit-Log. Demografische Angaben bleiben
absichtlich in den Identitätsregistern ([domain-model](../../../../spec/domain-model.md)). Wenn Ihre Organisation
Patienten oder Bürger als Kontakte erfasst, liegt die Klassifizierung dieser Daten **bei Ihnen**; der Plan ist nach der
strengeren Lesart geschrieben ([plan](../../../../spec/plan.md)).

## Wohin die Daten gehen

| Speicher | Enthält | Schutz im Repository |
|---|---|---|
| PostgreSQL | Alle Datensätze, Audit-Kette, Break-glass-Freigaben | Feldverschlüsselung bei zwei Spalten; Audit-Trigger, der nur anfügt; Verschlüsselung im Ruhezustand ist Sache des Betreibers |
| Redis | Cache der Insight-Antworten (30 s), Zähler für Ratenbegrenzung, Schlüssel der Break-glass-Freigaben | Je Aufrufer verschlüsselt: **braucht TLS, ein Passwort und keine öffentliche Erreichbarkeit** |
| Logs/Metriken | Anfragemetadaten, Prometheus-Metriken | Aufbewahrung ist Sache des Betreibers |
| E-Mail-Anbieter (künftig) | Nachrichteninhalt und Empfänger | Nicht gebaut (T39) |

## Integration und Identität

- Die Anmeldung kommt von einem Identitätsdienst, der PASETO-Token ausstellt, oder von Keycloak. Eintritt, Wechsel und
  Austritt, MFA und Sitzungsrichtlinie liegen daher **bei Ihrem Identitätsanbieter**; das CRM setzt durch, was das Token
  sagt, und weist abgelaufene zurück.
- Die API ist in OpenAPI dokumentiert ([gds-api-review](../../../../spec/compliance/gds-api-review.md)), per Header
  versioniert, mit `Link`-Headern paginiert und gibt Fehler als Problemdokumente nach RFC 9457 zurück. Die Regeln zu
  Änderung und Einstellung stehen in [api-lifecycle](../../../../spec/compliance/api-lifecycle.md).
- SBOMs (CycloneDX) werden in der CI für Dienst und Oberfläche erzeugt; der Release-Workflow signiert sie und die
  Images.

## Pflichten der Informations-Governance, die bei Ihnen bleiben

Nachweise zu Sicherung und Wiederherstellung, Aufbewahrungsfristen je Tabelle, das Verankern des Kopfes der
Audit-Kette außerhalb der Datenbank, die Verwahrung der Feldschlüssel in einem KMS, Auftragsverarbeitungsvereinbarungen
mit jedem Lieferanten und die Datenschutz-Folgenabschätzung ([dpia-template](../../../../spec/compliance/dpia-template.md)).
Die Kontrollzuordnung ([control-mapping](../../../../spec/compliance/control-mapping.md)) kennzeichnet jede Kontrolle
als Implemented, Deployer oder Gap.
