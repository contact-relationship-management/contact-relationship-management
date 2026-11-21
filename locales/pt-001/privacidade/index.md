# Aviso de privacidade: o que fica guardado no seu navegador

Este aviso cobre duas coisas: este site (`contact-relationship-management.github.io`) e a interface de usuário do
Contact Relationship Management como este repositório a distribui. Ele lista tudo o que cada uma das duas guarda no
seu navegador e por quê.

Uma organização que executa o CRM trata dados pessoais sobre seus contatos. Esse tratamento é coberto pelo aviso de
privacidade da própria organização, não por este ([spec/regulatory.md](../../../spec/regulatory.md)).

## O que é guardado e por quê

Cada item guarda uma escolha que você fez: um tema, um idioma e uma localidade, um tamanho de texto, um fuso horário
ou o layout de uma tabela. Nenhum deles guarda seu nome, endereço de e-mail, endereço IP ou qualquer identificador, e
nenhum é usado para rastrear você, medir você ou exibir publicidade a você. Cada um existe para que a próxima página
que você abrir use a mesma escolha, sem que você precise fazê-la de novo.

Nada é guardado até que você faça uma escolha. Um visitante que não altera nada não tem nada guardado.

### Neste site

| Nome | Tipo | O que guarda | Mantido por |
| --- | --- | --- | --- |
| `lily-theme` | Cookie e `localStorage` | `system` ("Seguir este dispositivo") ou o nome de um tema, como `dark` | Cookie: 400 dias. `localStorage`: até você limpar |
| `lily-text-size` | `localStorage` | O tamanho de texto que você escolheu | Até você limpar |
| `lily-time-zone` | `localStorage` | O fuso horário que você escolheu | Até você limpar |

O tema fica guardado em um cookie e no `localStorage`, para continuar disponível caso seu navegador limpe um dos dois.
As páginas do site são arquivos estáticos, e nenhum programa do site lê o cookie. Por ser um cookie, seu navegador o
envia ao GitHub Pages, que hospeda o site, a cada requisição ao site.

O idioma e a localidade do site fazem parte do endereço da página, como `/pt-001/`, portanto nada é guardado para eles.

### Na interface de usuário do CRM

| Nome | Tipo | O que guarda | Mantido por |
| --- | --- | --- | --- |
| `crm-theme` | Cookie | `system` ("Seguir este dispositivo") ou o nome de um tema | 400 dias |
| `mxi.crm.locale` | `localStorage` | O idioma e a localidade que você escolheu, como `pt-001` | Até você limpar |
| `lily-text-size` | `localStorage` | O tamanho de texto que você escolheu | Até você limpar |
| `crm-grid-<table>` | `localStorage` | Larguras das colunas, colunas ocultas e ordenação de uma tabela | Até você limpar |
| `crm-grid-order-<table>` | `localStorage` | A ordem das colunas de uma tabela | Até você limpar |
| `__Host-mxi_session` | Cookie | Um identificador aleatório da sua sessão autenticada | Até você sair ou a sessão terminar |

O servidor do CRM lê o cookie `crm-theme` para enviar cada página já no seu tema, de modo que ela não apareça primeiro
em outro. O CRM nunca o lê para nenhum outro fim.

O cookie de sessão não é uma escolha. Ele é definido quando você entra, os scripts do navegador não conseguem lê-lo
(`HttpOnly`), e sem ele você não consegue permanecer autenticado.

## Quem recebe

Esses itens permanecem no seu navegador. Nada neles é enviado a terceiros, exceto o cookie de tema do site, que seu
navegador envia ao host do site como descrito acima. Os cookies são próprios (`SameSite=Lax`) e `Secure` em uma página
`https`.

## Como removê-los

Para mudar uma escolha, faça-a de novo. Para remover tudo, limpe os dados deste site nas configurações do seu
navegador: a página seguinte voltará a usar os padrões, isto é, seguir a configuração clara ou escura do seu
dispositivo, o tamanho de texto padrão e o idioma padrão da página.
