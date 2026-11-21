# Governança, Risco e Conformidade (GRC)

> Resumo de engenharia, não é aconselhamento jurídico, certificação nem auditoria. Ele informa
> o que o repositório mostra em 2026-10-11. Nada aqui afirma acreditação.

## Por onde começar

O pacote de evidências é [spec/compliance](../../../../spec/compliance/index.md). Sua regra básica é a que deve ser
cobrada da equipe: um controle é **Implementado** somente quando código ou configuração o aplica e um teste ou
verificação o fixa; **Implantador** quando a sua organização deve fornecê-lo; **Lacuna** quando nenhuma das duas
condições é verdadeira hoje.

| Documento | Para que usá-lo |
|---|---|
| [control-mapping](../../../../spec/compliance/control-mapping.md) | NIST SP 800-53, HIPAA Security Rule, ISO 27001 Anexo A, GDPR art. 32, cada um marcado como Implementado, Implantador ou Lacuna |
| [dpia-template](../../../../spec/compliance/dpia-template.md) | Avaliação de impacto sobre a proteção de dados, pré-preenchida com os fluxos de dados deste sistema |
| [operations-runbook](../../../../spec/compliance/operations-runbook.md) | Reforço, backup e restauração, chaves, logs, resposta a incidentes |
| [dtac-supporting](../../../../spec/compliance/dtac-supporting.md) | Rascunhos de declarações do NHS DTAC para o fabricante adotar |
| [gds-api-review](../../../../spec/compliance/gds-api-review.md) | Autorrevisão contra os padrões de API do governo do Reino Unido (um rascunho, não conferido com o texto publicado) |
| [api-lifecycle](../../../../spec/compliance/api-lifecycle.md) | Formato de erro (RFC 9457), o que conta como mudança incompatível, as etapas de descontinuação e o prazo de aviso |
| [Avaliação do NHS DTAC v2.0](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md) | Resultado pergunta por pergunta: provavelmente fora do escopo do DTAC, e reprovaria nas seções aplicáveis por falta de evidências organizacionais |
| [plan](../../../../spec/plan.md) | Veredito de prontidão (não está pronto) e o registro de lacunas |
| [tasks](../../../../spec/tasks.md) | A tarefa de cada lacuna, a resolução e os limites declarados |

## Governança

- **Direitos de decisão:** [GOVERNANCE.md](../../../../GOVERNANCE.md), [MAINTAINERS.md](../../../../MAINTAINERS.md),
  [CODEOWNERS](../../../../CODEOWNERS). As regras de contribuição e de assistência de IA estão em
  [CONTRIBUTING.md](../../../../CONTRIBUTING.md) e [AI_STATEMENT.md](../../../../AI_STATEMENT.md).
- **Controle de mudanças:** a especificação é a fonte da verdade; código e testes a citam. A CI roda em dois
  provedores, e `scripts/ci-check.sh` a reproduz. Regra do repositório: não enfraqueça um teste ou portão para fazer
  algo passar.
- **Divulgação:** [SECURITY.md](../../../../SECURITY.md) é o caminho para relatos.

## Risco

A visão de risco está no [resumo do CRO](../diretor-de-riscos/index.md). Em resumo: os maiores riscos residuais são a
segurança não testada por terceiros (T42), a acessibilidade não quantificada (T48) e a classificação dos dados, que é
decisão de quem adota.

## Conformidade: o que é real e o que não é

| Área | Posição |
|---|---|
| Controle de acesso | Implementado e testado: mascaramento por papel, verificações no nível do registro, leituras operacionais restritas a administradores, break-glass com revisão obrigatória por uma segunda pessoa |
| Auditoria | Implementada e testada: encadeada por hash, somente de acréscimo, dentro da transação, auditoria de leituras, incluindo listas, listas de negócios e análises. **Implantador:** ancorar o topo da cadeia fora do banco de dados |
| Direitos de privacidade | Implementados: exportação de acesso pelo titular, exclusão (recusada enquanto houver obrigações em aberto), varredura de retenção. **Implantador:** base legal, cronograma de retenção, registros de tratamento |
| Criptografia | Parcial: AES-256-GCM em nível de campo em duas colunas com chaves locais, sem KMS. A criptografia em repouso e em trânsito é de quem implanta |
| Cadeia de suprimentos | Implementada: `cargo audit`, `cargo deny`, `pnpm audit`, SBOMs, imagens base fixadas por digest, atualizações do Dependabot. **Lacuna:** o fluxo de versão assinada não rodou em uma versão real (T49) |
| Acessibilidade | Parcial: varreduras automatizadas do axe e uma declaração pública honesta. **Lacuna:** sem auditoria manual (T48), traduções não revisadas |
| Garantia independente | **Lacuna:** nenhuma (T42) |
| Lei de e-mail e marketing | Caminho de envio com consentimento em primeiro lugar implementado; o envio real existe apenas como um caminho SMTP para desenvolvimento; o link de cancelamento, a lista de supressão e um webhook de resultado assinado existem, mas nenhum provedor está integrado e o acordo com o operador e a revisão jurídica estão em aberto (T39) |

## Controles que precisam de um responsável na sua organização

1. Revisão e alertas para as concessões de break-glass (alertar sobre `break_glass_granted`).
2. Ancorar o topo da cadeia de auditoria e verificá-lo em intervalos regulares (`GET /api/audits/verify`).
3. Evidência de backup e restauração e ensaio de rotação de chaves.
4. Tratamento de entradas, mudanças e saídas no seu provedor de identidade.
5. Retenção da tabela de auditoria, que cresce com o tráfego de leitura.
6. Proteger o Redis como o banco de dados: ele guarda dados de análises em cache.
7. Mesclar as atualizações do Dependabot e verificar uma versão assinada antes de implantá-la.

## O que recusar-se a aprovar

Qualquer declaração de que o sistema é conforme, certificado, compatível com a HIPAA ou aprovado pelo NHS. A declaração
defensável é mais restrita: os controles estão mapeados, os implementados são testados, e as lacunas são nomeadas e
acompanhadas.
