# Pour le directeur technique

> Note technique, et non avis juridique, certification ou audit. Elle indique
> ce que montre le dépôt au 2026-10-11. Rien ici ne revendique une accréditation.

## Architecture

- **Service :** Rust sur Axum et axa (sqlx 0.9), PostgreSQL 18, Redis pour le cache,
  les compteurs partagés et la propagation du bris de glace. `forbid(unsafe_code)`, clippy
  pedantic. La source de vérité du comportement est la spécification
  ([index](../../../../spec/index.md), [architecture](../../../../spec/architecture.md)).
- **Interface :** SvelteKit 3 / Svelte 5, un proxy backend pour frontend afin qu'aucun jeton
  n'atteigne jamais le JavaScript du navigateur, CSP stricte avec empreintes.
- **Authentification :** PASETO v4.public vérifié hors ligne, Keycloak facultatif ; contrôle d'accès
  fondé sur les attributs (actions lire/écrire/supprimer/destructif, une obligation `mask`,
  contrôles au niveau de l'enregistrement) issu d'un moteur partagé.
- **Audit :** chaîne d'empreintes SHA-256 écrite dans la même transaction que la modification,
  déclencheur en ajout seul, vérifiable à `GET /api/audits/verify`.
- **Protection des données :** chiffrement de champ AES-256-GCM avec rotation des clés sur deux
  colonnes ; clés locales, pas de KMS.

## Preuves de qualité

Tests unitaires (185), tests de base de données (56, dont application des règles, masquage, audit,
pagination, cache, idempotence, vérifications de schéma), tests unitaires de l'interface (106) et
Playwright (98), dont des analyses d'accessibilité axe. La CI exécute fmt, clippy, les tests, la compilation et le lint de l'interface et
une étape de chaîne d'approvisionnement (`cargo audit`, `cargo deny`, `pnpm audit`, SBOM CycloneDX)
sur GitHub Actions et Woodpecker. Une seule commande la reproduit : `scripts/ci-check.sh`.

## Dette technique et limites honnêtes

- La pagination utilise un curseur par décalage (des lignes peuvent être sautées ou répétées si les données changent
  en cours de parcours) ; pas de total par conception.
- Les clés d'idempotence sont facultatives dans l'API (T52) : un client doit envoyer
  `Idempotency-Key` lors d'une création pour être protégé contre une nouvelle tentative qui dupliquerait un
  enregistrement. L'interface incluse en envoie une à chaque création.
- Les schémas OpenAPI sont écrits à la main. Ils couvrent les requêtes, les listes, les lectures de détail,
  les tableaux de bord, les vues d'analyse, l'audit, la vie privée et le bris de glace ; les transitions d'état
  et les upserts qui renvoient l'enregistrement mis à jour ne sont pas typés. Un test échoue quand une
  route et le document divergent, et d'autres vérifient de vraies réponses, y compris masquées,
  par rapport aux schémas
  ([gds-api-review](../../../../spec/compliance/gds-api-review.md)).
- Les erreurs sont des documents de problème RFC 9457 (T54) qui conservent les anciens membres `error` et
  `description` ; les erreurs des couches extérieures au routeur (limite de taille,
  délai d'attente) ne sont pas converties. La politique de versionnage et d'obsolescence est
  [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) ; les en-têtes
  `Deprecation` et `Sunset` sont spécifiés mais non construits.
- Le chiffrement de champ couvre deux colonnes ; pas d'intégration à un KMS.
- L'état de la limitation de débit et du cache exige Redis pour être correct avec plusieurs répliques.
- Les vues d'analyse sont masquées par rôle mais mises en cache par jeton d'appelant pendant 30 s.
- La réauthentification renforcée avant les actions destructives exige un travail avec le fournisseur d'identité.

## Séquence technique recommandée

1. Clés de champ adossées à un KMS et couverture de champs plus large (reste de T41).
2. Étiqueter une première version pour prouver la signature sans clé et la vérification (T49), et
   faire vérifier par un relecteur le projet de normes d'API.
3. Construire les en-têtes `Deprecation` et `Sunset` avec la première obsolescence.
4. Commander le test indépendant (T42) sur une version de préproduction.
