# Pour le directeur de la sécurité

> Note technique, et non avis juridique, certification ou audit. Elle indique
> ce que montre le dépôt au 2026-10-11. Rien ici ne revendique une accréditation.

## Posture de sécurité en un paragraphe

Le service **échoue en sécurité**, contrôle l'accès par rôle et par enregistrement, masque
les valeurs sensibles aux rôles qui ne devraient pas les voir, et tient une piste
d'audit qui révèle toute altération. Ces contrôles sont mis en œuvre et testés par les auteurs. Ils n'ont
**été testés par personne d'indépendant** (T42) ; considérez donc chaque affirmation ci-dessous
comme « construit et autotesté », et non « vérifié ».

## Contrôles et où les vérifier

| Domaine | Ce qui est en place | Où regarder |
|---|---|---|
| Authentification | PASETO v4.public vérifié hors ligne, Keycloak facultatif ; la production refuse de démarrer sans authentification | [auth](../../../../spec/auth.md) |
| Autorisation | Fondée sur les attributs : lire, écrire, supprimer, destructif ; une obligation `mask` ; contrôles au niveau de l'enregistrement ; audit, conservation et revue du bris de glace réservés à l'administration | [auth](../../../../spec/auth.md), test de matrice de profils |
| Exposition des données | Masquage sur les enregistrements, les agrégats et les analyses ; une fuite sur le détail d'un compte a été trouvée et corrigée (T47) | [tasks](../../../../spec/tasks.md) |
| Accès d'urgence | Motif écrit, plafond de 60 minutes, lecture seule, revue obligatoire par une seconde personne, audité, partagé via Redis | tâches de bris de glace T44 |
| Audit | Chaîne d'empreintes SHA-256 dans la même transaction, déclencheur en ajout seul, point d'accès de vérification ; lectures auditées | [audit](../../../../spec/audit.md) |
| Résistance aux abus | Limitation de débit (600 par minute) et verrouillage (10 échecs en 5 minutes), partagés via Redis ; sondes exemptées | T43 |
| Navigateur et transport | CSP stricte avec empreintes, refus des cadres, `nosniff`, politique de référent, CORS fermé, en-têtes sécurisés ; aucun jeton dans le JavaScript du navigateur (BFF) | `hooks.server.ts` de l'interface |
| Protection des données | AES-256-GCM sur deux colonnes avec rotation ; clés locales, pas de KMS | T41 |
| Chaîne d'approvisionnement | `cargo audit`, `cargo deny`, `pnpm audit`, SBOM, images durcies sans privilèges root épinglées par empreinte, mises à jour Dependabot hebdomadaires, un flux de versions qui signe images et SBOM sans clé | [control-mapping](../../../../spec/compliance/control-mapping.md), [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Erreurs | Chaque erreur est un document de problème RFC 9457 ; un `401` nomme le schéma accepté | [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) |
| Sûreté de la mémoire | `forbid(unsafe_code)`, clippy pedantic | `src/lib.rs` |

## Faiblesses connues (déclarées, non cachées)

- **Aucun test d'intrusion indépendant ni exercice de modélisation des menaces** (T42). Le
  modèle de menaces de [dtac-supporting](../../../../spec/compliance/dtac-supporting.md)
  est un résumé, non un modèle structuré.
- **La signature des versions n'a pas été exécutée sur une vraie version** (T49). Le flux et les
  commandes de vérification existent ; tant qu'une étiquette n'est pas poussée, ils ne sont pas testés. Les actions
  sont épinglées par empreinte de commit.
- **Le chiffrement de champ** couvre deux colonnes avec des clés locales, sans KMS.
- **La réauthentification renforcée** avant les actions destructives n'est pas construite.
- **Redis contient des données personnelles** (analyses en cache, clés d'autorisation) et doit être protégé
  comme la base de données : TLS, mot de passe, aucune exposition publique.
- **Le bris de glace** ne peut pas être révoqué, seulement expirer, et aucune alerte n'est branchée.
- **La limitation de débit et le verrouillage** reviennent à des compteurs par réplique si Redis tombe ; le
  seau des appelants non authentifiés est par adresse, et derrière un proxy absent de
  `TRUSTED_PROXIES` c'est celle du proxy, de sorte qu'un attaquant peut verrouiller d'autres
  appelants non authentifiés. Un appelant muni d'un jeton valide est compté selon son sujet.
- **Les rechargements de politique** apparaissent dans les analyses en cache jusqu'à 30 secondes plus tard.
- **La déconnexion sur inactivité** se fait côté client.
- **Les clés d'idempotence** sont facultatives dans l'API (T52) : un client qui n'envoie pas
  `Idempotency-Key` n'est pas protégé contre une création rejouée qui dupliquerait un enregistrement.
  L'interface incluse en envoie une.

## Ce que le déployeur doit faire

1. Garder l'authentification activée ; fournir des clés de production et `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL`.
2. Alerter sur `break_glass_granted` et revoir rapidement les autorisations.
3. Ancrer la tête de la chaîne d'audit hors de la base de données et la vérifier
   (`GET /api/audits/verify`) selon un calendrier.
4. Protéger PostgreSQL et Redis (réseau, TLS, identifiants, sauvegardes, chiffrement
   au repos) ; faire tourner les clés de champ avec la tâche fournie.
5. Fusionner les mises à jour Dependabot ; relancer l'étape de chaîne d'approvisionnement ; vérifier une version
   signée avant de la déployer.
6. Suivre le guide pour la réponse aux incidents
   ([operations-runbook](../../../../spec/compliance/operations-runbook.md)).

## Gestion des vulnérabilités

Les signalements passent par [SECURITY.md](../../../../SECURITY.md). Commandez le test
indépendant (T42) sur une version de préproduction avant toute donnée réelle, et
refaites-le après tout changement important.

## Actions de sécurité suggérées, dans l'ordre

1. Définir le périmètre du test d'intrusion indépendant et le commander (T42).
2. Pousser une première étiquette de version pour prouver la signature et la vérification (T49).
3. Brancher l'alerte pour le bris de glace et pour les échecs de vérification de la chaîne d'audit.
4. Placer Redis derrière TLS et des identifiants dans chaque environnement.
5. Clés de champ adossées à un KMS et couverture de champs plus large (reste de T41).

Voir aussi la [fiche du directeur technique](../directeur-technique/index.md) et la
[fiche du directeur des risques](../directeur-des-risques/index.md).
