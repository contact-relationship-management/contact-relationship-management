# Pour le directeur des systèmes d'information

> Note technique, et non avis juridique, certification ou audit. Elle indique
> ce que montre le dépôt au 2026-10-11. Rien ici ne revendique une accréditation.

## Vue de l'information et des données

Ce CRM conserve des **données de relation** : contacts et comptes sous forme
d'enveloppes autour d'identifiants détenus dans d'autres systèmes, activités,
affaires, tickets, historique des consentements et journal d'audit. Les
données démographiques restent volontairement dans les registres d'identité
([domain-model](../../../../spec/domain-model.md)). Si votre organisation
enregistre des patients ou des citoyens comme contacts, la classification de
ces données **vous** revient ; le plan est rédigé selon la lecture la plus
stricte ([plan](../../../../spec/plan.md)).

## Où vont les données

| Stockage | Contenu | Protection dans le dépôt |
|---|---|---|
| PostgreSQL | Tous les enregistrements, chaîne d'audit, autorisations de bris de glace | Chiffrement de champ sur deux colonnes ; déclencheur d'audit en ajout seul ; le chiffrement au repos incombe au déployeur |
| Redis | Cache des réponses d'analyse (30 s), compteurs de limitation de débit, clés d'autorisation de bris de glace | Clé par appelant ; **exige TLS, un mot de passe et aucune exposition publique** |
| Journaux/métriques | Métadonnées des requêtes, métriques Prometheus | La conservation incombe au déployeur |
| Fournisseur de courriel (à venir) | Contenu des messages et destinataires | Non construit (T39) |

## Intégration et identité

- La connexion provient d'un service d'identité émettant des jetons PASETO, ou de
  Keycloak. L'arrivée, la mobilité et le départ des personnes, l'authentification
  multifacteur et la politique de session relèvent donc de **votre fournisseur
  d'identité** ; le CRM applique ce que dit le jeton et refuse les jetons expirés.
- L'API est documentée en OpenAPI ([gds-api-review](../../../../spec/compliance/gds-api-review.md)),
  versionnée par en-tête, paginée avec des en-têtes `Link`, et renvoie les erreurs
  sous forme de documents de problème RFC 9457. Les règles de modification et
  d'obsolescence figurent dans
  [api-lifecycle](../../../../spec/compliance/api-lifecycle.md).
- Des SBOM (CycloneDX) sont produits pour le service et l'interface en CI ; le flux
  de versions les signe, ainsi que les images.

## Obligations de gouvernance de l'information qui restent à votre charge

Preuves de sauvegarde et de restauration, calendriers de conservation par table,
ancrage de la tête de la chaîne d'audit hors de la base de données, garde des clés de
champ dans un KMS, accords de traitement des données pour tout fournisseur, et
analyse d'impact relative à la protection des données
([dpia-template](../../../../spec/compliance/dpia-template.md)). La cartographie
des contrôles ([control-mapping](../../../../spec/compliance/control-mapping.md))
marque chaque contrôle comme Mis en œuvre, Déployeur ou Lacune.
