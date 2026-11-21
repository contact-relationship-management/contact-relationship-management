# Para o Diretor de Informação (CIO)

> Resumo de engenharia, não é aconselhamento jurídico, certificação nem auditoria. Ele informa
> o que o repositório mostra em 2026-10-11. Nada aqui afirma acreditação.

## Visão de informação e dados

Este CRM guarda **dados de relacionamento**: contatos e contas como envoltórios de identificadores mantidos em outros
sistemas, atividades, negócios, chamados, histórico de consentimento e um registro de auditoria. Os dados demográficos
permanecem deliberadamente nos registros de identidade ([domain-model](../../../../spec/domain-model.md)). Se a sua
organização registra pacientes ou cidadãos como contatos, a classificação desses dados é **sua** decisão; o plano foi
escrito para a leitura mais rigorosa ([plan](../../../../spec/plan.md)).

## Para onde vão os dados

| Armazenamento | O que guarda | Proteção no repositório |
|---|---|---|
| PostgreSQL | Todos os registros, a cadeia de auditoria, as concessões de break-glass | Criptografia de campo em duas colunas; gatilho de auditoria somente de acréscimo; a criptografia em repouso é de quem implanta |
| Redis | Cache das respostas de análises (30 s), contadores de limite de taxa, chaves de concessão de break-glass | Chaveado por chamador; **precisa de TLS, senha e nenhuma exposição pública** |
| Logs/métricas | Metadados de requisições, métricas do Prometheus | A retenção é de quem implanta |
| Provedor de e-mail (futuro) | Conteúdo das mensagens e destinatários | Não construído (T39) |

## Integração e identidade

- O login vem de um serviço de identidade que emite tokens PASETO, ou do Keycloak. Entrada, mudança e saída de pessoas,
  MFA e política de sessão são, portanto, **do seu provedor de identidade**; o CRM aplica o que o token diz e recusa os
  expirados.
- A API é documentada em OpenAPI ([gds-api-review](../../../../spec/compliance/gds-api-review.md)), versionada por
  cabeçalho, paginada com cabeçalhos `Link` e devolve erros como documentos de problema RFC 9457. As regras de
  mudança e descontinuação estão em [api-lifecycle](../../../../spec/compliance/api-lifecycle.md).
- SBOMs (CycloneDX) são produzidos para o serviço e a interface na CI; o fluxo de versão assina-os e assina as imagens.

## Deveres de governança da informação que continuam com você

Evidência de backup e restauração, cronogramas de retenção por tabela, ancoragem do topo da cadeia de auditoria fora do
banco de dados, custódia das chaves de campo em um KMS, acordos de tratamento de dados com qualquer fornecedor e a
avaliação de impacto sobre a proteção de dados ([dpia-template](../../../../spec/compliance/dpia-template.md)). O
mapeamento de controles ([control-mapping](../../../../spec/compliance/control-mapping.md)) marca cada controle como
Implementado, Implantador ou Lacuna.
