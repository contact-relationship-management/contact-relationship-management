# Pour le directeur juridique

> Note technique, et non avis juridique, certification ou audit. Elle indique
> ce que montre le dépôt au 2026-10-11. Rien ici ne revendique une accréditation.

## Position juridique en bref

Le dépôt **n'est pas prêt pour la production** et contient des données synthétiques. Il ne revendique
la conformité à aucun régime. Il met en œuvre des fonctionnalités techniques qui en soutiennent
plusieurs, et énumère ce qu'un déployeur doit ajouter ([regulatory](../../../../spec/regulatory.md)).

## Régimes concernés

| Régime | Ce que le logiciel prend en charge | Ce que vous devez fournir |
|---|---|---|
| RGPD du Royaume-Uni et de l'UE | Historique des consentements, export d'accès des personnes concernées, effacement (refusé tant que des obligations sont ouvertes), balayage de conservation, minimisation des données, audit des lectures | Base légale par finalité, AIPD ([modèle](../../../../spec/compliance/dpia-template.md)), registre des traitements, calendrier de conservation, rôles de responsable du traitement et de sous-traitant |
| PECR / vie privée en ligne | Envois conditionnés au consentement, désabonnement immédiat | Identification de l'expéditeur et décisions de consentement implicite pour un vrai expéditeur |
| HIPAA (si données de santé américaines) | Fonctionnalités de contrôle d'audit, de contrôle d'accès, d'intégrité et de sécurité de la transmission | Un accord de partenaire commercial, une analyse des risques, des politiques, la formation du personnel. Le logiciel ne vous rend pas conforme |
| NHS DTAC / DCB0129/0160 | Projets de preuves ([dtac-supporting](../../../../spec/compliance/dtac-supporting.md)) | Responsable de la sécurité clinique nommé, registre des dangers, certifications. L'évaluation indique que le DTAC ne s'applique probablement pas |
| Accessibilité (WCAG 2.2 AA, secteur public) | Contrôles automatiques et déclaration honnête | Un audit manuel (T48) avant toute revendication de conformité |
| Sécurité gouvernementale (NIST 800-53, ISO 27001, Cyber Essentials) | Cartographie des contrôles ([control-mapping](../../../../spec/compliance/control-mapping.md)) | Certification et assurance indépendante |

## Contrats dont vous aurez besoin et qui n'existent pas encore

1. Accord de traitement des données et conditions de transfert international avec le fournisseur d'e-mail (T39).
2. Cahier des charges et conditions de confidentialité avec le cabinet de tests d'intrusion (T42).
3. Conditions d'hébergement et de sous-traitance pour PostgreSQL, Redis et les journaux.
4. Licence : voir [LICENSE.md](../../../../LICENSE.md) ; le dépôt comporte aussi
   des déclarations de sécurité, de gouvernance et d'usage de l'IA.

## Points de vigilance juridiques particuliers

- **L'effacement** anonymise le contact et est refusé tant qu'une affaire, un ticket
  ou une inscription active à une séquence de maturation reste ouvert ; la chaîne d'audit conserve des
  traces des actions, réglez donc dans votre AIPD la façon dont cela interagit avec les demandes d'effacement.
- **Le bris de glace** : les lectures de données masquées ne sont licites que sur votre base
  documentée ; le champ de motif est une preuve, non une autorité.
- **La notation** des prospects : si elle influence un jour des décisions importantes concernant
  des personnes, les règles sur les décisions automatisées s'appliquent ([regulatory](../../../../spec/regulatory.md)).
- **Engagements d'API :** [api-lifecycle](../../../../spec/compliance/api-lifecycle.md)
  fixe un préavis de 180 jours avant le retrait d'une modification d'API. C'est le
  choix du mainteneur ; si des clients doivent s'y fier, décidez si cela relève
  d'un contrat.
- **Déclarations publiques :** n'affirmez ni conformité, ni certification, ni approbation.
