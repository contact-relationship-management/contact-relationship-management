# Tutorial: run the guided tour

This tutorial starts the user interface on fictional data. It requires no database and no other service.

## Prerequisites

- [Git](https://git-scm.com/)
- Node.js 26
- [pnpm](https://pnpm.io/)

Rust and PostgreSQL are not required for this tutorial.

## Step 1: get the code

```bash
git clone https://github.com/contact-relationship-management/contact-relationship-management.git
cd contact-relationship-management/contact-relationship-management-ui-with-svelte
```

## Step 2: start the application

```bash
pnpm install
pnpm dev
```

Open the address that `pnpm dev` prints.

## Step 3: sign in

Sign in as `example@example.com`. This opens the guided tour. Its fictional data is held in the UI server, and no message reaches a real person.

## Step 4: review the tour

The tour page covers six screens.

| Screen | Content |
| --- | --- |
| Contacts | One record for each person, holding a reference and the relationship data |
| Leads | Scores calculated from readable rules |
| Deals | The pipeline by stage |
| Campaigns | Simulated sends and the consent they depend on |
| Tickets | Priorities and service-level deadlines |
| Executive | Summary figures |

## Step 5: change the display settings

The buttons at the top of the page change the language, theme and text size. The interface is available in eight languages. Arabic displays right to left.

## Next steps

- [Install and use](../INSTALL.md) describes how to start the API, PostgreSQL and Redis with one command.
- [The family bridges](tutorial-family-bridges.md) describes the optional connections to other systems. They are off until configured.
- [Comparisons](../COMPARISONS.md) lists other CRM products.
- [The specification](../spec/index.md) defines the system's behaviour.

This is demonstration software. Use synthetic data only.
