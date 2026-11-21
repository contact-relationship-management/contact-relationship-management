# Für die Strategieleitung

> Technische Übersicht, keine Rechtsberatung, Zertifizierung oder Prüfung. Sie nennt,
> was das Repository zum Stand 2026-10-11 zeigt. Nichts hier erhebt den Anspruch einer Akkreditierung.

## Die strategische Frage

Wofür ist das gedacht? Heute ist es ein **Referenz-CRM, bei dem die Spezifikation an erster Stelle steht**, mit einer
ungewöhnlich vollständigen Schicht für Sicherheit, Datenschutz und Audit. Es ist kein Produkt mit Kunden und steht nicht
im Wettbewerb mit ausgereiften CRMs ([COMPARISONS.md](../../../../COMPARISONS.md)). Die Wahl, vor der Sie stehen: Soll es
eine Referenz bleiben, oder soll investiert werden, um es in regulierten Umgebungen einsetzbar zu machen?

## Was wirklich anders ist

| Stärke | Warum sie wichtig sein könnte | Wie belastbar sie ist |
|---|---|---|
| Spezifikation zuerst: Eine Spezifikation steuert sowohl den Rust-Dienst als auch die Svelte-Oberfläche ([spec](../../../../spec/index.md)) | Billiger zu prüfen, zu portieren und konsistent zu halten | Real; Tests verweisen auf die Spezifikation |
| Marketing, das mit der Einwilligung beginnt, im Sendeweg durchgesetzt | Eine harte Anforderung für Käufer im Gesundheitswesen und im öffentlichen Sektor | Real, aber es gibt noch keinen echten Sender (T39) |
| Nachvollziehbares, regelbasiertes Lead-Scoring und Prognosen | Käufer, die Entscheidungen über Menschen begründen müssen | Real; absichtlich kein maschinelles Lernen |
| Kontakte werden aus einem Identitätsdienst bezogen, nicht dupliziert | Weniger gehaltene personenbezogene Daten, kleinere Angriffsfläche bei Verletzungen | Real; hängt davon ab, dass dieser Dienst existiert |
| Manipulationssicheres Audit, Maskierung nach Rolle, begründeter Notfallzugriff | Die Kontrollen, nach denen regulierte Käufer zuerst fragen | Umgesetzt und getestet; nicht unabhängig verifiziert |

## Was nicht anders ist

Der Funktionsumfang. Angebote, Verträge, Telefonie, Scoring mit maschinellem Lernen und Zahlungen liegen außerhalb des
Umfangs ([scope](../../../../spec/scope.md)). Wer ein CRM nach Funktionen auswählt, wählt ein anderes.

## Optionen

| Option | Was sie bedeutet | Kosten und Risiko | Benötigte Nachweise |
|---|---|---|---|
| A. Referenz bleiben | Als Bibliothek zum Lernen und für Muster behalten | Am niedrigsten; der Wert ist ein Ansehensgewinn | Nichts über das Heutige hinaus |
| B. Nische im regulierten Sektor | Die offenen Punkte abschließen und es an Stellen im Gesundheitswesen und im öffentlichen Sektor verkaufen oder unterstützen | Am höchsten; braucht einen Anbieter, rechtliche Bedingungen, externe Tests, laufende Absicherung | Unabhängiger Test (T42), Barrierefreiheits-Audit (T48), echte E-Mail (T39) |
| C. Komponenten, nicht Produkt | Die Muster für Audit-Kette, Maskierung und Break-glass als wiederverwendbare Bausteine anbieten | Mäßig; braucht Paketierung und Dokumentation | Arbeit zur Extraktion; keine neue Zertifizierung |

**Empfehlung: jetzt A tun und den günstigsten Test von B finanzieren, bevor Sie sich festlegen.** Die NHS-Bewertung kam
zu dem Schluss, dass das Produkt wahrscheinlich außerhalb des DTAC-Anwendungsbereichs liegt
([Bewertung](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md)),
der Weg zum Markt ist also nicht geklärt. Investieren Sie zuerst darin herauszufinden, welcher Käufer das braucht und was
er verlangen würde, nicht in Zertifizierungen, nach denen niemand gefragt hat.

## Fahrplan, in der Reihenfolge, die die Nachweise nahelegen

1. Unabhängiger Sicherheitstest und manuelles Barrierefreiheits-Audit (T42, T48): Sie entscheiden, ob B überhaupt
   möglich ist.
2. Echter E-Mail-Anbieter (T39), sobald ein Anbieter und Bedingungen zur Auftragsverarbeitung gewählt sind; die
   Sender-Schnittstelle und ein SMTP-Weg existieren und laufen gegen einen lokalen Testserver, mit signiertem
   Abmeldelink, Sperrliste und einem generischen signierten Webhook für Ergebnisse, aber es ist noch kein Anbieter
   eingebunden ([roadmap](../../../../spec/roadmap.md)).
3. Die Signierung von Releases mit einem ersten getaggten Release belegen (T49): Grundvoraussetzung für die Beschaffung.
   Der Workflow ist gebaut; er lief noch nicht.
4. Erst dann die Produktfunktionen der Roadmap (Attribution, Gebiete, SLA-Uhren nach Geschäftszeiten, öffentliches Portal
   der Wissensdatenbank).

## Strategische Risiken

- **Mehr behaupten, als wahr ist.** Der schnellste Weg, einen regulierten Käufer zu verlieren, ist eine Konformitätsaussage,
  die die Nachweise nicht stützen. Siehe die [Übersicht zu GRC](../governance-risiko-compliance/index.md).
- **Wartung.** Sicherheitsarbeit endet nie: Aktualisierung von Abhängigkeiten, Erneuerung von Digests, Schlüsselrotation,
  erneutes Testen. Option B verpflichtet Sie dazu.
- **Abhängigkeit von einem Identitätsdienst**, der außerhalb dieses Repositorys liegt.
- **Ein Maintainer.** Das Projekt hat einen einzigen Maintainer und sagt, dass die Tiefe der Prüfung die einer Person ist
  ([MAINTAINERS.md](../../../../MAINTAINERS.md), [GOVERNANCE.md](../../../../GOVERNANCE.md)). Option B braucht mehr als
  eine Person, die für die Sicherheitsarbeit einstehen kann.

## Entscheidungen für Sie

1. Welche von A, B oder C, und das Datum, an dem diese Wahl überprüft wird.
2. Wer der erste Zielkäufer ist, durch Gespräche ermittelt, bevor Geld ausgegeben wird.
3. Ob unabhängige Absicherung jetzt finanziert wird (siehe die [Übersicht für die Geschäftsführung](../geschäftsführer/index.md)).
