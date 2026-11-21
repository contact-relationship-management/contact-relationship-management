# Gouvernance, risques, conformité

> Note technique, et non avis juridique, certification ou audit. Elle indique
> ce que montre le dépôt au 2026-10-11. Rien ici ne revendique une accréditation.

## Par où commencer

Le dossier de preuves est [spec/compliance](../../../../spec/compliance/index.md). Sa
règle de base est celle à laquelle tenir l'équipe : un contrôle est **Mis en œuvre** seulement
quand le code ou la configuration l'applique et qu'un test ou une vérification l'épingle ; **Déployeur**
quand votre organisation doit le fournir ; **Lacune** quand ni l'un ni l'autre n'est vrai aujourd'hui.

| Document | À utiliser pour |
|---|---|
| [control-mapping](../../../../spec/compliance/control-mapping.md) | NIST SP 800-53, règle de sécurité HIPAA, annexe A d'ISO 27001, art. 32 du RGPD, chacun marqué Mis en œuvre, Déployeur ou Lacune |
| [dpia-template](../../../../spec/compliance/dpia-template.md) | Analyse d'impact relative à la protection des données, préremplie avec les flux de données de ce système |
| [operations-runbook](../../../../spec/compliance/operations-runbook.md) | Durcissement, sauvegarde et restauration, clés, journalisation, réponse aux incidents |
| [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) | Projets de déclarations DTAC du NHS à adopter par le fabricant |
| [gds-api-review](../../../../spec/compliance/gds-api-review.md) | Autoévaluation par rapport aux normes d'API du gouvernement britannique (un projet, non confronté au texte publié) |
| [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) | Format d'erreur (RFC 9457), ce qui constitue un changement incompatible, les étapes d'obsolescence et le préavis |
| [Évaluation NHS DTAC v2.0](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md) | Résultat question par question : probablement hors périmètre du DTAC, et échouerait aux sections applicables faute de preuves organisationnelles |
| [plan](../../../../spec/plan.md) | Verdict de préparation (pas prêt) et registre des lacunes |
| [tasks](../../../../spec/tasks.md) | La tâche de chaque lacune, sa résolution et les limites déclarées |

## Gouvernance

- **Droits de décision :** [GOVERNANCE.md](../../../../GOVERNANCE.md), [MAINTAINERS.md](../../../../MAINTAINERS.md),
  [CODEOWNERS](../../../../CODEOWNERS). Les règles de contribution et d'assistance par IA figurent dans
  [CONTRIBUTING.md](../../../../CONTRIBUTING.md) et [AI_STATEMENT.md](../../../../AI_STATEMENT.md).
- **Maîtrise des changements :** la spécification est la source de vérité ; le code et les tests
  la citent. La CI s'exécute chez deux fournisseurs, et `scripts/ci-check.sh` la reproduit.
  Règle du dépôt : ne pas affaiblir un test ou une barrière pour faire passer quelque chose.
- **Divulgation :** [SECURITY.md](../../../../SECURITY.md) est la voie de signalement.

## Risques

La vue des risques est dans la [fiche du directeur des risques](../directeur-des-risques/index.md). En bref : les risques
résiduels les plus élevés sont la sécurité non testée par des tiers (T42), l'accessibilité
non quantifiée (T48) et la classification des données, qui relève de l'adoptant.

## Conformité : ce qui est réel et ce qui ne l'est pas

| Domaine | Position |
|---|---|
| Contrôle d'accès | Mis en œuvre et testé : masquage par rôle, contrôles au niveau de l'enregistrement, lectures opérationnelles réservées à l'administration, bris de glace avec revue obligatoire par une seconde personne |
| Audit | Mis en œuvre et testé : chaîné par empreintes, en ajout seul, dans la transaction, audit des lectures y compris listes, listes d'affaires et analyses. **Déployeur :** ancrer la tête de la chaîne hors de la base de données |
| Droits relatifs à la vie privée | Mis en œuvre : export d'accès des personnes concernées, effacement (refusé tant que des obligations sont ouvertes), balayage de conservation. **Déployeur :** base légale, calendrier de conservation, registre des traitements |
| Chiffrement | Partiel : AES-256-GCM au niveau du champ sur deux colonnes avec des clés locales, sans KMS. Le chiffrement au repos et en transit incombe au déployeur |
| Chaîne d'approvisionnement | Mis en œuvre : `cargo audit`, `cargo deny`, `pnpm audit`, SBOM, images de base épinglées par empreinte, mises à jour Dependabot. **Lacune :** le flux de version signée n'a pas été exécuté sur une vraie version (T49) |
| Accessibilité | Partiel : analyses axe automatiques et déclaration publique honnête. **Lacune :** pas d'audit manuel (T48), traductions non relues |
| Assurance indépendante | **Lacune :** aucune (T42) |
| Droit de l'e-mail et du marketing | Circuit d'envoi fondé sur le consentement mis en œuvre ; l'envoi réel n'existe que comme circuit SMTP de développement ; le lien de désabonnement, la liste de suppression et un webhook de résultat signé existent, mais aucun fournisseur n'est intégré et l'accord de sous-traitance et la revue juridique restent ouverts (T39) |

## Contrôles qui ont besoin d'un responsable dans votre organisation

1. Revue et alerte pour les autorisations de bris de glace (alerter sur `break_glass_granted`).
2. Ancrage de la tête de la chaîne d'audit et vérification selon un calendrier
   (`GET /api/audits/verify`).
3. Preuves de sauvegarde et de restauration, et répétition de rotation des clés.
4. Gestion des arrivées, mobilités et départs dans votre fournisseur d'identité.
5. Conservation de la table d'audit, qui croît avec le trafic de lecture.
6. Protection de Redis comme la base de données : il contient des données d'analyse en cache.
7. Fusion des mises à jour Dependabot et vérification d'une version signée avant de la déployer.

## Ce qu'il faut refuser d'approuver

Toute affirmation que le système est conforme, certifié, conforme à la HIPAA ou approuvé
par le NHS. L'affirmation défendable est plus étroite : les contrôles sont cartographiés, ceux
qui sont mis en œuvre sont testés, et les lacunes sont nommées et suivies.
