# Para o Diretor de Operações (COO)

> Resumo de engenharia, não é aconselhamento jurídico, certificação nem auditoria. Ele informa
> o que o repositório mostra em 2026-10-11. Nada aqui afirma acreditação.

## O que é preciso para operar isto

O serviço é um processo Rust por réplica, mais PostgreSQL 18 e Redis; o front-end é um processo Node. Ambos são
distribuídos como imagens de contêiner reforçadas, sem root, com imagens base fixadas por digest. O manual de operações
é [operations-runbook.md](../../../../spec/compliance/operations-runbook.md).

## Factos operacionais para planear

- **Falha de forma segura na inicialização.** Em produção, o serviço se recusa a iniciar se a autenticação estiver
  desligada, se a cadeia de auditoria estiver inutilizável ou se faltarem as chaves obrigatórias e
  `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL`. Uma implantação mal configurada para, em vez de rodar aberta.
- **Saúde e métricas.** `/_health/live`, `/_health/ready` e `/_health/startup` nunca têm limite de taxa, de modo que
  um bloqueio não pode reiniciar um pod saudável. Métricas em `/metrics.prom`.
- **Limites contra abuso** ficam ativos em produção: 600 requisições por minuto por cliente e um bloqueio após 10
  inícios de sessão falhados em 5 minutos. Eles são compartilhados entre réplicas por meio do Redis; se o Redis falhar,
  voltam a contagens por réplica.
- **Acesso de emergência** (break-glass): qualquer usuário autenticado pode obter uma elevação de leitura justificada
  por até 60 minutos. Outra pessoa deve revisar cada concessão. **Processo que você deve executar:** alertar sobre as
  linhas de auditoria `break_glass_granted` e revisar a contagem de itens não revisados (página de administração
  `/break-glass/review`).
- Os **erros** têm um único formato (RFC 9457 `application/problem+json`, mantendo `error` e `description`), de modo
  que a análise de logs e as regras de alerta podem confiar em `status` e `type`.
- **Crescimento da auditoria.** As leituras são auditadas, então a tabela de auditoria cresce com o tráfego. Dimensione
  a retenção para ela. Um cliente que faz polling a fará crescer rápido.

## Trabalho operacional em aberto

| Item | Situação |
|---|---|
| Simulações de backup/restauração, ensaio de rotação de chaves | Escritos no manual; **você** deve executá-los e comprová-los |
| Ancoragem do topo da cadeia de auditoria fora do banco de dados | Dever seu; a API informa o hash do topo |
| Atualização dos digests de imagem fixados | O Dependabot abre pull requests semanais; **você** deve revisá-los e mesclá-los, ou as fixações ficam obsoletas |
| Verificação de uma versão assinada antes de implantá-la | Os comandos estão no manual; implante por digest e recuse uma imagem que falhe na verificação. Não testado até a primeira versão marcada |
| Provedor real de e-mail (T39) | Em aberto; muda o quadro de incidentes e de fornecedores |
| O Redis guarda dados de análises | Proteja-o como o banco de dados: TLS, senha, nenhuma exposição pública |

## Limites conhecidos que vale contar à equipa de plantão

O fim de sessÃ£o por inatividade é do lado do cliente; um token revogado é recusado na hora, mas uma recarga de política só
aparece nas análises em cache após até 30 segundos; o break-glass não pode ser revogado, apenas expirar.
