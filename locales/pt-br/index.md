# Contact Relationship Management

Software de CRM de código aberto para vendas, marketing e suporte ao cliente.

O Contact Relationship Management acompanha os relacionamentos com clientes e potenciais clientes em vendas, marketing e suporte. Ele mantém um único registro para cada contato e exige consentimento registrado antes de enviar qualquer mensagem. O software não está pronto para produção: ele funciona com dados sintéticos.

## Benefícios

- **Registro único de contato.** Vendas, marketing, suporte e análises usam o mesmo registro.
- **Consentimento registrado.** Nada é enviado a um contato sem consentimento registrado. Cada e-mail inclui um link de cancelamento de inscrição assinado.
- **Menos dados pessoais.** O CRM guarda uma referência a uma pessoa em um serviço de identidade, não uma cópia do nome ou do endereço.
- **Resultados explicáveis.** As pontuações de leads são calculadas com regras legíveis. As previsões são calculadas com etapas de funil ponderadas.
- **Design internacional.** Nomes, endereços, calendários e direções de escrita não dependem do formato de um único país. A interface está disponível em oito idiomas.
- **Especificação pública.** A especificação, o código e os testes são públicos. A documentação informa o que foi construído e o que não foi.

## Recursos

- **Vendas.** Pontuação de leads baseada em regras, funis de negócios em um quadro e previsões ponderadas por etapa.
- **Marketing.** Campanhas segmentadas e sequências de relacionamento. O envio é simulado, a menos que um modo de e-mail esteja configurado.
- **Suporte.** Chamados com prioridades e prazos de nível de serviço, e artigos de base de conhecimento.
- **Análises.** Painéis de taxa de vitórias, funil por etapa, retorno de campanhas, saúde do nível de serviço e valor do tempo de vida do cliente.
- **Qualidade dos dados.** Detecção de duplicatas por probabilidade, com mesclagem e desfazer.
- **Governança.** Registro de auditoria, controle de acesso por papel e suporte a solicitações de acesso e de exclusão pelo titular dos dados.

## Primeiros passos

1. **Obtenha o código.** Clone o repositório. Node.js e pnpm são necessários.
2. **Inicie a aplicação.** No diretório do Svelte, execute `pnpm install` e depois `pnpm dev`.
3. **Entre.** Use `example@example.com`. Nenhum outro serviço é necessário.
4. **Faça o tour.** O tour percorre contatos, leads, negócios, campanhas, chamados e a visão executiva, com dados fictícios.

O [tutorial](../../docs/tutorial-first-tour.md) descreve cada passo. [Instalação e uso](../../INSTALL.md) cobre o serviço completo.

## Comparação

| Projeto | Categoria | Diferença |
| --- | --- | --- |
| [Twenty](../../docs/compare/twenty.md) | CRM de código aberto | Produto completo. Este projeto é uma demonstração menor, orientada por especificação. |
| [EspoCRM](../../docs/compare/espocrm.md) | CRM de código aberto | Mais recursos. Este projeto dá ênfase a regras explicáveis. |
| [SuiteCRM](../../docs/compare/suitecrm.md) | CRM de código aberto | Maduro e grande. Este projeto é uma referência compacta. |
| [Salesforce](../../docs/compare/salesforce.md), [HubSpot](../../docs/compare/hubspot.md) | CRM comercial em SaaS | Este projeto modela a mesma prática. Nenhum código ou dado é compartilhado. |

Este projeto é uma demonstração e não concorre com esses produtos. Veja [todas as comparações](../../COMPARISONS.md).

## Resumos por papel

Os [resumos por papel](para/) oferecem uma página para cada cargo sênior: o que o software faz, o que ele não faz e quais decisões pertencem a esse papel.

## Situação

Este software não está pronto para produção ([caminho para produção](caminho-para-produção/index.md)). Use apenas dados sintéticos. Nenhum e-mail real é enviado, a menos que um modo de e-mail real esteja configurado. Nada neste site é aconselhamento jurídico.
