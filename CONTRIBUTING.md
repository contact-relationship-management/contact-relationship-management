# Contributing

Thank you for considering a contribution. There are several ways to help.

## Time

- Report bugs and gaps as issues, with steps to reproduce.
- Give feedback on the specification in [spec](spec/index.md); see also [RFC.md](RFC.md).
- Improve documentation, tests, or code by pull request.

## Code

1. Read the [specification](spec/index.md) first. It is the source of truth;
   code and tests should cite it.
2. Rust service: `cd contact-relationship-management-service-with-rust`, then
   `cargo test` and `cargo clippy`. Database tests need PostgreSQL (see `compose.test.yaml`).
3. Svelte front end: `cd contact-relationship-management-ui-with-svelte`,
   then `pnpm install` and `pnpm test`.
4. Run `scripts/ci-check.sh` before opening a pull request.
5. Do not weaken a test, expectation, or gate to make something pass.
6. Keep changes small and explain the why in the pull request description.

## AI-assisted contributions

You may use AI tools. If a contribution contains AI-generated content, say so
in the pull request description: which tool, and what it did. You remain
responsible for the submission: understood, explained on request, tested, and
honest. See [AI_STATEMENT.md](AI_STATEMENT.md).

## Money

If you want to support the work financially, email joel@joelparkerhenderson.com.

## Conduct

Be kind and constructive. Contributions are licensed as described in [LICENSE.md](LICENSE.md).
