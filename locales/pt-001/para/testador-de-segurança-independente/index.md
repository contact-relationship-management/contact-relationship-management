# Testador de Segurança Independente

> Resumo de engenharia, não é aconselhamento jurídico, certificação nem auditoria. Ele informa
> o que o repositório mostra em 2026-10-11. Nada aqui afirma acreditação.

## Por que esta página existe

Todo controle de segurança deste projeto foi construído e testado por seus autores (CRM-T42). Isso mostra que os
controles funcionam como os autores os entenderam. Não mostra que nada falta. Hospitais e compradores governamentais
costumam exigir um teste feito por alguém independente antes de guardarem dados reais, e a afirmação honesta até lá é
"construído com alto padrão, não testado por terceiros" ([resumo do CEO](../diretor-executivo/index.md),
[resumo de segurança](../diretor-de-segurança/index.md)).

Esta página é o que se entrega a uma empresa de testes e o que se pode esperar em troca.

## O que passar significa e o que não significa

Um relatório limpo diz que uma equipe nomeada, em um prazo nomeado, com um escopo nomeado, não encontrou problemas
graves. Ele **não** é uma certificação, nem uma acreditação, nem uma garantia. Não descreva o sistema como "seguro",
"certificado" ou "testado por intrusão conforme um padrão" além do que o relatório afirma
([resumo de GRC](../governança-risco-conformidade/index.md)).

## Escolhendo o testador

- Independente dos autores e de quem quer que tenha configurado a implantação.
- Composto por testadores nomeados e experientes, com a acreditação pertinente ao seu mercado onde houver uma (por
  exemplo, a CREST ou o esquema CHECK do NCSC para trabalhos do governo do Reino Unido). Pergunte qual acreditação cobre
  as pessoas que farão o trabalho, não apenas a empresa.
- Concorda por escrito com escopo, regras de engajamento, confidencialidade, tratamento de dados e responsabilidade
  antes de qualquer teste começar. O contrato deve ser acordado pela equipe jurídica
  ([resumo do CLO](../diretor-jurídico/index.md)).

## O que testar

| Área | O que os testadores devem tentar | Onde o design é descrito |
|---|---|---|
| Autenticação | Tokens forjados, expirados, reproduzidos e com audiência errada; rebaixamento para não autenticado; a verificação de inicialização que se recusa a rodar com a autenticação desligada | [auth](../../../../spec/auth.md) |
| Autorização | Alcançar registros ou ações que um papel não deveria: escalonamento horizontal e vertical, verificações no nível do registro, ações destrutivas, leituras restritas a administradores (auditoria, retenção, revisão de break-glass) | [auth](../../../../spec/auth.md) |
| Exposição de dados | Valores mascarados vazando por listas, leituras de detalhe agrupadas, análises, agregados, mensagens de erro, cache ou a chave de cache | [tasks](../../../../spec/tasks.md) (T32, T47) |
| Acesso de emergência | Obter, estender ou abusar do break-glass; revisar a própria concessão; revisar duas vezes | [auth](../../../../spec/auth.md) (T44) |
| Auditoria | Alterar, excluir ou reordenar o histórico; contornar a cadeia; ocultar leituras | [audit](../../../../spec/audit.md) |
| Resistência a abuso | Contornar o limite de taxa e o bloqueio, bloquear outros usuários, falsificar cabeçalhos para a chave do cliente | T43 |
| Tratamento de entrada | Injeção (SQL, JSON, cabeçalhos), corpos superdimensionados, cursores e chaves de idempotência malformados, reprodução ou colisão de chaves entre chamadores | T51, T52 |
| Front-end web | Cross-site scripting, falsificação de requisição, tratamento de sessão, o proxy backend-for-frontend, política de segurança de conteúdo, clickjacking | UI `hooks.server.ts` |
| Infraestrutura como distribuída | Imagens de contêiner, portas expostas, alcance do Redis e do PostgreSQL, tratamento de segredos, padrões de configuração | [operations-runbook](../../../../spec/compliance/operations-runbook.md) |
| Cadeia de suprimentos | Procedência de dependências e imagens, os SBOMs, builds não assinadas | [control-mapping](../../../../spec/compliance/control-mapping.md) |
| Lógica de negócio | Contorno do consentimento no caminho de envio, exclusão que deixa dados, manipulação da pontuação, criação duplicada | [regulatory](../../../../spec/regulatory.md) |

