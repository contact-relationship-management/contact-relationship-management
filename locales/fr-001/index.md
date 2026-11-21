# Gestion de la relation client

Logiciel CRM libre pour la vente, le marketing et le support client.

Contact Relationship Management suit les relations avec les clients et les prospects à travers la vente, le marketing et le support. Il conserve une fiche unique par contact et exige un consentement enregistré avant l'envoi de tout message. Le logiciel n'est pas prêt pour la production : il fonctionne avec des données synthétiques.

## Avantages

- **Fiche contact unique.** La vente, le marketing, le support et l'analytique utilisent la même fiche.
- **Consentement enregistré.** Rien n'est envoyé à un contact sans consentement enregistré. Chaque e-mail comporte un lien de désabonnement signé.
- **Moins de données personnelles.** Le CRM conserve une référence à une personne dans un service d'identité, et non une copie du nom ou de l'adresse.
- **Résultats explicables.** Les scores de prospects sont calculés à partir de règles lisibles. Les prévisions sont calculées à partir d'étapes d'entonnoir pondérées.
- **Conception internationale.** Les noms, les adresses, les calendriers et les sens d'écriture ne sont pas liés au format d'un seul pays. L'interface est disponible en huit langues.
- **Spécification publique.** La spécification, le code et les tests sont publics. La documentation indique ce qui est construit et ce qui ne l'est pas.

## Fonctionnalités

- **Vente.** Score de prospects fondé sur des règles, entonnoirs d'affaires en tableau et prévisions pondérées par étape.
- **Marketing.** Campagnes segmentées et séquences de maturation. L'envoi est simulé tant qu'un mode de messagerie n'est pas configuré.
- **Support.** Tickets avec priorités et échéances de niveau de service, et articles de base de connaissances.
- **Analytique.** Tableaux de bord pour le taux de réussite, l'entonnoir par étape, le retour des campagnes, la santé des niveaux de service et la valeur vie client.
- **Qualité des données.** Détection de doublons par probabilité, avec fusion et annulation.
- **Gouvernance.** Journal d'audit, accès fondé sur les rôles et prise en charge des demandes d'accès et d'effacement des personnes concernées.

## Pour commencer

1. **Récupérer le code.** Clonez le dépôt. Node.js et pnpm sont requis.
2. **Démarrer l'application.** Dans le répertoire Svelte, exécutez `pnpm install`, puis `pnpm dev`.
3. **Se connecter.** Utilisez `example@example.com`. Aucun autre service n'est nécessaire.
4. **Suivre la visite guidée.** La visite couvre les contacts, les prospects, les affaires, les campagnes, les tickets et la vue de la direction, avec des données fictives.

Le [tutoriel](../../docs/tutorial-first-tour.md) décrit chaque étape. [Installer et utiliser](../../INSTALL.md) couvre le service complet.

## Comparaison

| Projet | Catégorie | Différence |
| --- | --- | --- |
| [Twenty](../../docs/compare/twenty.md) | CRM libre | Produit complet. Ce projet est une démonstration plus petite, guidée par la spécification. |
| [EspoCRM](../../docs/compare/espocrm.md) | CRM libre | Fonctionnalités plus larges. Ce projet met l'accent sur des règles explicables. |
| [SuiteCRM](../../docs/compare/suitecrm.md) | CRM libre | Mature et volumineux. Ce projet est une référence compacte. |
| [Salesforce](../../docs/compare/salesforce.md), [HubSpot](../../docs/compare/hubspot.md) | CRM commercial en SaaS | Ce projet modélise la même pratique. Aucun code ni aucune donnée ne sont partagés. |

Ce projet est une démonstration et ne concurrence pas ces produits. Voir [toutes les comparaisons](../../COMPARISONS.md).

## Fiches par rôle

Les [fiches par rôle](pour/) offrent une page pour chaque poste de direction : ce que fait le logiciel, ce qu'il ne fait pas et quelles décisions relèvent de ce rôle.

## État

Ce logiciel n'est pas prêt pour la production ([chemin vers la production](chemin-vers-la-production/index.md)). Utilisez uniquement des données synthétiques. Aucun vrai e-mail n'est envoyé tant qu'un vrai mode de messagerie n'est pas configuré. Rien sur ce site ne constitue un avis juridique.
