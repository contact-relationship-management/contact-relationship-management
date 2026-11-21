# Para o Diretor Jurídico (CLO)

> Resumo de engenharia, não é aconselhamento jurídico, certificação nem auditoria. Ele informa
> o que o repositório mostra em 2026-10-11. Nada aqui afirma acreditação.

## Postura jurídica em resumo

O repositório **não está pronto para produção** e guarda dados sintéticos. Ele não afirma conformidade com nenhum
regime. Implementa recursos técnicos que dão suporte a vários deles e lista o que quem implanta deve acrescentar
([regulatory](../../../../spec/regulatory.md)).

## Regimes envolvidos

| Regime | O que o software oferece | O que você deve fornecer |
|---|---|---|
| GDPR do Reino Unido/UE | Histórico de consentimento, exportação de acesso pelo titular, exclusão (recusada enquanto houver obrigações em aberto), varredura de retenção, minimização de dados, auditoria de leituras | Base legal por finalidade, DPIA ([modelo](../../../../spec/compliance/dpia-template.md)), registos de tratamento, calendÃ¡rio de retenção, papéis de controlador/operador |
| PECR / e-privacy | Envios condicionados a consentimento, cancelamento imediato | Identificação do remetente e decisões de soft opt-in para um remetente real |
| HIPAA (se houver dados de saúde dos EUA) | Controles de auditoria, controle de acesso, integridade, recursos de segurança na transmissão | Um acordo de associado de negócios, análise de riscos, políticas, treinamento da equipa. O software não torna você conforme |
| NHS DTAC / DCB0129/0160 | Rascunhos de evidências ([dtac-supporting](../../../../spec/compliance/dtac-supporting.md)) | Responsável nomeado pela segurança clínica, registo de perigos, certificações. A avaliação diz que o DTAC provavelmente não se aplica |
| Acessibilidade (WCAG 2.2 AA, setor público) | Verificações automatizadas e uma declaração honesta | Uma auditoria manual (T48) antes de qualquer afirmação de conformidade |
| Segurança governamental (NIST 800-53, ISO 27001, Cyber Essentials) | Mapeamento de controles ([control-mapping](../../../../spec/compliance/control-mapping.md)) | Certificação e garantia independente |

## Contratos de que você precisará e que ainda não existem

1. Acordo de tratamento de dados e termos de transferência internacional com o provedor de e-mail (T39).
2. Declaração de trabalho e termos de confidencialidade com a empresa de testes de intrusão (T42).
3. Termos de alojamento e de suboperadores para PostgreSQL, Redis e logs.
4. Licença: veja [LICENSE.md](../../../../LICENSE.md); o repositório traz também declarações de segurança, governança e
   uso de IA.

## Pontos jurídicos específicos de atenção

- A **exclusão** anonimiza o contacto e é recusada enquanto existir um negócio, chamado ou inscrição ativa em sequência
  de relacionamento em aberto; a cadeia de auditoria guarda registos das ações, portanto resolva na sua DPIA como isso
  interage com os pedidos de exclusão.
- As leituras de dados mascarados por **break-glass** só são lícitas com a sua base documentada; o campo de motivo é
  evidência, não autoridade.
- A **pontuação** de leads: se um dia influenciar decisões significativas sobre pessoas, aplicam-se as regras de decisão
  automatizada ([regulatory](../../../../spec/regulatory.md)).
- **Compromissos da API:** [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) define um aviso prévio de 180
  dias antes de remover uma mudança da API. Essa é uma escolha do mantenedor; se os clientes passarem a depender dela,
  decida se ela deve constar de um contrato.
- **Declarações públicas:** não afirme conformidade, certificação nem aprovação.
