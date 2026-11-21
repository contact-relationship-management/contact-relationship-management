# Testeur de sécurité indépendant

> Note technique, et non avis juridique, certification ou audit. Elle indique
> ce que montre le dépôt au 2026-10-11. Rien ici ne revendique une accréditation.

## Pourquoi cette page existe

Chaque contrôle de sécurité de ce projet a été construit et testé par ses auteurs
(CRM-T42). Cela montre que les contrôles fonctionnent comme les auteurs les ont compris. Cela ne
montre pas que rien ne manque. Les hôpitaux et les acheteurs publics exigent généralement
un test par une personne indépendante avant de conserver des données réelles, et l'affirmation honnête
d'ici là est « construit selon une norme élevée, non testé par des tiers »
([fiche du directeur général](../directeur-général/index.md),
[fiche de sécurité](../directeur-de-la-sécurité/index.md)).

Cette page est ce qu'il faut remettre à un cabinet de tests, et ce qu'on doit en attendre.

## Ce que signifie réussir, et ce que cela ne signifie pas

Un rapport sans réserve dit qu'une équipe nommée, en un temps déterminé, avec un périmètre
déterminé, n'a trouvé aucun problème grave. Ce n'est **pas** une certification, ni une accréditation, ni une
garantie. Ne décrivez pas le système comme « sûr », « certifié » ou « soumis à des tests d'intrusion
selon une norme » au-delà de ce que dit le rapport
([fiche GRC](../gouvernance-risques-conformité/index.md)).

## Choisir le testeur

- Indépendant des auteurs et de quiconque a configuré le déploiement.
- Composé de testeurs nommés et expérimentés, avec l'accréditation pertinente pour votre
  marché lorsqu'il y en a une (par exemple CREST, ou le schéma CHECK du NCSC pour
  les travaux du gouvernement britannique). Demandez quelle accréditation couvre les personnes qui font
  le travail, et pas seulement l'entreprise.
- Accepte par écrit le périmètre, les règles d'engagement, la confidentialité, le traitement
  des données et la responsabilité avant tout test. Le contrat est à convenir par le
  service juridique ([fiche du directeur juridique](../directeur-juridique/index.md)).

## Quoi tester

