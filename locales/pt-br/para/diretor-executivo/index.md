# Para o Diretor Executivo (CEO)

> Resumo de engenharia, não é aconselhamento jurídico, certificação nem auditoria. Ele informa
> o que o repositório mostra em 2026-10-11. Nada aqui afirma acreditação.

## A decisão em um parágrafo

Este é um CRM de nível de demonstração, com uma base de segurança e privacidade incomumente sólida. Ele **não está
pronto** para guardar dados reais de pacientes ou cidadãos, e o próprio repositório diz isso
([plan.md](../../../../spec/plan.md)). Adotá-lo é decidir financiar o trabalho externo restante, não ligá-lo.

## O que existe

- Um CRM funcional em vendas, marketing, suporte e análises, com uma API em Rust e um front-end web, construído
  conforme uma especificação escrita.
- Marketing com consentimento em primeiro lugar, controle de acesso que oculta valores sensíveis dos papéis que não
  deveriam vê-los, uma trilha de auditoria que revela adulteração, acesso pelo titular e exclusão para pedidos de
  privacidade, e acesso de emergência ("break-glass") justificado, limitado no tempo e revisado.
- 54 de 58 itens de trabalho acompanhados estão concluídos ([tasks.md](../../../../spec/tasks.md)).

## O que separa isto do uso real

| Bloqueio | Por que importa | Quem pode resolver |
|---|---|---|
| Sem entrega real de e-mail (T39) | O marketing é simulado; um provedor real exige contratos e um acordo de tratamento de dados | Você escolhe o fornecedor; o jurídico assina |
| Sem teste de segurança independente (T42) | Hospitais e compradores governamentais costumam exigir um; nossos próprios testes não são independentes | Uma empresa externa |
| Sem auditoria manual de acessibilidade (T48) | Órgãos públicos precisam publicar uma declaração de acessibilidade que possam sustentar | Pessoas que usam tecnologia assistiva |
| A assinatura de versões está construída, mas não rodou em uma versão real, e nenhum revisor verificou o rascunho dos padrões de API (T49) | As listas de verificação de compras pedem ambos | Engenharia (marcar uma primeira versão); um revisor externo |

## Decisões que são suas

1. **Qual mercado primeiro.** A avaliação dos Digital Technology Assessment Criteria do NHS
   ([docs](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md))
   conclui que o produto provavelmente fica fora do escopo do DTAC e reprovaria nas seções aplicáveis por falta de
   evidências organizacionais (responsável nomeado pela segurança clínica, certificações). Escolha o caminho antes de
   gastar com ele.
2. **Se vai financiar a garantia independente** (T42, T48). Sem ela, a afirmação honesta é "construído com alto padrão,
   não testado por terceiros".
3. **Quem é dono do risco** quando dados reais forem guardados. Veja os resumos do [CRO](../diretor-de-riscos/index.md)
   e do [CLO](../diretor-jurídico/index.md).

## O que não dizer externamente

Não afirme conformidade, certificação, "compatível com a HIPAA" nem "aprovado pelo NHS". O pacote de evidências
([compliance](../../../../spec/compliance/index.md)) sustenta uma declaração mais restrita e verdadeira: os controles
estão mapeados, os implementados são testados e as lacunas são nomeadas.
