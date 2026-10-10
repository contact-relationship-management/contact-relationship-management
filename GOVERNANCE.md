# Governance

## Model

This project has a single maintainer, listed in [MAINTAINERS.md](MAINTAINERS.md).
The maintainer decides what is merged, what ships, and what a silence in the
specification means. Decisions with consequences are recorded in [spec](spec/index.md),
because a decision that exists only in a chat or tool session is not a decision
this project made.

## Honest limitation

Review depth is one person's. Machine gates (tests, clippy, CI in
`.github/workflows` and `.woodpecker.yml`, `scripts/ci-check.sh`) stand in for
the review capacity a larger team would have. The claim is that the maintainer
understands and can explain every merged change, not that every line was
independently re-derived.

## Contributions

Anyone may propose changes by issue or pull request; see [CONTRIBUTING.md](CONTRIBUTING.md).
The maintainer accepts, requests changes to, or declines each one, and gives a
reason when declining.

## Changing this document

By pull request, like everything else. If the project gains more maintainers,
this document changes in the same pull request that adds them.
