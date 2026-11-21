# Für die Sicherheitsleitung (CSO)

> Technische Übersicht, keine Rechtsberatung, Zertifizierung oder Prüfung. Sie nennt,
> was das Repository zum Stand 2026-10-11 zeigt. Nichts hier erhebt den Anspruch einer Akkreditierung.

## Sicherheitslage in einem Absatz

Der Dienst **fällt sicher aus**, steuert den Zugriff nach Rolle und nach Datensatz, verbirgt sensible Werte vor Rollen,
die sie nicht sehen sollen, und führt einen manipulationssicheren Audit-Pfad. Diese Kontrollen sind von den Autoren
umgesetzt und getestet. Sie wurden **von niemandem Unabhängigen getestet** (T42); behandeln Sie daher jede Aussage
unten als „gebaut und selbst getestet“, nicht als „verifiziert“.

## Kontrollen und wo man sie prüft

| Bereich | Was vorhanden ist | Wo nachzusehen ist |
|---|---|---|
| Authentifizierung | PASETO v4.public offline verifiziert, optional Keycloak; Produktion startet nicht, wenn die Authentifizierung aus ist | [auth](../../../../spec/auth.md) |
| Autorisierung | Attributbasiert: lesen, schreiben, löschen, zerstörerisch; eine `mask`-Verpflichtung; Prüfungen auf Datensatzebene; Audit, Aufbewahrung und Break-glass-Prüfung nur für Admins | [auth](../../../../spec/auth.md), Persona-Matrix-Test |
| Offenlegung von Daten | Maskierung bei Datensätzen, Aggregaten und Insights; ein Leck in den Kontodetails wurde gefunden und behoben (T47) | [tasks](../../../../spec/tasks.md) |
| Notfallzugriff | Schriftlicher Grund, Grenze von 60 Minuten, nur lesend, verpflichtende Prüfung durch eine zweite Person, auditiert, über Redis geteilt | Break-glass-Aufgaben T44 |
| Audit | SHA-256-Hash-Kette in derselben Transaktion, Trigger, der nur anfügt, Endpunkt zur Verifikation; Lesezugriffe auditiert | [audit](../../../../spec/audit.md) |
| Missbrauchsresistenz | Ratenbegrenzung (600 pro Minute) und Sperre (10 Fehlversuche in 5 Minuten), über Redis geteilt; Sonden ausgenommen | T43 |
| Browser und Transport | Strenge CSP mit Hashes, Verbot von Frames, `nosniff`, Referrer-Richtlinie, geschlossenes CORS, sichere Header; kein Token in JavaScript des Browsers (BFF) | UI `hooks.server.ts` |
| Datenschutz | AES-256-GCM bei zwei Spalten mit Rotation; lokale Schlüssel, kein KMS | T41 |
| Lieferkette | `cargo audit`, `cargo deny`, `pnpm audit`, SBOMs, gehärtete Images ohne Root-Rechte per Digest festgelegt, wöchentliche Dependabot-Updates, ein Release-Workflow, der Images und SBOMs schlüssellos signiert | [control-mapping](../../../../spec/compliance/control-mapping.md), [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Fehler | Jeder Fehler ist ein Problemdokument nach RFC 9457; ein `401` nennt das akzeptierte Schema | [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) |
| Speichersicherheit | `forbid(unsafe_code)`, clippy pedantic | `src/lib.rs` |

## Bekannte Schwächen (benannt, nicht verborgen)

- **Kein unabhängiger Penetrationstest und keine Bedrohungsmodellierung** (T42). Das Bedrohungsmodell in
  [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) ist eine Zusammenfassung, kein strukturiertes.
- **Die Signierung von Releases lief nicht auf einem echten Release** (T49). Workflow und Verifikationsbefehle
  existieren; bis ein Tag gepusht wird, sind sie ungetestet. Die Actions sind per Commit-Hash festgelegt.
- **Feldverschlüsselung** deckt zwei Spalten mit lokalen Schlüsseln ab, kein KMS.
- **Erneute Authentifizierung** vor zerstörerischen Aktionen ist nicht gebaut.
- **Redis hält personenbezogene Daten** (zwischengespeicherte Insights, Freigabeschlüssel) und muss wie die Datenbank
  geschützt werden: TLS, Passwort, keine öffentliche Erreichbarkeit.
- **Break-glass** kann nicht widerrufen werden, nur ablaufen, und es ist kein Alarm angebunden.
- **Ratenbegrenzung und Sperre** fallen auf Zähler je Replikat zurück, wenn Redis ausfällt; der Bucket für nicht
  authentifizierte Aufrufer gilt je Adresse, und hinter einem Proxy, der nicht in `TRUSTED_PROXIES` steht, ist das die
  Adresse des Proxys, sodass ein Angreifer andere nicht authentifizierte Aufrufer aussperren kann. Ein Aufrufer mit
  gültigem Token wird nach seinem Subjekt gezählt.
- **Neuladen der Richtlinie** zeigt sich in zwischengespeicherten Insights erst nach bis zu 30 Sekunden.
- **Abmeldung bei Inaktivität** läuft im Client.
- **Idempotenzschlüssel** sind in der API optional (T52): Ein Client, der keinen `Idempotency-Key` sendet, ist nicht
  davor geschützt, dass eine wiederholte Anlage einen Datensatz dupliziert. Die mitgelieferte Oberfläche sendet einen.

## Was der Betreiber tun muss

1. Die Authentifizierung eingeschaltet lassen; Produktionsschlüssel und `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL`
   bereitstellen.
2. Alarm bei `break_glass_granted` einrichten und Freigaben zügig prüfen.
3. Den Kopf der Audit-Kette außerhalb der Datenbank verankern und ihn nach Zeitplan verifizieren
   (`GET /api/audits/verify`).
4. PostgreSQL und Redis schützen (Netzwerk, TLS, Zugangsdaten, Sicherungen, Verschlüsselung im Ruhezustand); die
   Feldschlüssel mit der mitgelieferten Aufgabe rotieren.
5. Die Dependabot-Updates mergen; die Lieferketten-Stufe erneut ausführen; ein signiertes Release vor dem Einsatz
   verifizieren.
6. Dem Betriebshandbuch für die Reaktion auf Vorfälle folgen
   ([operations-runbook](../../../../spec/compliance/operations-runbook.md)).

## Umgang mit Schwachstellen

Meldungen laufen über [SECURITY.md](../../../../SECURITY.md). Beauftragen Sie den unabhängigen Test (T42) gegen einen
Staging-Build, bevor echte Daten vorhanden sind, und testen Sie nach wesentlichen Änderungen erneut.

## Empfohlene Sicherheitsmaßnahmen, in dieser Reihenfolge

1. Den unabhängigen Penetrationstest umreißen und beauftragen (T42).
2. Ein erstes Release-Tag pushen, um Signierung und Verifikation zu belegen (T49).
3. Alarmierung für Break-glass und für fehlgeschlagene Verifikationen der Audit-Kette anbinden.
4. Redis in jeder Umgebung hinter TLS und Zugangsdaten stellen.
5. KMS-gestützte Feldschlüssel und breitere Abdeckung der Felder (Rest von T41).

Siehe auch die Übersichten für die [Technologieleitung](../technologieleiter/index.md) und die
[Risikoleitung](../risikoleiter/index.md).
