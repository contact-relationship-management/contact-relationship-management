# Pour le directeur des opérations

> Note technique, et non avis juridique, certification ou audit. Elle indique
> ce que montre le dépôt au 2026-10-11. Rien ici ne revendique une accréditation.

## Ce qu'exige l'exploitation

Le service est un processus Rust par réplique, plus PostgreSQL 18 et Redis ; l'interface
est un processus Node. Les deux sont livrés sous forme d'images de conteneur durcies, sans
privilèges root, avec des images de base épinglées par empreinte. Le guide d'exploitation est
[operations-runbook.md](../../../../spec/compliance/operations-runbook.md).

## Faits opérationnels à prévoir

- **Échec sûr au démarrage.** En production, le service refuse de démarrer si
  l'authentification est désactivée, si la chaîne d'audit est inutilisable, ou si les clés requises et
  `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL` sont absentes. Un déploiement mal configuré s'arrête au lieu de fonctionner sans protection.
- **Santé et métriques.** `/_health/live`, `/_health/ready` et `/_health/startup` ne sont jamais
  soumis à la limitation de débit, de sorte qu'un verrouillage ne peut pas redémarrer un pod sain. Métriques sur
  `/metrics.prom`.
- **Les limites d'abus** sont actives en production : 600 requêtes par minute et par client, et un
  verrouillage après 10 échecs de connexion en 5 minutes. Elles sont partagées entre
  les répliques via Redis ; si Redis tombe, elles reviennent à des compteurs par réplique.
- **Accès d'urgence** (bris de glace) : tout utilisateur connecté peut prendre une
  élévation de lecture motivée jusqu'à 60 minutes. Quelqu'un d'autre doit revoir chaque autorisation.
  **Processus que vous devez mener :** alerter sur les lignes d'audit `break_glass_granted` et
  examiner le nombre d'autorisations non revues (page d'administration `/break-glass/review`).
- **Les erreurs** ont une seule forme (RFC 9457 `application/problem+json`, avec `error` et
  `description` conservés), de sorte que l'analyse des journaux et les règles d'alerte peuvent s'appuyer sur `status` et `type`.
- **Croissance de l'audit.** Les lectures sont auditées, donc la table d'audit croît avec le trafic.
  Dimensionnez sa conservation. Un client qui interroge en boucle la fera croître vite.

## Travail opérationnel ouvert

| Élément | État |
|---|---|
| Exercices de sauvegarde/restauration, répétition de rotation des clés | Écrits dans le guide ; **vous** devez les exécuter et en conserver les preuves |
| Ancrage de la tête de la chaîne d'audit hors de la base de données | Votre devoir ; l'API indique l'empreinte de la tête |
| Actualisation des empreintes d'images épinglées | Dependabot ouvre des pull requests chaque semaine ; **vous** devez les examiner et les fusionner, sinon les épinglages deviennent périmés |
| Vérification d'une version signée avant son déploiement | Les commandes sont dans le guide ; déployez par empreinte et refusez toute image qui échoue à la vérification. Non testé tant qu'une première version étiquetée n'existe pas |
| Vrai fournisseur de courriel (T39) | Ouvert ; modifie le tableau des incidents et des fournisseurs |
| Redis contient des données d'analyse | Protégez-le comme la base de données : TLS, mot de passe, aucune exposition publique |

## Limites connues à signaler à l'équipe de garde

La déconnexion sur inactivité se fait côté client ; un jeton révoqué est refusé aussitôt, mais un
rechargement de politique apparaît dans les analyses en cache jusqu'à 30 secondes plus tard ; le bris de glace ne peut pas être
révoqué, seulement expirer.