| Domaine | Ce que les testeurs doivent essayer | Où la conception est décrite |
|---|---|---|
| Authentification | Jetons falsifiés, expirés, rejoués et d'audience erronée ; rétrogradation vers le non authentifié ; la vérification de démarrage qui refuse de s'exécuter sans authentification | [auth](../../../../spec/auth.md) |
| Autorisation | Atteindre des enregistrements ou des actions qu'un rôle ne devrait pas : escalade horizontale et verticale, contrôles au niveau de l'enregistrement, actions destructives, lectures réservées à l'administration (audit, conservation, revue du bris de glace) | [auth](../../../../spec/auth.md) |
| Exposition des données | Valeurs masquées qui fuient par les listes, les lectures de détail groupées, les analyses, les agrégats, les messages d'erreur, le cache ou la clé de cache | [tasks](../../../../spec/tasks.md) (T32, T47) |
| Accès d'urgence | Prendre, prolonger ou détourner le bris de glace ; revoir sa propre autorisation ; revoir deux fois | [auth](../../../../spec/auth.md) (T44) |
| Audit | Altérer, supprimer ou réordonner l'historique ; contourner la chaîne ; masquer des lectures | [audit](../../../../spec/audit.md) |
| Résistance aux abus | Contournement de la limitation de débit et du verrouillage, verrouillage d'autres utilisateurs, usurpation d'en-têtes pour la clé du client | T43 |
| Traitement des entrées | Injection (SQL, JSON, en-têtes), corps surdimensionnés, curseurs et clés d'idempotence malformés, rejeu ou collision de clés entre appelants | T51, T52 |
| Interface web | Cross-site scripting, falsification de requêtes, gestion des sessions, le proxy backend pour frontend, politique de sécurité du contenu, détournement de clic | `hooks.server.ts` de l'interface |
| Infrastructure telle que livrée | Images de conteneur, ports exposés, accessibilité de Redis et PostgreSQL, gestion des secrets, valeurs de configuration par défaut | [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Chaîne d'approvisionnement | Provenance des dépendances et des images, les SBOM, compilations non signées | [control-mapping](../../../../spec/compliance/control-mapping.md) |
| Logique métier | Contournement du consentement dans le circuit d'envoi, effacement qui laisse des données, manipulation du score, création en double | [regulatory](../../../../spec/regulatory.md) |

**Hors périmètre sauf accord contraire :** le service d'identité qui émet
les jetons (il est hors de ce dépôt), la plateforme d'hébergement et le déni de service
à grand volume.

## Ce qu'il faut remettre aux testeurs

1. Une **version de préproduction** qui correspond à ce qui tournera en production : configuration
   de production, authentification activée, TLS, de vrais Redis et PostgreSQL. Ne testez pas
   les valeurs par défaut de la démonstration.
2. **Des données synthétiques uniquement.** La tâche d'amorçage construit un portefeuille d'affaires sans personnes
   réelles. Des données réelles de patients ou de citoyens ne doivent jamais se trouver dans un environnement de test.
3. **Un compte par rôle**, afin de tester l'autorisation de chaque côté. La
   politique de référence est `config/abac-policy.reference.json`.
4. Les documents liés ci-dessus, le document OpenAPI à `/api-docs/openapi.json`
   et un accès en lecture au dépôt, pour que le test puisse combiner boîte noire et revue
   de code.
5. Les **faiblesses connues**, afin que les testeurs consacrent leur temps à ce qui n'est pas déjà
   écrit :
   - la signature des versions est construite mais n'a pas été exécutée sur une vraie version (T49) ;
   - le chiffrement de champ couvre deux colonnes avec des clés locales, sans KMS ;
   - la réauthentification renforcée n'est pas construite ;
   - le bris de glace ne peut pas être révoqué, seulement expirer, et aucune alerte n'est branchée ;
   - les rechargements de politique apparaissent dans les analyses en cache jusqu'à 30 secondes ;
   - la déconnexion sur inactivité se fait côté client ;
   - l'état de la limitation de débit revient à un compte par réplique si Redis tombe.
   Demandez-leur de confirmer ou de réfuter chacune, et non de les ignorer.

## Règles d'engagement à convenir par écrit

- Testeurs, dates et adresses sources nommés ; un contact joignable 24 h sur 24 de votre côté.
- Ce qui peut être tenté (par exemple, ne pas détruire le magasin de données de test
  sans préavis) et ce qui doit s'arrêter à la preuve de l'exposition.
- Qui est informé immédiatement d'un constat critique, et comment, avant le rapport final.
- Comment le rapport, les preuves et toute note des testeurs sont stockés et détruits.

## Ce que vous devez recevoir en retour

- Un rapport avec périmètre, méthode, dates, noms des testeurs et accréditation.
- Chaque constat avec gravité, preuves, étapes de reproduction, composant concerné
  et correctif recommandé.
- Une déclaration claire de ce qui n'a **pas** été testé.
- Un nouveau test des constats corrigés, inclus dans la mission ou chiffré.

## Après le rapport

1. Trier chaque constat dans `spec/tasks.md` comme une tâche avec un responsable et une date.
2. Corriger les constats critiques et élevés avant toute donnée réelle, puis demander aux testeurs
   de confirmer les corrections.
3. Ajouter un test de non-régression pour chaque correction. La règle du projet est de ne pas affaiblir un test
   ni une barrière pour faire passer quelque chose.
4. Consigner honnêtement le résultat : la date du rapport, le périmètre, les constats et
   leur état. Mettre à jour le plan de préparation ([plan](../../../../spec/plan.md)) et
   le dossier de conformité ([conformité](../../../../spec/compliance/index.md)).
5. Retester après tout changement important, et selon un calendrier que fixe votre responsable des risques.
   Un test est un instantané, non un état.

## Ordre des travaux

1. Fermer d'abord ce qui est peu coûteux et connu, pour que les testeurs ne dépensent pas le budget
   à le redécouvrir : une première version signée (T49), une alerte sur le bris de glace
   et sur les échecs de vérification de l'audit, Redis derrière TLS et des identifiants.
2. Préparer la version de préproduction, les comptes et les données d'amorçage.
3. Convenir du périmètre, du contrat et des règles d'engagement.
4. Tester ; recevoir immédiatement les constats critiques.
5. Corriger, retester et consigner.
6. Seulement ensuite, mettre à jour les affirmations externes, et uniquement selon ce que le rapport étaye.

## Décisions qui vous reviennent

1. Qui commande et qui est responsable des constats (voir la
   [fiche du directeur des risques](../directeur-des-risques/index.md)).
2. Quelle accréditation votre marché attend des testeurs.
3. Le budget et la date, fixés par ce que coûte un retard et non par ce qui est
   commode.
4. Tester une seule fois avant le lancement ou selon un calendrier récurrent.
