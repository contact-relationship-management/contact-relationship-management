# Für die Betriebsleitung (COO)

> Technische Übersicht, keine Rechtsberatung, Zertifizierung oder Prüfung. Sie nennt,
> was das Repository zum Stand 2026-10-11 zeigt. Nichts hier erhebt den Anspruch einer Akkreditierung.

## Was der Betrieb erfordert

Der Dienst ist ein Rust-Prozess je Replikat plus PostgreSQL 18 und Redis; das Front-End ist ein Node-Prozess. Beide
werden als gehärtete Container-Images ohne Root-Rechte ausgeliefert, mit per Digest festgelegten Basis-Images. Das
Betriebshandbuch ist [operations-runbook.md](../../../../spec/compliance/operations-runbook.md).

## Betriebliche Fakten, nach denen Sie planen sollten

- **Sicherer Ausfall beim Start.** In Produktion weigert sich der Dienst zu starten, wenn die Authentifizierung aus ist,
  wenn die Audit-Kette unbrauchbar ist oder wenn erforderliche Schlüssel und
  `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL` fehlen. Ein falsch konfigurierter Einsatz hält an, statt offen zu laufen.
- **Health und Metriken.** `/_health/live`, `/_health/ready` und `/_health/startup` unterliegen nie der Ratenbegrenzung,
  sodass eine Sperre keinen gesunden Pod neu starten kann. Metriken unter `/metrics.prom`.
- **Missbrauchsgrenzen** sind in Produktion aktiv: 600 Anfragen pro Minute und Client und eine Sperre nach 10
  fehlgeschlagenen Anmeldungen in 5 Minuten. Sie werden über Redis zwischen Replikaten geteilt; fällt Redis aus, greifen
  Zähler je Replikat.
- **Notfallzugriff** (Break-glass): Jeder angemeldete Benutzer kann eine begründete Leseerhöhung für bis zu 60 Minuten
  nehmen. Jemand anderes muss jede Freigabe prüfen. **Prozess, den Sie betreiben müssen:** Alarm bei
  Audit-Zeilen `break_glass_granted` und Prüfung der Zahl ungeprüfter Freigaben (Admin-Seite `/break-glass/review`).
- **Fehler** haben eine einheitliche Form (RFC 9457 `application/problem+json`, mit erhaltenen `error` und
  `description`), sodass Log-Analyse und Alarmregeln sich auf `status` und `type` verlassen können.
- **Wachstum des Audits.** Lesezugriffe werden auditiert, die Audit-Tabelle wächst also mit dem Verkehr. Bemessen Sie
  ihre Aufbewahrung. Ein Client, der ständig abfragt, lässt sie schnell wachsen.

## Offene betriebliche Arbeit

| Punkt | Stand |
|---|---|
| Übungen zu Sicherung und Wiederherstellung, Probe der Schlüsselrotation | Im Betriebshandbuch beschrieben; **Sie** müssen sie durchführen und belegen |
| Verankerung des Kopfes der Audit-Kette außerhalb der Datenbank | Ihre Pflicht; die API meldet den Kopf-Hash |
| Aktualisierung der festgelegten Image-Digests | Dependabot öffnet wöchentlich Pull Requests; **Sie** müssen sie prüfen und mergen, sonst veralten die Festlegungen |
| Prüfung eines signierten Releases vor dem Einsatz | Die Befehle stehen im Betriebshandbuch; per Digest ausrollen und ein Image ablehnen, das die Prüfung nicht besteht. Ungetestet bis zu einem ersten getaggten Release |
| Echter E-Mail-Anbieter (T39) | Offen; verändert das Bild von Vorfällen und Lieferanten |
| Redis hält Insight-Daten | Wie die Datenbank schützen: TLS, Passwort, keine öffentliche Erreichbarkeit |

## Bekannte Grenzen, die das Bereitschaftsteam kennen sollte

Die Abmeldung bei Inaktivität läuft im Client; ein widerrufenes Token wird sofort abgelehnt, aber ein Neuladen der
Richtlinie zeigt sich in zwischengespeicherten Insights erst nach bis zu 30 Sekunden; Break-glass kann nicht widerrufen
werden, es läuft nur ab.
