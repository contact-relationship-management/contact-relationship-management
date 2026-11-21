# Chemin vers la production

> Contact Relationship Management n'est pas prêt pour la production. Cette page explique ce que signifie la production
> ici et où en est chaque partie.

Le logiciel fonctionne avec des données synthétiques. L'e-mail est simulé tant qu'un vrai mode de messagerie n'est
pas configuré. N'y saisissez pas de données personnelles réelles.

## Ce que signifie la production

Un déploiement en production conserve les données de personnes réelles et leur envoie de vrais messages. Pour ce
projet, cela exige tout ce qui suit :

- **Connexion réelle.** Chaque requête est authentifiée et les règles d'accès correspondent aux rôles de
  l'organisation qui déploie.
- **Envoi licite.** Chaque message a un consentement enregistré et une base légale conforme à la loi de chaque lieu
  de destination, et passe par un vrai fournisseur d'e-mail qui gère les rebonds, les plaintes et les
  désabonnements.
- **Assurance indépendante.** Des personnes extérieures au projet ont testé la sécurité et l'accessibilité, et les
  constats sont corrigés ou acceptés.
- **Exploitation.** Des sauvegardes qui ont été restaurées, une supervision, un processus d'incident et quelqu'un
  d'astreinte.
- **Preuve de stabilité.** De longues exécutions sous charge réaliste, sur un matériel semblable à celui du
  déploiement, sans erreur ni ralentissement.

La préparation à la production se décide par déploiement. Un déployeur peut fermer certaines barrières seulement pour
sa propre organisation et ses destinataires. L'évaluation technique derrière cette page est
[spec/plan.md](../../../spec/plan.md). Les barrières ouvertes figurent dans [spec/tasks.md](../../../spec/tasks.md).

## Préparation

Chaque pourcentage est une estimation faite le 2026-11-13 à partir de la liste des tâches, et non une mesure.

- **Authentification et contrôle d'accès** (70%): La connexion par jetons PASETO ou Keycloak/OIDC, les règles d'accès
  fondées sur les attributs avec contrôles au niveau de l'enregistrement et un mode production qui refuse de démarrer
  sans authentification sont construits. Aucun déploiement réel ne les a activés avec ses propres attributs, et la
  matrice d'application ne couvre pas encore tous les gestionnaires (CRM-G1).
- **Livraison des e-mails** (40%): Un expéditeur simulé est la valeur par défaut et un expéditeur SMTP existe. Il
  n'y a pas d'adaptateur de fournisseur d'e-mail avec webhooks de rebonds et de plaintes, ni de liste de
  suppression (CRM-T39).
- **Protection des données et droit du marketing électronique** (15%): Les registres de consentement, l'accès des
  personnes concernées, l'effacement et la conservation sont construits. Le droit d'aucune région n'a été lu pour ce
  projet, et la revue du vrai circuit d'envoi reste ouverte (CRM-G2, [réglementation](../../../spec/regulatory.md)).
- **Assurance de sécurité indépendante** (10%): Un périmètre de test d'intrusion est rédigé. Aucun test d'intrusion,
  analyse formelle des risques ni accréditation n'a été réalisé (CRM-T42). Les audits de sécurité par IA ne sont pas
  une assurance indépendante.
- **Accessibilité** (50%): Les contrôles automatiques axe-core selon WCAG 2.2 A et AA réussissent sur chaque route de
  page. Aucun audit manuel au clavier seul, avec lecteurs d'écran ou zoom à 400% n'a été réalisé (CRM-T48).
- **Traductions** (30%): L'interface est disponible en huit langues. Les chaînes dans des langues autres que
  l'anglais sont écrites par machine et aucun locuteur ne les a relues.
- **Exploitation** (30%): Un guide d'exploitation couvre la sauvegarde et la restauration, la gestion des clés, le
  chiffrement au repos, la réponse aux incidents et la conservation des journaux. Aucune restauration n'a été testée
  et la liste de contrôle de mise en service du guide reste ouverte.
- **Performance et stabilité** (20%): Des tests de charge et un test de stabilité de longue durée existent. Le test
  de stabilité a duré deux minutes sur la pile de démonstration ; il n'existe ni exécution longue ni référence sur un
  matériel proche de la production.
- **Intégrité des versions** (85%): Les images des versions sont multi-architectures, signées et attestées, et leurs
  signatures se vérifient. Les actions et les images sont épinglées par empreinte, et l'historique est analysé à la
  recherche de secrets. La moitié amd64 d'une image de version téléchargée n'a pas été vérifiée, et il n'y a pas
  encore de second hébergeur de code (CRM-T192).
- **Intégration continue** (100%): Un seul script exécute toutes les vérifications sur GitHub Actions et Woodpecker.
  Les deux bases de code Rust et l'interface ont un plancher de 95% de lignes couvertes ; des barrières de navigateur,
  de lint, de chaîne d'approvisionnement, de mutation des lignes modifiées et de mutation complète hebdomadaire sont
  en place (CRM-T94).
- **Garde-fous de la démonstration** (100%): Chaque visiteur de la visite reçoit une copie séparée des données de
  démonstration, plafonnée à 200 copies. La visite est désactivée par défaut ; définissez
  `CONTACT_RELATIONSHIP_MANAGEMENT_DEMO_TOUR=on` pour la proposer (CRM-T92).

## Comment cette page est tenue à jour

Lorsqu'une barrière se ferme, modifiez l'estimation et sa preuve dans le même commit que le travail, et liez la tâche.
