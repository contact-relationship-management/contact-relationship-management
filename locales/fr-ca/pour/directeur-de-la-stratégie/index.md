# Pour le directeur de la stratégie

> Note technique, et non avis juridique, certification ou audit. Elle indique
> ce que montre le dépôt au 2026-10-11. Rien ici ne revendique une accréditation.

## La question stratégique

À quoi sert-il ? Aujourd'hui, c'est un **CRM de référence guidé par la spécification**, doté d'une
couche de sécurité, de confidentialité et d'audit exceptionnellement complète. Ce n'est pas un produit avec
des clients, et il ne concurrence pas les CRM matures
([COMPARISONS.md](../../../../COMPARISONS.md)). Le choix qui vous est offert est de
le garder comme référence, ou d'investir pour le rendre déployable dans des
contextes réglementés.

## Ce qui est réellement différent

| Atout | Pourquoi cela pourrait compter | Solidité |
|---|---|---|
| La spécification d'abord : une seule spécification pilote le service Rust et l'interface Svelte ([spec](../../../../spec/index.md)) | Moins coûteux à auditer, porter et garder cohérent | Réel ; les tests citent la spécification |
| Marketing fondé sur le consentement, appliqué dans le circuit d'envoi | Une exigence stricte pour les acheteurs de la santé et du secteur public | Réel, mais aucun vrai expéditeur n'existe encore (T39) |
| Score de prospects et prévisions explicables fondés sur des règles | Acheteurs qui doivent justifier des décisions concernant des personnes | Réel ; volontairement sans apprentissage automatique |
| Contacts consommés depuis un service d'identité, non dupliqués | Moins de données personnelles conservées, surface de fuite réduite | Réel ; dépend de l'existence de ce service |
| Audit infalsifiable, masquage par rôle, accès d'urgence motivé | Les contrôles dont s'enquièrent d'abord les acheteurs réglementés | Mis en œuvre et testé ; non vérifié de façon indépendante |

## Ce qui n'est pas différent

L'étendue des fonctionnalités. Les devis, les contrats, la téléphonie, le score par apprentissage automatique et
les paiements sont hors périmètre ([scope](../../../../spec/scope.md)). Quiconque choisit un
CRM sur ses fonctionnalités en choisira un autre.

## Options

| Option | Ce que cela signifie | Coût et risque | Preuves nécessaires |
|---|---|---|---|
| A. Rester une référence | Le garder comme bibliothèque d'enseignement et de modèles | Le plus faible ; la valeur est une question de réputation | Rien de plus qu'aujourd'hui |
| B. Niche des secteurs réglementés | Terminer les points ouverts et le vendre ou l'assister auprès d'organismes de santé et publics | Le plus élevé ; exige un fournisseur, des conditions juridiques, des tests externes, une assurance continue | Test indépendant (T42), audit d'accessibilité (T48), vrai courriel (T39) |
| C. Composants, non produit | Proposer la chaîne d'audit, le masquage et les modèles de bris de glace comme éléments réutilisables | Modéré ; exige conditionnement et documentation | Travail d'extraction ; aucune nouvelle certification |

**Recommandation : faire A maintenant, et financer le test le moins cher de B avant de s'engager.**
L'évaluation du NHS a conclu que le produit sort probablement du périmètre du DTAC
([évaluation](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md)),
la voie vers le marché n'est donc pas arrêtée. Dépensez d'abord pour découvrir quel acheteur
a besoin de cela et ce qu'il exigerait, et non pour des certifications que personne n'a demandées.

## Feuille de route, dans l'ordre que suggèrent les preuves

1. Test de sécurité indépendant et audit manuel d'accessibilité (T42, T48) : ils
   décident si B est seulement possible.
2. Vrai fournisseur de courriel (T39) une fois qu'un fournisseur et des conditions de traitement des données sont
   choisis ; l'interface d'expéditeur et un circuit SMTP existent et fonctionnent contre un serveur de test local, avec un lien de désabonnement signé, une liste de suppression et un webhook générique de résultat signé, mais aucun fournisseur n'est encore intégré ([feuille de route](../../../../spec/roadmap.md)).
3. Prouver la signature des versions avec une première version étiquetée (T49) : condition minimale pour
   les achats. Le flux est construit ; il n'a pas été exécuté.
4. Seulement ensuite, les fonctionnalités produit de la feuille de route (attribution, territoires,
   horloges de niveau de service en heures ouvrées, portail public de base de connaissances).

## Risques stratégiques

- **Affirmer plus que la vérité.** Le moyen le plus rapide de perdre un acheteur réglementé est une
  affirmation de conformité que les preuves ne soutiennent pas. Voir la [fiche GRC](../gouvernance-risques-conformité/index.md).
- **Maintenance.** Le travail de sécurité ne finit jamais : mises à jour des dépendances, actualisation
  des empreintes, rotation des clés, nouveaux tests. L'option B vous y engage.
- **Dépendance à un service d'identité** extérieur à ce dépôt.
- **Un seul mainteneur.** Le projet a un seul mainteneur et indique que la profondeur de
  revue est celle d'une seule personne ([MAINTAINERS.md](../../../../MAINTAINERS.md),
  [GOVERNANCE.md](../../../../GOVERNANCE.md)). L'option B exige plus d'une personne
  capable de répondre du travail de sécurité.

## Décisions qui vous reviennent

1. Laquelle de A, B ou C, et la date de revue de ce choix.
2. Qui est le premier acheteur visé, trouvé par la conversation avant de dépenser.
3. Financer ou non dès maintenant une assurance indépendante (voir la [fiche du directeur général](../directeur-général/index.md)).
