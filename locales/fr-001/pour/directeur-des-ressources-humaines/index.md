# Pour le directeur des ressources humaines

> Note technique, et non avis juridique, certification ou audit. Elle indique
> ce que montre le dépôt au 2026-10-11. Rien ici ne revendique une accréditation.

## Pourquoi les personnes comptent pour ce produit

L'accès aux données personnelles est décidé par rôle. Le logiciel applique ce que dit le
jeton d'une personne ; **qui obtient quel rôle, et quand il est retiré, relève d'un processus
humain** que ce dépôt ne peut pas assurer à votre place.

## Ce que le logiciel apporte à vos processus

- **Visibilité par rôle :** certains rôles voient des valeurs masquées pour les canaux de contact,
  les montants des affaires et les vues financières ; les responsables, l'administration et les niveaux de service en voient davantage
  ([auth](../../../../spec/auth.md)).
- **Lectures auditées :** la lecture de données personnelles laisse une trace qui nomme la personne.
  Le personnel doit en être informé avant la mise en service.
- **Accès d'urgence :** une personne peut prendre une élévation de lecture motivée jusqu'à 60
  minutes ; une autre personne doit la revoir ensuite. Formez les deux parties.
- **Déconnexion sur inactivité** après 15 minutes dans le navigateur.
- **Accessibilité :** une déclaration publique et des contrôles automatiques ; l'audit manuel
  (T48) nécessite encore des utilisateurs en situation de handicap et des testeurs de technologies d'assistance.
- **Langues :** l'interface est traduite en 8 paramètres régionaux ; les traductions n'ont
  **pas** été relues par des locuteurs natifs.

## Vos responsabilités

| Domaine | Action |
|---|---|
| Arrivées, mobilités, départs | Accordez et retirez les rôles via votre fournisseur d'identité ; le CRM refuse les jetons expirés mais ne gère pas les comptes |
| Formation | Protection des données, règles de consentement, usage et revue du bris de glace, pratiques accessibles |
| Politiques | Usage acceptable, avis de surveillance (les lectures sont auditées), voie disciplinaire en cas d'usage abusif de l'accès d'urgence |
| Capacité de revue | Nommez qui revoit les autorisations de bris de glace et qui assure la suppléance en cas d'absence |
| Recrutement | Rôles dont le programme a besoin : liaison avec le testeur indépendant, responsable de la sécurité clinique si le NHS est concerné, responsable de la protection des données, responsable de l'accessibilité |
| Bien-être | Soutenez le personnel qui traite des cas sensibles ; le CRM n'est pas un système de gestion de dossiers mais peut contenir des données de relation sensibles |

## Points ouverts qui nécessitent des personnes

Audit manuel d'accessibilité avec des utilisateurs de technologies d'assistance (T48), relecture des
traductions par des locuteurs natifs, et prise en charge des tâches de revue et d'alerte ci-dessus.
Voir [tasks.md](../../../../spec/tasks.md).
