# Para o Diretor de Segurança (CISO)

> Resumo de engenharia, não é aconselhamento jurídico, certificação nem auditoria. Ele informa
> o que o repositório mostra em 2026-10-11. Nada aqui afirma acreditação.

## Postura de segurança em um parágrafo

O serviço **falha de forma segura**, controla o acesso por papel e por registro, oculta valores sensíveis dos papéis que
não deveriam vê-los e mantém uma trilha de auditoria que revela adulteração. Esses controles foram implementados e
testados pelos autores. Eles **não foram testados por ninguém independente** (T42); portanto, trate cada afirmação
abaixo como "construído e autotestado", não como "verificado".

## Controles e onde conferi-los

| Área | O que está em vigor | Onde olhar |
|---|---|---|
| Autenticação | PASETO v4.public verificado offline, Keycloak opcional; a produção se recusa a iniciar com a autenticação desligada | [auth](../../../../spec/auth.md) |
| Autorização | Baseada em atributos: ler, escrever, excluir, destrutivo; uma obrigação `mask`; verificações no nível do registro; auditoria, retenção e revisão de break-glass restritas a administradores | [auth](../../../../spec/auth.md), teste de matriz de personas |
| Exposição de dados | Mascaramento em registros, agregados e análises; um vazamento nos detalhes da conta foi encontrado e corrigido (T47) | [tasks](../../../../spec/tasks.md) |
| Acesso de emergência | Motivo escrito, limite de 60 minutos, somente leitura, revisão obrigatória por uma segunda pessoa, auditado, compartilhado via Redis | tarefas de break-glass T44 |
| Auditoria | Cadeia de hash SHA-256 na mesma transação, gatilho somente de acréscimo, endpoint de verificação; leituras auditadas | [audit](../../../../spec/audit.md) |
| Resistência a abuso | Limite de taxa (600 por minuto) e bloqueio (10 falhas em 5 minutos), compartilhados via Redis; sondas isentas | T43 |
| Navegador e transporte | CSP rigorosa com hashes, negação de frames, `nosniff`, política de referrer, CORS fechado, cabeçalhos seguros; nenhum token em JavaScript do navegador (BFF) | UI `hooks.server.ts` |
| Proteção de dados | AES-256-GCM em duas colunas com rotação; chaves locais, sem KMS | T41 |
| Cadeia de suprimentos | `cargo audit`, `cargo deny`, `pnpm audit`, SBOMs, imagens reforçadas sem root fixadas por digest, atualizações semanais do Dependabot, um fluxo de versão que assina imagens e SBOMs sem chave | [control-mapping](../../../../spec/compliance/control-mapping.md), [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Erros | Todo erro é um documento de problema RFC 9457; um `401` nomeia o esquema aceito | [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) |
| Segurança de memória | `forbid(unsafe_code)`, clippy pedantic | `src/lib.rs` |

## Fragilidades conhecidas (declaradas, não escondidas)

- **Nenhum teste de intrusão independente nem exercício de modelagem de ameaças** (T42). O modelo de ameaças em
  [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) é um resumo, não um modelo estruturado.
- **A assinatura de versões não rodou em uma versão real** (T49). O fluxo e os comandos de verificação existem; até que
  uma tag seja enviada, não foram testados. As actions são fixadas por hash de commit.
- A **criptografia de campo** cobre duas colunas com chaves locais, sem KMS.
- A **reautenticação elevada** antes de ações destrutivas não está construída.
- O **Redis guarda dados pessoais** (análises em cache, chaves de concessão) e deve ser protegido como o banco de dados:
  TLS, senha, nenhuma exposição pública.
- O **break-glass** não pode ser revogado, apenas expirar, e nenhum alerta está conectado.
- O **limite de taxa e o bloqueio** voltam a contagens por réplica se o Redis falhar; o balde sem autenticação é por
  endereço e, atrás de um proxy que não esteja em `TRUSTED_PROXIES`, é o do proxy, de modo que um atacante pode
  bloquear outros chamadores sem autenticação. Um chamador com um token válido é contado por seu sujeito.
- As **recargas de política** aparecem nas análises em cache após até 30 segundos.
- O **logout por inatividade** é do lado do cliente.
- As **chaves de idempotência** são opcionais na API (T52): um cliente que não envia `Idempotency-Key` não fica
  protegido contra uma criação repetida que duplique um registro. A interface incluída envia uma.

## O que quem implanta deve fazer

1. Manter a autenticação ligada; fornecer chaves de produção e `CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL`.
2. Alertar sobre `break_glass_granted` e revisar as concessões prontamente.
3. Ancorar o topo da cadeia de auditoria fora do banco de dados e verificá-lo (`GET /api/audits/verify`) em
   intervalos regulares.
4. Proteger o PostgreSQL e o Redis (rede, TLS, credenciais, backups, criptografia em repouso); rotacionar as chaves de
   campo com a tarefa fornecida.
5. Mesclar as atualizações do Dependabot; executar de novo a etapa de cadeia de suprimentos; verificar uma versão
   assinada antes de implantá-la.
6. Seguir o manual para a resposta a incidentes ([operations-runbook](../../../../spec/compliance/operations-runbook.md)).

## Tratamento de vulnerabilidades

Os relatos seguem por [SECURITY.md](../../../../SECURITY.md). Contrate o teste independente (T42) contra uma build de
homologação antes de qualquer dado real e refaça-o após mudanças relevantes.

## Ações de segurança sugeridas, em ordem

1. Definir o escopo e contratar o teste de intrusão independente (T42).
2. Enviar uma primeira tag de versão para provar a assinatura e a verificação (T49).
3. Conectar alertas para o break-glass e para falhas de verificação da cadeia de auditoria.
4. Colocar o Redis atrás de TLS e credenciais em todos os ambientes.
5. Chaves de campo apoiadas em KMS e maior cobertura de campos (resto da T41).

Veja também o [resumo do CTO](../diretor-de-tecnologia/index.md) e o [resumo do CRO](../diretor-de-riscos/index.md).
