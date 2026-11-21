# Para o Diretor de Pessoas (CPO)

> Resumo de engenharia, não é aconselhamento jurídico, certificação nem auditoria. Ele informa
> o que o repositório mostra em 2026-10-11. Nada aqui afirma acreditação.

## Por que as pessoas importam para este produto

O acesso a dados pessoais é decidido por papel. O software aplica o que o token de uma pessoa diz; **quem recebe qual
papel, e quando ele é removido, é um processo de pessoas** que este repositório não pode fazer por você.

## O que o software oferece aos seus processos

- **Visibilidade por papel:** alguns papéis veem valores mascarados em canais de contato, valores de negócios e visões
  de dinheiro; gerentes, administradores e camadas de serviço veem mais ([auth](../../../../spec/auth.md)).
- **Leituras auditadas:** ler dados pessoais deixa um rastro que nomeia a pessoa. A equipe deve ser informada disso
  antes da entrada em produção.
- **Acesso de emergência:** uma pessoa pode obter uma elevação de leitura justificada de até 60 minutos; outra pessoa
  deve revisá-la depois. Treine os dois lados.
- **Logout por inatividade** após 15 minutos no navegador.
- **Acessibilidade:** uma declaração pública e verificações automatizadas; a auditoria manual (T48) ainda precisa de
  usuários com deficiência e de testadores de tecnologia assistiva.
- **Idiomas:** a interface está traduzida em 8 localidades; as traduções **não** foram revisadas por falantes nativos.

## Suas responsabilidades

| Área | Ação |
|---|---|
| Entradas, mudanças e saídas | Conceda e remova papéis pelo seu provedor de identidade; o CRM recusa tokens expirados, mas não gerencia contas |
| Treinamento | Proteção de dados, regras de consentimento, uso e revisão do break-glass, prática acessível |
| Políticas | Uso aceitável, aviso de monitoramento (as leituras são auditadas), via disciplinar para o mau uso do acesso de emergência |
| Capacidade de revisão | Nomeie quem revisa as concessões de break-glass e quem cobre as ausências |
| Contratação | Papéis de que o programa precisa: contato com o testador independente, responsável pela segurança clínica se estiver no escopo do NHS, responsável pela proteção de dados, responsável pela acessibilidade |
| Bem-estar | Apoie a equipe que lida com casos sensíveis; o CRM não é um sistema de gestão de casos, mas pode guardar dados de relacionamento sensíveis |

## Itens em aberto que precisam de pessoas

Auditoria manual de acessibilidade com usuários de tecnologia assistiva (T48), revisão das traduções por falantes
nativos e a responsabilidade pelos deveres de revisão e de alerta descritos acima. Veja
[tasks.md](../../../../spec/tasks.md).
