# Your first tour

This tutorial takes you from nothing to a running Contact Relationship Management, on fictional data, with no database and no other service. Allow about ten minutes.

## Before you start

You need [Git](https://git-scm.com/), Node.js 26 and [pnpm](https://pnpm.io/). You do not need Rust, PostgreSQL or an account anywhere.

## Step 1: get the code

```bash
git clone https://github.com/contact-relationship-management/contact-relationship-management.git
cd contact-relationship-management/contact-relationship-management-ui-with-svelte
```

## Step 2: start the app

```bash
pnpm install
pnpm dev
```

Open the address that `pnpm dev` prints.

## Step 3: sign in

Sign in as `example@example.com`. This is the guided tour: its fictitious data is held in the UI server, so nothing you do reaches a real person.

## Step 4: take the tour

The tour page walks through six screens, in the order a customer usually meets them.

| Screen | What to look for |
| --- | --- |
| Contacts | One record for each person, holding a reference and the relationship, not a copy of their details |
| Leads | A score built from rules you can read |
| Deals | The pipeline, stage by stage |
| Campaigns | Sends that are simulated, and the consent they depend on |
| Tickets | Priorities and service deadlines |
| Executive | The numbers together in one view |

## Step 5: make it yours

While the app is open, try the buttons at the top of the page: change the language, the theme and the text size. The interface comes in eight languages, and Arabic reads right to left.

## Where next

- Run the real service: [Install and use](../INSTALL.md) starts the API, PostgreSQL and Redis with one command.
- See how it fits with other systems: [the family bridges](tutorial-family-bridges.md). They stay off until you configure them.
- Weigh it against other CRMs: [Comparisons](../COMPARISONS.md).
- Read the rules behind it: [the specification](../spec/index.md).

Remember that this is demo software: use synthetic data only.