**Fora do escopo, salvo acordo em contrário:** o serviço de identidade que emite os tokens (ele fica fora deste
repositório), a plataforma de hospedagem e a negação de serviço em volume.

## O que entregar aos testadores

1. Uma **build de homologação** que corresponda ao que rodará em produção: configuração de produção, autenticação
   ligada, TLS, um Redis e um PostgreSQL reais. Não teste os padrões da demonstração.
2. **Apenas dados sintéticos.** A tarefa de carga inicial cria uma carteira de negócios sem pessoas reais. Dados reais
   de pacientes ou cidadãos nunca devem estar em um ambiente de teste.
3. **Uma conta por papel**, para que a autorização possa ser testada de cada lado. A política de referência é
   `config/abac-policy.reference.json`.
4. Os documentos vinculados acima, o documento OpenAPI em `/api-docs/openapi.json` e acesso de leitura ao
   repositório, para que o teste possa combinar caixa-preta e revisão de código.
5. As **fragilidades conhecidas**, para que os testadores gastem o tempo no que ainda não está escrito:
   - a assinatura de versões está construída, mas não rodou em uma versão real (T49);
   - a criptografia de campo cobre duas colunas com chaves locais, sem KMS;
   - a reautenticação elevada não está construída;
   - o break-glass não pode ser revogado, apenas expirar, e nenhum alerta está conectado;
   - as recargas de política aparecem nas análises em cache por até 30 segundos;
   - o logout por inatividade é do lado do cliente;
   - o estado do limite de taxa volta a ser por réplica se o Redis falhar.
   Peça que confirmem ou refutem cada uma, não que as pulem.

## Regras de engajamento a acordar por escrito

- Testadores nomeados, datas e endereços de origem; um contato 24 horas do seu lado.
- O que pode ser tentado (por exemplo, nenhuma destruição do armazenamento de dados do teste sem aviso) e o que deve
  parar na prova de exposição.
- Quem é informado imediatamente de um achado crítico, e como, antes do relatório final.
- Como o relatório, as evidências e quaisquer anotações dos testadores são guardados e destruídos.

## O que você deve receber de volta

- Um relatório com escopo, método, datas, nomes dos testadores e acreditação.
- Cada achado com gravidade, evidência, passos para reproduzir, componente afetado e correção recomendada.
- Uma declaração clara do que **não** foi testado.
- Um novo teste dos achados corrigidos, incluído no contrato ou com preço definido.

## Depois do relatório

1. Triar cada achado em `spec/tasks.md` como uma tarefa com responsável e data.
2. Corrigir os achados críticos e altos antes de qualquer dado real e depois pedir que os testadores confirmem as
   correções.
3. Acrescentar um teste de regressão para cada correção. A regra do projeto é não enfraquecer um teste ou portão para
   fazer algo passar.
4. Registrar o resultado com honestidade: a data do relatório, o escopo, os achados e a situação deles. Atualizar o
   plano de prontidão ([plan](../../../../spec/plan.md)) e o pacote de conformidade
   ([compliance](../../../../spec/compliance/index.md)).
5. Testar de novo após mudanças relevantes e conforme um cronograma definido pelo responsável pelo risco. Um teste é
   um retrato, não um estado.

## Ordem do trabalho

1. Primeiro, fechar o que é barato e conhecido, para que os testadores não gastem o orçamento redescobrindo: uma
   primeira versão assinada (T49), alertas para o break-glass e para falhas de verificação da auditoria, Redis atrás de
   TLS e credenciais.
2. Preparar a build de homologação, as contas e os dados iniciais.
3. Acordar o escopo, o contrato e as regras de engajamento.
4. Testar; receber os achados críticos imediatamente.
5. Corrigir, testar de novo e registrar.
6. Só então atualizar as declarações externas, e apenas até o que o relatório sustenta.

## Decisões para você

1. Quem contrata e quem é dono dos achados (veja o [resumo do CRO](../diretor-de-riscos/index.md)).
2. Qual acreditação o seu mercado espera dos testadores.
3. O orçamento e a data, definidos pelo que um atraso custa e não pelo que é conveniente.
4. Se vai testar uma vez antes do lançamento ou em um cronograma recorrente.
