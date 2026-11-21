# Pour le directeur des risques

> Note technique, et non avis juridique, certification ou audit. Elle indique
> ce que montre le dépôt au 2026-10-11. Rien ici ne revendique une accréditation.

## Position face au risque

**Globalement : pas prêt pour des données réglementées.** Le logiciel est bien maîtrisé pour
son stade, mais les principaux risques résiduels ne sont atténués ni par des tests ni par des processus
que seule l'organisation qui l'adopte ou des tiers peuvent fournir.

## Principaux risques

| # | Risque | Base d'évaluation | Atténuation dans le dépôt | Résiduel |
|---|---|---|---|---|
| 1 | Vulnérabilités non découvertes | Aucun test d'intrusion indépendant (T42) | Tests propres, barrières de chaîne d'approvisionnement, en-têtes/CSP stricts | **Élevé** tant que non testé |
| 2 | Divulgation de champs sensibles au mauvais rôle | Règles d'accès complexes | Masquage par rôle sur les enregistrements, les agrégats et les analyses ; un test de matrice de profils ; une fuite trouvée et corrigée sur le détail d'un compte (T47) | Moyen : le masquage est testé, non vérifié de façon indépendante |
| 3 | Usage abusif de l'accès d'urgence par un initié | Le bris de glace élève les lectures | Motif écrit, plafond de 60 minutes, lecture seule, revue obligatoire par une seconde personne, audité | Moyen : aucune alerte n'est branchée ; pas de révocation |
| 4 | Piste d'audit altérée | Exige un accès à la base de données | Chaîne d'empreintes plus déclencheur en ajout seul ; signale la première rupture | Faible à moyen : l'empreinte de tête doit être ancrée hors de la base |
| 5 | Marketing illicite | Erreurs de consentement | Le circuit d'envoi applique le consentement, et non l'interface ; historique de consentement immuable | Moyen tant qu'un vrai fournisseur n'est pas intégré et revu (T39) |
| 6 | Disponibilité / abus | API publique | Limitation de débit et verrouillage, partagés via Redis | Moyen : les limites reviennent par réplique si Redis tombe |
| 7 | Non-conformité en accessibilité | Obligation du secteur public | Analyses axe automatiques ; déclaration d'accessibilité honnête | **Non quantifié** tant qu'il n'y a pas d'audit manuel (T48) |
| 8 | Chaîne d'approvisionnement | Dépendances et images | `cargo deny/audit`, `pnpm audit`, SBOM, empreintes épinglées actualisées par Dependabot, un flux de versions signé sans clé | Moyen : la signature n'a pas été exécutée sur une vraie version (T49) |
| 9 | Enregistrements en double lors de nouvelles tentatives | Les clés d'idempotence sont facultatives dans l'API (T52) | Rejeu pour les clients qui envoient la clé ; l'interface incluse le fait toujours | Faible |
| 10 | Mauvaise classification des données | Le déployeur décide ce qu'est un « contact » | Note de périmètre et modèle d'AIPD | Dépend de vous |

## Lacunes qui restent ouvertes par nature

T39 (vrai fournisseur d'e-mail), T42 (assurance indépendante), T48 (audit manuel
d'accessibilité), T49 (une première version signée, un contrôle du projet d'API par un relecteur). Voir [tasks.md](../../../../spec/tasks.md)
et [plan.md](../../../../spec/plan.md).

## Actions de risque suggérées

Enregistrer formellement les risques 1, 7 et 10 ; exiger T42 avant toute donnée réelle ; désigner
un responsable de la revue et de l'alerte du bris de glace ; fixer la cadence d'ancrage de l'empreinte de tête de l'audit.
