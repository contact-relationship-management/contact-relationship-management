# Para o Diretor de Estratégia

> Resumo de engenharia, não é aconselhamento jurídico, certificação nem auditoria. Ele informa
> o que o repositório mostra em 2026-10-11. Nada aqui afirma acreditação.

## A questão estratégica

Para que serve isto? Hoje é um **CRM de referência orientado por especificação**, com uma camada de segurança,
privacidade e auditoria incomumente completa. Não é um produto com clientes e não concorre com CRMs maduros
([COMPARISONS.md](../../../../COMPARISONS.md)). A escolha à sua frente é manter isto como referência ou investir para
torná-lo implantável em ambientes regulados.

## O que é realmente diferente

| Ponto forte | Por que poderia importar | Quão sólido é |
|---|---|---|
| Especificação primeiro: uma só especificação conduz o serviço em Rust e a interface em Svelte ([spec](../../../../spec/index.md)) | Mais barato de auditar, portar e manter consistente | Real; os testes citam a especificação |
| Marketing com consentimento em primeiro lugar, aplicado no caminho de envio | Exigência rígida para compradores da saúde e do setor público | Real, mas ainda não existe um remetente real (T39) |
| Pontuação e previsão de leads explicáveis, baseadas em regras | Compradores que precisam justificar decisões sobre pessoas | Real; deliberadamente sem aprendizado de máquina |
| Contatos consumidos de um serviço de identidade, não duplicados | Menos dados pessoais guardados, menor superfície de vazamento | Real; depende da existência desse serviço |
| Auditoria que revela adulteração, mascaramento por papel, acesso de emergência justificado | Os controles sobre os quais compradores regulados perguntam primeiro | Implementados e testados; não verificados de forma independente |

## O que não é diferente

A amplitude de recursos. Orçamentos, contratos, telefonia, pontuação por aprendizado de máquina e pagamentos estão fora
do escopo ([scope](../../../../spec/scope.md)). Quem escolhe um CRM por recursos escolherá outro.

## Opções

| Opção | O que significa | Custo e risco | Evidência necessária |
|---|---|---|---|
| A. Continuar como referência | Mantê-lo como biblioteca de ensino e de padrões | O mais baixo; o valor é de reputação | Nada além do que existe hoje |
| B. Nicho de setor regulado | Concluir os itens em aberto e vendê-lo ou dar suporte a ele para a saúde e órgãos públicos | O mais alto; exige fornecedor, termos jurídicos, testes externos, garantia contínua | Teste independente (T42), auditoria de acessibilidade (T48), e-mail real (T39) |
| C. Componentes, não produto | Oferecer a cadeia de auditoria, o mascaramento e os padrões de break-glass como peças reutilizáveis | Moderado; exige empacotamento e documentação | Trabalho de extração; nenhuma nova certificação |

**Recomendação: faça A agora e financie o teste mais barato de B antes de se comprometer.** A avaliação do NHS concluiu
que o produto provavelmente fica fora do escopo do DTAC
([avaliação](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md)),
portanto o caminho até o mercado não está definido. Gaste primeiro para descobrir qual comprador precisa disto e o que
ele exigiria, não em certificações que ninguém pediu.

## Roteiro, na ordem que as evidências sugerem

1. Teste de segurança independente e auditoria manual de acessibilidade (T42, T48): eles decidem se B é possível.
2. Provedor real de e-mail (T39), quando um fornecedor e os termos de tratamento de dados forem escolhidos; o ponto de
   integração do remetente e um caminho SMTP existem e rodam contra um servidor de teste local, com link de
   cancelamento assinado, lista de supressão e um webhook genérico de resultado assinado, mas nenhum provedor está
   integrado ainda ([roadmap](../../../../spec/roadmap.md)).
3. Provar a assinatura de versões com uma primeira versão marcada (T49): requisito mínimo para compras. O fluxo está
   construído; não rodou.
4. Só então os recursos de produto do roteiro (atribuição, territórios, relógios de SLA em horário comercial, portal
   público de base de conhecimento).

## Riscos estratégicos

- **Afirmar mais do que é verdade.** A forma mais rápida de perder um comprador regulado é uma declaração de
  conformidade que as evidências não sustentam. Veja o [resumo de GRC](../governança-risco-conformidade/index.md).
- **Manutenção.** O trabalho de segurança não termina: atualizações de dependências, renovação de digests, rotação de
  chaves, novos testes. A opção B compromete você com isso.
- **Dependência de um serviço de identidade** que está fora deste repositório.
- **Um único mantenedor.** O projeto tem um só mantenedor e diz que a profundidade da revisão é a de uma pessoa
  ([MAINTAINERS.md](../../../../MAINTAINERS.md), [GOVERNANCE.md](../../../../GOVERNANCE.md)). A opção B exige mais de
  uma pessoa que possa responder pelo trabalho de segurança.

## Decisões para você

1. Qual entre A, B ou C, e a data de revisão dessa escolha.
2. Quem é o primeiro comprador-alvo, descoberto por conversas antes de gastar.
3. Se vai financiar a garantia independente agora (veja o [resumo do CEO](../diretor-executivo/index.md)).
