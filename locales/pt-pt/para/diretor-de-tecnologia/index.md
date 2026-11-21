# Para o Diretor de Tecnologia (CTO)

> Resumo de engenharia, não é aconselhamento jurídico, certificação nem auditoria. Ele informa
> o que o repositório mostra em 2026-10-11. Nada aqui afirma acreditação.

## Arquitetura

- **Serviço:** Rust com Axum e axa (sqlx 0.9), PostgreSQL 18, Redis para cache, contadores compartilhados e propagação
  do break-glass. `forbid(unsafe_code)`, clippy pedantic. A fonte da verdade do comportamento é a especificação
  ([index](../../../../spec/index.md), [architecture](../../../../spec/architecture.md)).
- **Interface:** SvelteKit 3 / Svelte 5, um proxy backend-for-frontend para que nenhum token chegue ao JavaScript do
  navegador, CSP rigorosa com hashes.
- **Autenticação:** PASETO v4.public verificado offline, Keycloak opcional; controle de acesso baseado em atributos
  (ações de leitura/escrita/exclusão/destrutivas, uma obrigação `mask`, verificações no nível do registo) a partir de
  um mecanismo compartilhado.
- **Auditoria:** cadeia de hash SHA-256 gravada na mesma transação da mudança, acionador somente de acréscimo, verificável
  em `GET /api/audits/verify`.
- **Proteção de dados:** criptografia de campo AES-256-GCM com rotação de chaves em duas colunas; chaves locais, sem KMS.

## Evidência de qualidade

Testes unitários (185), testes de banco de dados (56, incluindo aplicação de regras, mascaramento, auditoria,
paginação, cache, idempotência e verificações de esquema), testes unitários do front-end (106) e Playwright (98),
incluindo varreduras de acessibilidade do axe. A CI executa fmt, clippy, testes, build e lint do front-end e uma etapa
de cadeia de suprimentos (`cargo audit`, `cargo deny`, `pnpm audit`, SBOMs CycloneDX) no GitHub Actions e no
Woodpecker. Um comando a reproduz: `scripts/ci-check.sh`.

## Dívida técnica e limites honestos

- A paginação usa um cursor por deslocamento (linhas podem ser puladas ou repetidas se os dados mudarem durante a
  navegação); não há contagem total, por design.
- As chaves de idempotência são opcionais na API (T52): um cliente deve enviar `Idempotency-Key` em uma criação para
  ficar protegido de uma nova tentativa que duplique um registo. A interface incluída envia uma em toda criação.
- Os esquemas OpenAPI são escritos à mão. Cobrem requisições, listas, leituras de detalhe, painéis, visões de análises,
  auditoria, privacidade e break-glass; transições de estado e upserts que devolvem o registo atualizado não têm
  tipos. Um teste falha quando uma rota e o documento divergem, e outros verificam respostas reais, inclusive as
  mascaradas, contra os esquemas ([gds-api-review](../../../../spec/compliance/gds-api-review.md)).
- Os erros são documentos de problema RFC 9457 (T54), mantendo os membros anteriores `error` e `description`; erros de
  camadas fora do roteador (limite de tamanho, timeout) não são convertidos. A política de versionamento e
  descontinuação é [api-lifecycle](../../../../spec/compliance/api-lifecycle.md); os cabeçalhos `Deprecation` e
  `Sunset` estão especificados, mas não construídos.
- A criptografia de campo cobre duas colunas; não há integração com KMS.
- O estado de limite de taxa e de cache precisa do Redis para estar correto com várias réplicas.
- As visões de análises são mascaradas por papel, mas ficam em cache por token do chamador durante 30 s.
- A reautenticação elevada antes de ações destrutivas exige trabalho no provedor de identidade.

## Sequência técnica recomendada

1. Chaves de campo apoiadas em KMS e maior cobertura de campos (resto da T41).
2. Marcar uma primeira versão para provar a assinatura sem chave e a verificação (T49), e pedir a um revisor que
   verifique o rascunho dos padrões de API.
3. Construir os cabeçalhos `Deprecation` e `Sunset` junto com a primeira descontinuação.
4. Contratar o teste independente (T42) contra uma build de homologação.
