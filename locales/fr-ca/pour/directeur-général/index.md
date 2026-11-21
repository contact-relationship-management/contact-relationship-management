# Pour le directeur général

> Note technique, et non avis juridique, certification ou audit. Elle indique
> ce que montre le dépôt au 2026-10-11. Rien ici ne revendique une accréditation.

## La décision en un paragraphe

Il s'agit d'un CRM de niveau démonstration, doté d'une base de sécurité et de
confidentialité exceptionnellement solide. Il **n'est pas prêt** à conserver de
vraies données de patients ou de citoyens, et le dépôt le dit lui-même
([plan.md](../../../../spec/plan.md)). L'adopter, c'est décider de financer le
travail externe restant, et non de le mettre en service.

## Ce qui existe

- Un CRM fonctionnel couvrant la vente, le marketing, le support et l'analytique,
  avec une API Rust et une interface web, construit selon une spécification écrite.
- Un marketing fondé sur le consentement, un contrôle d'accès qui masque les valeurs
  sensibles aux rôles qui ne devraient pas les voir, une piste d'audit qui révèle
  toute altération, l'accès et l'effacement pour les demandes relatives à la vie
  privée, et un accès d'urgence (« bris de glace ») motivé, limité dans le temps
  et revu.
- 54 des 58 éléments de travail suivis sont terminés ([tasks.md](../../../../spec/tasks.md)).

## Ce qui sépare ce projet d'un usage réel

| Obstacle | Pourquoi c'est important | Qui peut le résoudre |
|---|---|---|
| Pas de vraie livraison de courriels (T39) | Le marketing est simulé ; un vrai fournisseur exige des contrats et un accord de traitement des données | Vous choisissez le fournisseur ; le service juridique signe |
| Pas de test de sécurité indépendant (T42) | Les hôpitaux et les acheteurs publics en exigent généralement un ; nos propres tests ne sont pas indépendants | Un cabinet extérieur |
| Pas d'audit manuel d'accessibilité (T48) | Les organismes publics doivent publier une déclaration d'accessibilité qu'ils peuvent défendre | Des personnes utilisant des technologies d'assistance |
| La signature des versions est construite mais n'a pas été exécutée sur une vraie version, et aucun relecteur n'a vérifié le projet de normes d'API (T49) | Les listes de contrôle des achats demandent les deux | L'ingénierie (étiqueter une première version) ; un relecteur extérieur |

## Décisions qui vous reviennent

1. **Quel marché d'abord.** L'évaluation selon les critères d'évaluation des
   technologies numériques du NHS ([docs](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md))
   conclut que le produit sort probablement du périmètre du DTAC et échouerait
   aux sections applicables faute de preuves organisationnelles (responsable de
   la sécurité clinique nommé, certifications). Choisissez la voie avant d'engager
   des dépenses.
2. **Financer ou non une assurance indépendante** (T42, T48). Sans elle,
   l'affirmation honnête est « construit selon une norme élevée, non testé par des tiers ».
3. **Qui porte le risque** une fois que de vraies données sont conservées. Voir les
   fiches du [directeur des risques](../directeur-des-risques/index.md) et du
   [directeur juridique](../directeur-juridique/index.md).

## Ce qu'il ne faut pas dire à l'extérieur

Ne revendiquez ni conformité, ni certification, ni « conforme à la HIPAA », ni « approuvé par le NHS ».
Le dossier de preuves ([conformité](../../../../spec/compliance/index.md)) étaye une affirmation plus
étroite et vraie : les contrôles sont cartographiés, ceux qui sont mis en œuvre sont testés, les lacunes sont nommées.
