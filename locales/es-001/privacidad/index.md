# Aviso de privacidad: qué se guarda en su navegador

Este aviso abarca dos cosas: este sitio web (`contact-relationship-management.github.io`) y la interfaz de usuario de
Gestión de Relaciones con Contactos tal como la distribuye este repositorio. Enumera todo lo que cualquiera de los
dos guarda en su navegador y por qué.

Una organización que ejecuta el CRM trata datos personales de sus contactos. Ese tratamiento lo cubre el aviso de
privacidad de esa organización, no este ([spec/regulatory.md](../../../spec/regulatory.md)).

## Qué se guarda y por qué

Cada elemento contiene una elección que usted hizo: un tema, un idioma y configuración regional, un tamaño de texto,
una zona horaria o la disposición de una tabla. Ninguno contiene su nombre, correo electrónico, dirección IP ni
identificador alguno, y ninguno se usa para rastrearlo, medirlo ni mostrarle publicidad. Cada uno existe para que la
siguiente página que abra use la misma elección y no tenga que volver a hacerla.

No se guarda nada hasta que usted hace una elección. Un visitante que no cambia nada no tiene nada guardado.

### En este sitio web

| Nombre | Tipo | Qué contiene | Duración |
| --- | --- | --- | --- |
| `lily-theme` | Cookie y `localStorage` | `system` («Seguir este dispositivo») o el nombre de un tema, como `dark` | Cookie: 400 días. `localStorage`: hasta que lo borre |
| `lily-text-size` | `localStorage` | El tamaño de texto que eligió | Hasta que lo borre |
| `lily-time-zone` | `localStorage` | La zona horaria que eligió | Hasta que lo borre |

El tema se guarda en una cookie y en `localStorage` para que siga ahí si el navegador borra uno de los dos. Las
páginas del sitio son archivos estáticos y ningún programa del sitio lee la cookie. Por ser una cookie, su navegador la
envía a GitHub Pages, que aloja el sitio, con cada solicitud al sitio.

El idioma y la configuración regional del sitio forman parte de la dirección de la página, como `/de-de/`, por lo que
no se guarda nada para ellos.

### En la interfaz de usuario del CRM

| Nombre | Tipo | Qué contiene | Duración |
| --- | --- | --- | --- |
| `crm-theme` | Cookie | `system` («Seguir este dispositivo») o el nombre de un tema | 400 días |
| `mxi.crm.locale` | `localStorage` | El idioma y la configuración regional que eligió, como `de-de` | Hasta que lo borre |
| `lily-text-size` | `localStorage` | El tamaño de texto que eligió | Hasta que lo borre |
| `crm-grid-<table>` | `localStorage` | Anchos de columna, columnas ocultas y orden de una tabla | Hasta que lo borre |
| `crm-grid-order-<table>` | `localStorage` | El orden de las columnas de una tabla | Hasta que lo borre |
| `__Host-mxi_session` | Cookie | Un identificador aleatorio de su sesión iniciada | Hasta que cierre sesión o termine la sesión |

El servidor del CRM lee la cookie `crm-theme` para enviar cada página con su tema y que la página no aparezca primero
con otro. El CRM nunca la lee para ningún otro fin.

La cookie de sesión no es una elección. Se establece al iniciar sesión, los scripts del navegador no pueden leerla
(`HttpOnly`) y sin ella no podría mantener la sesión iniciada.

## Quién lo recibe

Estos elementos permanecen en su navegador. Nada de ello se envía a terceros, salvo la cookie de tema del sitio, que su
navegador envía al alojamiento del sitio como se describió arriba. Las cookies son propias (`SameSite=Lax`) y
`Secure` en una página `https`.

## Cómo eliminarlo

Para cambiar una elección, vuelva a hacerla. Para eliminar todo, borre los datos de este sitio en la configuración de
su navegador: la siguiente página volverá a usar los valores predeterminados, es decir, seguir el ajuste claro u
oscuro de su dispositivo, el tamaño de texto estándar y el idioma predeterminado de la página.
