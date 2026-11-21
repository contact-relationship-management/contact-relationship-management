# Informativa sulla privacy: che cosa viene conservato nel browser

Questa informativa riguarda due cose: questo sito web (`contact-relationship-management.github.io`) e l'interfaccia
utente di Contact Relationship Management così come la distribuisce questo repository. Elenca tutto ciò che l'uno o
l'altra memorizzano nel browser, e perché.

Un'organizzazione che gestisce il CRM tratta dati personali sui propri contatti. Tale trattamento è coperto
dall'informativa sulla privacy di quell'organizzazione, non da questa ([spec/regulatory.md](../../../spec/regulatory.md)).

## Che cosa viene memorizzato e perché

Ogni elemento contiene una scelta che avete fatto: un tema, una lingua e una impostazione locale, una dimensione del
testo, un fuso orario o la disposizione di una tabella. Nessuno di essi contiene il vostro nome, indirizzo email,
indirizzo IP o altro identificativo, e nessuno è usato per tracciarvi, misurarvi o mostrarvi pubblicità. Ognuno esiste
affinché la pagina successiva che aprite usi la stessa scelta e non dobbiate rifarla.

Nulla viene memorizzato finché non fate una scelta. Un visitatore che non cambia nulla non ha nulla di memorizzato.

### Su questo sito web

| Nome | Tipo | Che cosa contiene | Conservato per |
| --- | --- | --- | --- |
| `lily-theme` | Cookie e `localStorage` | `system` («Segui questo dispositivo») o il nome di un tema, come `dark` | Cookie: 400 giorni. `localStorage`: finché non lo cancellate |
| `lily-text-size` | `localStorage` | La dimensione del testo scelta | Finché non la cancellate |
| `lily-time-zone` | `localStorage` | Il fuso orario scelto | Finché non lo cancellate |

Il tema è conservato in un cookie e in `localStorage`, così rimane disponibile anche se il browser ne cancella uno. Le
pagine del sito sono file statici e nessun programma del sito legge il cookie. Poiché è un cookie, il browser lo invia
a GitHub Pages, che ospita il sito, a ogni richiesta al sito.

La lingua e l'impostazione locale del sito fanno parte dell'indirizzo della pagina, per esempio `/de-de/`, quindi per
esse non viene memorizzato nulla.

### Nell'interfaccia utente del CRM

| Nome | Tipo | Che cosa contiene | Conservato per |
| --- | --- | --- | --- |
| `crm-theme` | Cookie | `system` («Segui questo dispositivo») o il nome di un tema | 400 giorni |
| `mxi.crm.locale` | `localStorage` | La lingua e l'impostazione locale scelte, per esempio `de-de` | Finché non le cancellate |
| `lily-text-size` | `localStorage` | La dimensione del testo scelta | Finché non la cancellate |
| `crm-grid-<table>` | `localStorage` | Larghezza delle colonne, colonne nascoste e ordinamento di una tabella | Finché non li cancellate |
| `crm-grid-order-<table>` | `localStorage` | L'ordine delle colonne di una tabella | Finché non lo cancellate |
| `__Host-mxi_session` | Cookie | Un identificativo casuale della sessione con accesso effettuato | Fino alla disconnessione o alla fine della sessione |

Il server del CRM legge il cookie `crm-theme` per poter inviare ogni pagina già nel vostro tema, in modo che la pagina
non appaia prima in un altro. Il CRM non lo legge mai per nessun altro scopo.

Il cookie di sessione non è una scelta. Viene impostato quando accedete, gli script del browser non possono leggerlo
(`HttpOnly`) e senza di esso non potete restare connessi.

## Chi li riceve

Questi elementi restano nel browser. Nulla di ciò che contengono viene inviato a terzi, a parte il cookie del tema del
sito, che il browser invia all'host del sito come descritto sopra. I cookie sono di prima parte (`SameSite=Lax`) e
`Secure` su una pagina `https`.

## Come rimuoverli

Per cambiare una scelta, basta farla di nuovo. Per rimuovere tutto, cancellate i dati di questo sito nelle impostazioni
del browser: la pagina successiva userà di nuovo i valori predefiniti, cioè seguirà l'impostazione chiara o scura del
dispositivo, la dimensione standard del testo e la lingua predefinita della pagina.
