# Avis de confidentialité : ce qui est conservé dans votre navigateur

Cet avis couvre deux choses : ce site web (`contact-relationship-management.github.io`) et l'interface utilisateur de
Contact Relationship Management telle que ce dépôt la fournit. Il énumère tout ce que l'un ou l'autre stocke dans
votre navigateur, et pourquoi.

Une organisation qui exploite le CRM traite des données personnelles sur ses contacts. Ce traitement relève de l'avis
de confidentialité de cette organisation, et non de celui-ci ([spec/regulatory.md](../../../spec/regulatory.md)).

## Ce qui est stocké, et pourquoi

Chaque élément contient un choix que vous avez fait : un thème, une langue et des paramètres régionaux, une taille de
texte, un fuseau horaire ou la disposition d'un tableau. Aucun ne contient votre nom, votre adresse e-mail, votre
adresse IP ni un identifiant, et aucun n'est utilisé pour vous suivre, vous mesurer ou vous montrer de la publicité.
Chacun existe pour que la page suivante que vous ouvrez utilise le même choix, sans que vous ayez à le refaire.

Rien n'est stocké tant que vous ne faites pas de choix. Un visiteur qui ne change rien n'a rien de stocké.

### Sur ce site web

| Nom | Type | Contenu | Conservé pendant |
| --- | --- | --- | --- |
| `lily-theme` | Cookie et `localStorage` | `system` (« Suivre cet appareil ») ou le nom d'un thème, tel que `dark` | Cookie : 400 jours. `localStorage` : jusqu'à ce que vous l'effaciez |
| `lily-text-size` | `localStorage` | La taille de texte que vous avez choisie | Jusqu'à ce que vous l'effaciez |
| `lily-time-zone` | `localStorage` | Le fuseau horaire que vous avez choisi | Jusqu'à ce que vous l'effaciez |

Le thème est conservé dans un cookie et dans `localStorage`, afin qu'il subsiste si votre navigateur en efface un.
Les pages du site sont des fichiers statiques, et aucun programme du site ne lit le cookie. Comme c'est un cookie,
votre navigateur l'envoie à GitHub Pages, qui héberge le site, avec chaque requête vers le site.

La langue et les paramètres régionaux du site font partie de l'adresse de la page, comme `/de-de/`, donc rien n'est
stocké pour eux.

### Dans l'interface utilisateur du CRM

| Nom | Type | Contenu | Conservé pendant |
| --- | --- | --- | --- |
| `crm-theme` | Cookie | `system` (« Suivre cet appareil ») ou le nom d'un thème | 400 jours |
| `mxi.crm.locale` | `localStorage` | La langue et les paramètres régionaux que vous avez choisis, comme `de-de` | Jusqu'à ce que vous l'effaciez |
| `lily-text-size` | `localStorage` | La taille de texte que vous avez choisie | Jusqu'à ce que vous l'effaciez |
| `crm-grid-<table>` | `localStorage` | Les largeurs de colonnes, les colonnes masquées et l'ordre de tri d'un tableau | Jusqu'à ce que vous l'effaciez |
| `crm-grid-order-<table>` | `localStorage` | L'ordre des colonnes d'un tableau | Jusqu'à ce que vous l'effaciez |
| `__Host-mxi_session` | Cookie | Un identifiant aléatoire de votre session ouverte | Jusqu'à ce que vous vous déconnectiez ou que la session prenne fin |

Le serveur du CRM lit le cookie `crm-theme` afin d'envoyer chaque page dans votre thème, pour que la page n'apparaisse
pas d'abord dans un autre. Le CRM ne le lit jamais à d'autres fins.

Le cookie de session n'est pas un choix. Il est défini quand vous vous connectez, les scripts du navigateur ne
peuvent pas le lire (`HttpOnly`), et sans lui vous ne pouvez pas rester connecté.

## Qui le reçoit

Ces éléments restent dans votre navigateur. Aucun n'est envoyé à un tiers, à l'exception du cookie de thème du site,
que votre navigateur envoie à l'hébergeur du site comme décrit plus haut. Les cookies sont propres au site
(`SameSite=Lax`) et `Secure` sur une page `https`.

## Comment les supprimer

Pour changer un choix, faites-le de nouveau. Pour tout supprimer, effacez les données de ce site dans les paramètres
de votre navigateur : la page suivante utilise alors de nouveau les valeurs par défaut, c'est-à-dire qu'elle suit le
réglage clair ou sombre de votre appareil, la taille de texte standard et la langue par défaut de la page.
