# Privacy notice: what is kept in your browser

This notice covers two things: this website (`contact-relationship-management.github.io`), and the Contact
Relationship Management user interface as this repository ships it. It lists everything either one stores in your
browser, and why.

An organization that runs the CRM processes personal data about its contacts. That processing is covered by that
organization's own privacy notice, not this one ([spec/regulatory.md](../spec/regulatory.md)).

## What is stored, and why

Each item holds a choice that you made: a theme, a language and locale, a text size, a time zone, or the layout of
a table. None of them holds your name, email address, IP address or any identifier, and none of them is used to
track you, measure you or show you advertising. Each one exists so that the next page you open uses the same
choice, and you do not have to make it again.

Nothing is stored until you make a choice. A visitor who changes nothing has nothing stored.

### On this website

| Name | Kind | What it holds | Kept for |
| --- | --- | --- | --- |
| `lily-theme` | Cookie and `localStorage` | `system` ("Follow this device") or the name of a theme, such as `dark` | Cookie: 400 days. `localStorage`: until you clear it |
| `lily-text-size` | `localStorage` | The text size you picked | Until you clear it |
| `lily-time-zone` | `localStorage` | The time zone you picked | Until you clear it |

The theme is kept in a cookie and in `localStorage`, so that it is still there if your browser clears one of them.
The site's pages are static files, and no program on the site reads the cookie. Because it is a cookie, your browser
sends it to GitHub Pages, which hosts the site, with each request to the site.

The site's language and locale are part of the page address, such as `/de-de/`, so nothing is stored for them.

### In the CRM user interface

| Name | Kind | What it holds | Kept for |
| --- | --- | --- | --- |
| `crm-theme` | Cookie | `system` ("Follow this device") or the name of a theme | 400 days |
| `mxi.crm.locale` | `localStorage` | The language and locale you picked, such as `de-de` | Until you clear it |
| `lily-text-size` | `localStorage` | The text size you picked | Until you clear it |
| `crm-grid-<table>` | `localStorage` | One table's column widths, hidden columns and sort order | Until you clear it |
| `crm-grid-order-<table>` | `localStorage` | One table's column order | Until you clear it |
| `__Host-mxi_session` | Cookie | A random identifier for your signed-in session | Until you sign out or the session ends |

The CRM's server reads the `crm-theme` cookie, so that it can send each page in your theme and the page does not
first appear in another one. The CRM never reads it for any other purpose.

The session cookie is not a choice. It is set when you sign in, the browser's scripts cannot read it (`HttpOnly`),
and without it you cannot stay signed in.

## Who receives it

These items stay in your browser. Nothing in them is sent to a third party, apart from the site's theme cookie,
which your browser sends to the site's host as described above. Cookies are first party (`SameSite=Lax`), and
`Secure` on an `https` page.

## How to remove it

To change a choice, make it again. To remove everything, clear this site's data in your browser's settings: the
next page then uses the defaults again, which means following your device's light or dark setting, the standard
text size and the page's default language.
