# Datenschutzhinweis: Was in Ihrem Browser gespeichert wird

Dieser Hinweis betrifft zwei Dinge: diese Website (`contact-relationship-management.github.io`) und die
Benutzeroberfläche von Contact Relationship Management in der Form, in der dieses Repository sie ausliefert. Er listet
alles auf, was eines von beiden in Ihrem Browser speichert, und warum.

Eine Organisation, die das CRM betreibt, verarbeitet personenbezogene Daten über ihre Kontakte. Diese Verarbeitung wird
vom eigenen Datenschutzhinweis dieser Organisation abgedeckt, nicht von diesem ([spec/regulatory.md](../../../spec/regulatory.md)).

## Was gespeichert wird, und warum

Jeder Eintrag enthält eine Auswahl, die Sie getroffen haben: ein Farbschema, eine Sprache und Region, eine Schriftgröße,
eine Zeitzone oder das Layout einer Tabelle. Keiner davon enthält Ihren Namen, Ihre E-Mail-Adresse, Ihre IP-Adresse oder
irgendeine Kennung, und keiner wird verwendet, um Sie zu verfolgen, zu vermessen oder Ihnen Werbung zu zeigen. Jeder
existiert nur, damit die nächste Seite, die Sie öffnen, dieselbe Auswahl verwendet und Sie sie nicht erneut treffen
müssen.

Solange Sie nichts auswählen, wird nichts gespeichert. Bei einem Besucher, der nichts ändert, ist nichts gespeichert.

### Auf dieser Website

| Name | Art | Inhalt | Aufbewahrung |
| --- | --- | --- | --- |
| `lily-theme` | Cookie und `localStorage` | `system` („Diesem Gerät folgen“) oder der Name eines Farbschemas, etwa `dark` | Cookie: 400 Tage. `localStorage`: bis Sie ihn löschen |
| `lily-text-size` | `localStorage` | Die von Ihnen gewählte Schriftgröße | Bis Sie sie löschen |
| `lily-time-zone` | `localStorage` | Die von Ihnen gewählte Zeitzone | Bis Sie sie löschen |

Das Farbschema wird in einem Cookie und in `localStorage` gespeichert, damit es erhalten bleibt, falls Ihr Browser eines
von beiden löscht. Die Seiten der Website sind statische Dateien, und kein Programm auf der Website liest das Cookie. Da
es ein Cookie ist, sendet Ihr Browser es bei jeder Anfrage an die Website an GitHub Pages, das die Website hostet.

Sprache und Region der Website sind Teil der Seitenadresse, etwa `/de-at/`; dafür wird also nichts gespeichert.

### In der Benutzeroberfläche des CRM

| Name | Art | Inhalt | Aufbewahrung |
| --- | --- | --- | --- |
| `crm-theme` | Cookie | `system` („Diesem Gerät folgen“) oder der Name eines Farbschemas | 400 Tage |
| `mxi.crm.locale` | `localStorage` | Die von Ihnen gewählte Sprache und Region, etwa `de-at` | Bis Sie sie löschen |
| `lily-text-size` | `localStorage` | Die von Ihnen gewählte Schriftgröße | Bis Sie sie löschen |
| `crm-grid-<table>` | `localStorage` | Spaltenbreiten, ausgeblendete Spalten und Sortierung einer Tabelle | Bis Sie sie löschen |
| `crm-grid-order-<table>` | `localStorage` | Spaltenreihenfolge einer Tabelle | Bis Sie sie löschen |
| `__Host-mxi_session` | Cookie | Eine zufällige Kennung Ihrer angemeldeten Sitzung | Bis Sie sich abmelden oder die Sitzung endet |

Der Server des CRM liest das Cookie `crm-theme`, damit er jede Seite in Ihrem Farbschema senden kann und die Seite nicht
zuerst in einem anderen erscheint. Das CRM liest es für keinen anderen Zweck.

Das Sitzungscookie ist keine Auswahl. Es wird beim Anmelden gesetzt, Skripte des Browsers können es nicht lesen
(`HttpOnly`), und ohne es können Sie nicht angemeldet bleiben.

## Wer es erhält

Diese Einträge bleiben in Ihrem Browser. Nichts davon wird an Dritte gesendet, abgesehen vom Farbschema-Cookie der
Website, das Ihr Browser wie oben beschrieben an den Host der Website sendet. Die Cookies sind Erstanbieter-Cookies
(`SameSite=Lax`) und auf einer `https`-Seite `Secure`.

## Wie Sie es entfernen

Um eine Auswahl zu ändern, treffen Sie sie erneut. Um alles zu entfernen, löschen Sie die Daten dieser Website in den
Einstellungen Ihres Browsers: Die nächste Seite verwendet dann wieder die Standardwerte, das heißt, sie folgt der
Hell- oder Dunkel-Einstellung Ihres Geräts, der Standardschriftgröße und der Standardsprache der Seite.
