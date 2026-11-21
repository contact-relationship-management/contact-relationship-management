# Governance, Risiko, Compliance

> Technische Übersicht, keine Rechtsberatung, Zertifizierung oder Prüfung. Sie nennt,
> was das Repository zum Stand 2026-10-11 zeigt. Nichts hier erhebt den Anspruch einer Akkreditierung.

## Wo man beginnt

Das Nachweispaket ist [spec/compliance](../../../../spec/compliance/index.md). Seine Grundregel ist die, an der das Team
festgehalten werden sollte: Eine Kontrolle ist nur dann **Implemented**, wenn Code oder Konfiguration sie durchsetzt und
ein Test oder eine Prüfung sie festschreibt; **Deployer**, wenn Ihre Organisation sie liefern muss; **Gap**, wenn heute
keines von beiden zutrifft.

| Dokument | Wofür es dient |
|---|---|
| [control-mapping](../../../../spec/compliance/control-mapping.md) | NIST SP 800-53, HIPAA Security Rule, ISO 27001 Anhang A, DSGVO Art. 32, jeweils als Implemented, Deployer oder Gap gekennzeichnet |
| [dpia-template](../../../../spec/compliance/dpia-template.md) | Datenschutz-Folgenabschätzung, vorausgefüllt mit den Datenflüssen dieses Systems |
| [operations-runbook](../../../../spec/compliance/operations-runbook.md) | Härtung, Sicherung und Wiederherstellung, Schlüssel, Protokollierung, Reaktion auf Vorfälle |
| [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) | Entwürfe für NHS-DTAC-Aussagen, die der Hersteller übernehmen kann |
| [gds-api-review](../../../../spec/compliance/gds-api-review.md) | Selbstprüfung gegen die API-Standards der britischen Regierung (ein Entwurf, nicht gegen den veröffentlichten Text geprüft) |
| [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) | Fehlerformat (RFC 9457), was als inkompatible Änderung gilt, die Schritte der Einstellung und die Ankündigungsfrist |
| [Bewertung nach NHS DTAC v2.0](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md) | Ergebnis Frage für Frage: wahrscheinlich außerhalb des DTAC-Anwendungsbereichs, und es würde die zutreffenden Abschnitte mangels organisatorischer Nachweise nicht bestehen |
| [plan](../../../../spec/plan.md) | Urteil zur Reife (nicht bereit) und das Register der Lücken |
| [tasks](../../../../spec/tasks.md) | Aufgabe, Lösung und genannte Grenzen jeder Lücke |

## Governance

- **Entscheidungsrechte:** [GOVERNANCE.md](../../../../GOVERNANCE.md), [MAINTAINERS.md](../../../../MAINTAINERS.md),
  [CODEOWNERS](../../../../CODEOWNERS). Regeln für Beiträge und die Unterstützung durch KI stehen in
  [CONTRIBUTING.md](../../../../CONTRIBUTING.md) und [AI_STATEMENT.md](../../../../AI_STATEMENT.md).
- **Änderungskontrolle:** Die Spezifikation ist die maßgebliche Quelle; Code und Tests verweisen auf sie. Die CI läuft
  bei zwei Anbietern, und `scripts/ci-check.sh` reproduziert sie. Regel im Repository: keinen Test und kein Tor
  abschwächen, damit etwas besteht.
- **Offenlegung:** [SECURITY.md](../../../../SECURITY.md) ist der Meldeweg.

## Risiko

Die Sicht auf das Risiko steht in der [Übersicht für die Risikoleitung](../risikoleiter/index.md). Kurz: Die höchsten
Restrisiken sind die von Außenstehenden ungetestete Sicherheit (T42), die nicht quantifizierte Barrierefreiheit (T48) und
die Klassifizierung der Daten, die Sache des Anwenders ist.

## Compliance: was real ist und was nicht

| Bereich | Stand |
|---|---|
| Zugriffskontrolle | Umgesetzt und getestet: rollenbasierte Maskierung, Prüfungen auf Datensatzebene, betriebliche Lesezugriffe nur für Admins, Break-glass mit verpflichtender Prüfung durch eine zweite Person |
| Audit | Umgesetzt und getestet: Hash-Kette, nur anfügend, innerhalb der Transaktion, Audit von Lesezugriffen einschließlich Listen, Deal-Listen und Insights. **Deployer:** den Kopf der Kette außerhalb der Datenbank verankern |
| Datenschutzrechte | Umgesetzt: Export für Auskunft, Löschung (abgelehnt, solange Verpflichtungen offen sind), Aufbewahrungsprüfung. **Deployer:** Rechtsgrundlage, Aufbewahrungsfristen, Verzeichnis der Verarbeitungstätigkeiten |
| Verschlüsselung | Teilweise: AES-256-GCM auf Feldebene bei zwei Spalten mit lokalen Schlüsseln, kein KMS. Verschlüsselung im Ruhezustand und bei der Übertragung ist Sache des Betreibers |
| Lieferkette | Umgesetzt: `cargo audit`, `cargo deny`, `pnpm audit`, SBOMs, Basis-Images per Digest festgelegt, Dependabot-Updates. **Gap:** Der Workflow für signierte Releases lief noch nicht auf einem echten Release (T49) |
| Barrierefreiheit | Teilweise: automatisierte axe-Scans und eine ehrliche öffentliche Erklärung. **Gap:** kein manuelles Audit (T48), Übersetzungen ungeprüft |
| Unabhängige Absicherung | **Gap:** keine (T42) |
| E-Mail- und Marketingrecht | Sendeweg mit Einwilligung zuerst umgesetzt; echter Versand existiert nur als SMTP-Weg für die Entwicklung; Abmeldelink, Sperrliste und ein signierter Webhook für Ergebnisse sind vorhanden, aber es ist kein Anbieter eingebunden, und die Auftragsverarbeitungsvereinbarung sowie die rechtliche Prüfung sind offen (T39) |

## Kontrollen, die in Ihrer Organisation eine verantwortliche Person brauchen

1. Prüfung und Alarmierung bei Break-glass-Freigaben (Alarm bei `break_glass_granted`).
2. Verankerung des Kopfes der Audit-Kette und ihre Verifikation nach Zeitplan
   (`GET /api/audits/verify`).
3. Nachweise zu Sicherung und Wiederherstellung sowie die Probe der Schlüsselrotation.
4. Umgang mit Eintritt, Wechsel und Austritt bei Ihrem Identitätsanbieter.
5. Aufbewahrung für die Audit-Tabelle, die mit dem Lesezugriff wächst.
6. Redis wie die Datenbank schützen: Sie hält zwischengespeicherte Insight-Daten.
7. Dependabot-Updates mergen und ein signiertes Release vor dem Einsatz verifizieren.

## Wofür man die Freigabe verweigern sollte

Jede Aussage, das System sei konform, zertifiziert, HIPAA-konform oder vom NHS zugelassen. Die vertretbare Aussage ist
enger: Kontrollen sind zugeordnet, die umgesetzten sind getestet, und die Lücken sind benannt und werden verfolgt.
