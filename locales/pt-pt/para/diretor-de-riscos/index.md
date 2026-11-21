# Para o Diretor de Riscos (CRO)

> Resumo de engenharia, não é aconselhamento jurídico, certificação nem auditoria. Ele informa
> o que o repositório mostra em 2026-10-11. Nada aqui afirma acreditação.

## Posição de risco

**No geral: não está pronto para dados regulados.** O software é bem controlado para a sua fase, mas os principais
riscos residuais não são mitigados por testes e processos que só a organização adotante ou terceiros podem fornecer.

## Principais riscos

| # | Risco | Base da classificação | Mitigação no repositório | Residual |
|---|---|---|---|---|
| 1 | Vulnerabilidades não descobertas | Sem teste de intrusão independente (T42) | Testes próprios, portões de cadeia de suprimentos, cabeçalhos/CSP rigorosos | **Alto** até ser testado |
| 2 | Divulgação de campos sensíveis ao papel errado | Regras de acesso complexas | Mascaramento por papel em registos, agregados e análises; um teste de matriz de personas; um vazamento encontrado e corrigido nos detalhes da conta (T47) | Médio: o mascaramento é testado, não verificado de forma independente |
| 3 | Mau uso interno do acesso de emergência | O break-glass eleva leituras | Motivo escrito, limite de 60 minutos, somente leitura, revisão obrigatória por uma segunda pessoa, auditado | Médio: nenhum alerta conectado; sem revogação |
| 4 | Trilha de auditoria alterada | Exige acesso ao banco de dados | Cadeia de hash mais acionador somente de acréscimo; informa a primeira quebra | Baixo-Médio: o hash do topo deve ser ancorado fora do banco |
| 5 | Marketing ilícito | Erros de consentimento | O caminho de envio aplica o consentimento, não a interface; histórico de consentimento imutável | Médio até que um provedor real seja integrado e revisado (T39) |
| 6 | Disponibilidade / abuso | API pública | Limite de taxa e bloqueio, compartilhados via Redis | Médio: os limites voltam a ser por réplica se o Redis falhar |
| 7 | Não conformidade de acessibilidade | Dever do setor público | Varreduras automatizadas do axe; declaração de acessibilidade honesta | **Não quantificado** até a auditoria manual (T48) |
| 8 | Cadeia de suprimentos | Dependências e imagens | `cargo deny/audit`, `pnpm audit`, SBOM, digests fixados e atualizados pelo Dependabot, um fluxo de versão com assinatura sem chave | Médio: a assinatura não rodou em uma versão real (T49) |
| 9 | Registos duplicados por novas tentativas | As chaves de idempotência são opcionais na API (T52) | Reprodução para clientes que enviam a chave; a interface incluída sempre a envia | Baixo |
| 10 | Classificação incorreta de dados | Quem implanta decide o que é um "contacto" | Nota de escopo e modelo de DPIA | Depende de você |

## Lacunas que permanecem abertas por natureza

T39 (provedor real de e-mail), T42 (garantia independente), T48 (auditoria manual de acessibilidade), T49 (uma primeira
versão assinada, a verificação do rascunho da API por um revisor). Veja [tasks.md](../../../../spec/tasks.md) e
[plan.md](../../../../spec/plan.md).

## Ações de risco sugeridas

Registre formalmente os riscos 1, 7 e 10; exija a T42 antes de qualquer dado real; defina um responsável pela revisão e
pelo alerta do break-glass; defina a cadência de ancoragem do topo da auditoria.
