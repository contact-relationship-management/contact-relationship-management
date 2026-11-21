# Caminho para produção

> O Contact Relationship Management não está pronto para produção. Esta página diz o que produção significa aqui e até
> onde cada parte avançou.

O software funciona com dados sintéticos. O e-mail é simulado, a menos que um modo de e-mail real esteja configurado.
Não coloque dados pessoais reais nele.

## O que produção significa

Uma implantação em produção guarda dados de pessoas reais e envia mensagens reais a elas. Para este projeto, isso
exige tudo o que segue:

- **Login real.** Toda requisição é autenticada, e as regras de acesso correspondem aos papéis da organização que
  implanta.
- **Envio lícito.** Cada mensagem tem consentimento registrado e uma base legal segundo a lei de cada lugar para onde
  é enviada, e passa por um provedor de e-mail real que trata devoluções, reclamações e cancelamentos de inscrição.
- **Garantia independente.** Pessoas de fora do projeto testaram a segurança e a acessibilidade, e os achados foram
  corrigidos ou aceitos.
- **Operações.** Backups que foram restaurados, monitoramento, um processo de incidentes e alguém de plantão.
- **Evidência de estabilidade.** Execuções longas sob carga realista, em hardware semelhante ao da implantação, sem
  erros nem lentidão.

A prontidão para produção é decidida a cada implantação. Quem implanta pode fechar alguns portões apenas para a sua
própria organização e os seus próprios destinatários. A avaliação de engenharia por trás desta página é
[spec/plan.md](../../../spec/plan.md). Os portões abertos estão em [spec/tasks.md](../../../spec/tasks.md).

## Prontidão

Cada porcentagem é uma estimativa feita em 2026-11-13 a partir da lista de tarefas, não uma medição.

- **Autenticação e controle de acesso** (70%): Login por tokens PASETO ou Keycloak/OIDC, regras de acesso baseadas em
  atributos com verificações no nível do registro e um modo de produção que se recusa a iniciar sem autenticação estão
  construídos. Nenhuma implantação real os ativou com atributos próprios, e a matriz de aplicação ainda não cobre todos
  os handlers (CRM-G1).
- **Entrega de e-mail** (40%): Um remetente simulado é o padrão, e existe um remetente SMTP. Não há adaptador para
  provedor de e-mail com webhooks de devolução e reclamação, nem lista de supressão (CRM-T39).
- **Proteção de dados e lei de marketing por e-mail** (15%): Registros de consentimento, acesso pelo titular, exclusão
  e retenção estão construídos. A lei de nenhuma região foi lida para este projeto, e a revisão do caminho real de envio
  está aberta (CRM-G2, [regulatory](../../../spec/regulatory.md)).
- **Garantia independente de segurança** (10%): Um escopo de teste de intrusão está escrito. Nenhum teste de intrusão,
  análise formal de riscos ou acreditação foi feito (CRM-T42). As auditorias de segurança por IA não são garantia
  independente.
- **Acessibilidade** (50%): As verificações automatizadas do axe-core contra a WCAG 2.2 A e AA passam em todas as rotas
  de página. Nenhuma auditoria manual com uso apenas do teclado, leitores de tela ou zoom de 400% foi feita (CRM-T48).
- **Traduções** (30%): A interface está disponível em oito idiomas. Os textos em idiomas diferentes do inglês foram
  escritos por máquina, e nenhum falante os revisou.
- **Operações** (30%): Um manual de operações cobre backup e restauração, gestão de chaves, criptografia em repouso,
  resposta a incidentes e retenção de logs. Nenhuma restauração foi testada, e a lista de verificação de entrada em
  produção do manual está aberta.
- **Desempenho e estabilidade** (20%): Existem testes de carga e um teste de estabilidade de longa duração. O teste de
  estabilidade rodou por dois minutos contra a pilha de demonstração; não existe execução longa nem linha de base em
  hardware semelhante ao de produção.
- **Integridade das versões** (85%): As imagens de versão são multiarquitetura, assinadas e atestadas, e suas
  assinaturas são verificadas. Actions e imagens são fixadas por digest, e o histórico é varrido em busca de segredos.
  A metade amd64 de uma imagem de versão baixada não foi verificada, e ainda não há um segundo host de código
  (CRM-T192).
- **Integração contínua** (100%): Um script executa todas as verificações no GitHub Actions e no Woodpecker. As duas
  bases de código em Rust e a interface têm pisos de 95% das linhas; portões de navegador, lint, cadeia de suprimentos,
  mutação das linhas alteradas e mutação completa semanal estão em vigor (CRM-T94).
- **Salvaguardas da demonstração** (100%): Cada visitante do tour recebe uma cópia separada dos dados de demonstração,
  limitada a 200 cópias. O tour vem desativado por padrão; defina `CONTACT_RELATIONSHIP_MANAGEMENT_DEMO_TOUR=on` para
  oferecê-lo (CRM-T92).

## Como esta página é mantida

Quando um portão fecha, altere a estimativa e a sua evidência no mesmo commit do trabalho e vincule a tarefa.
