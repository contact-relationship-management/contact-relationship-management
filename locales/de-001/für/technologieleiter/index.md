# Für die Technologieleitung (CTO)

> Technische Übersicht, keine Rechtsberatung, Zertifizierung oder Prüfung. Sie nennt,
> was das Repository zum Stand 2026-10-11 zeigt. Nichts hier erhebt den Anspruch einer Akkreditierung.

## Architektur

- **Dienst:** Rust auf Axum und axa (sqlx 0.9), PostgreSQL 18, Redis für Cache, gemeinsame Zähler und die Verbreitung
  von Break-glass. `forbid(unsafe_code)`, clippy pedantic. Maßgeblich für das Verhalten ist die Spezifikation
  ([index](../../../../spec/index.md), [architecture](../../../../spec/architecture.md)).
- **Oberfläche:** SvelteKit 3 / Svelte 5, ein Backend-for-Frontend-Proxy, sodass kein Token je JavaScript des Browsers
  erreicht, strenge CSP mit Hashes.
- **Authentifizierung:** PASETO v4.public offline verifiziert, optional Keycloak; attributbasierte Zugriffskontrolle
  (Aktionen lesen/schreiben/löschen/zerstörerisch, eine `mask`-Verpflichtung, Prüfungen auf Datensatzebene) aus einer
  gemeinsamen Engine.
- **Audit:** SHA-256-Hash-Kette, in derselben Transaktion wie die Änderung geschrieben, Trigger, der nur anfügt,
  verifizierbar unter `GET /api/audits/verify`.
- **Datenschutz:** AES-256-GCM-Feldverschlüsselung mit Schlüsselrotation bei zwei Spalten; lokale Schlüssel, kein KMS.

## Nachweise zur Qualität

Unit-Tests (185), Datenbanktests (56, darunter Durchsetzung, Maskierung, Audit, Paginierung, Caching, Idempotenz,
Schemaprüfungen), Front-End-Unit-Tests (106) und Playwright (98), einschließlich axe-Scans zur Barrierefreiheit. Die CI
führt fmt, clippy, Tests, Front-End-Build/Lint und eine Lieferketten-Stufe (`cargo audit`, `cargo deny`, `pnpm audit`,
CycloneDX-SBOMs) auf GitHub Actions und Woodpecker aus. Ein Befehl reproduziert sie: `scripts/ci-check.sh`.

## Technische Schulden und ehrliche Grenzen

- Die Paginierung verwendet einen Offset-Cursor (Zeilen können übersprungen oder wiederholt werden, wenn sich Daten
  mitten im Durchlauf ändern); absichtlich keine Gesamtzahl.
- Idempotenzschlüssel sind in der API optional (T52): Ein Client muss bei einer Anlage `Idempotency-Key` senden, um davor
  geschützt zu sein, dass eine Wiederholung einen Datensatz dupliziert. Die mitgelieferte Oberfläche sendet bei jeder
  Anlage einen.
- Die OpenAPI-Schemas sind von Hand geschrieben. Sie decken Anfragen, Listen, Detaillesezugriffe, Dashboards,
  Insight-Ansichten, Audit, Datenschutz und Break-glass ab; Statusübergänge und Upserts, die den aktualisierten
  Datensatz zurückgeben, sind nicht typisiert. Ein Test schlägt fehl, wenn eine Route und das Dokument abweichen, und
  andere prüfen echte Antworten, auch maskierte, gegen die Schemas
  ([gds-api-review](../../../../spec/compliance/gds-api-review.md)).
- Fehler sind Problemdokumente nach RFC 9457 (T54), die die früheren Mitglieder `error` und `description` behalten;
  Fehler aus Schichten außerhalb des Routers (Größenlimit, Zeitüberschreitung) werden nicht umgewandelt. Die Richtlinie
  zu Versionierung und Einstellung ist [api-lifecycle](../../../../spec/compliance/api-lifecycle.md); die Header
  `Deprecation` und `Sunset` sind spezifiziert, aber nicht gebaut.
- Die Feldverschlüsselung deckt zwei Spalten ab; keine KMS-Integration.
- Der Zustand von Ratenbegrenzung und Cache braucht Redis, um mit mehreren Replikaten korrekt zu sein.
- Insight-Ansichten werden nach Rolle maskiert, aber je Token des Aufrufers 30 s lang zwischengespeichert.
- Die erneute Authentifizierung vor zerstörerischen Aktionen erfordert Arbeit beim Identitätsanbieter.

## Empfohlene technische Reihenfolge

1. KMS-gestützte Feldschlüssel und breitere Abdeckung der Felder (Rest von T41).
2. Ein erstes Release taggen, um schlüssellose Signierung und Verifikation zu belegen (T49), und den Entwurf der
   API-Standards von einer Prüferin oder einem Prüfer kontrollieren lassen.
3. Die Header `Deprecation` und `Sunset` mit der ersten Einstellung bauen.
4. Den unabhängigen Test (T42) gegen einen Staging-Build beauftragen.
