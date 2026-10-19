# First tagged release: checklist (CRM-T49)

The release workflow (`.github/workflows/release.yml`) and the verification
commands in [operations-runbook.md](operations-runbook.md) have never run on a
real tag. Until they do, the claim "releases are signed and verifiable" is
unproven. This checklist proves it with a throwaway release first.

## Before tagging

Check these in the GitHub repository settings (they cannot be set from the
code):

- [ ] **Actions** are enabled and the default `GITHUB_TOKEN` can be granted
      `packages: write`, `id-token: write` and `attestations: write` (the
      workflow requests them per job).
- [ ] **Packages** are enabled for the repository. Decide whether the images
      are public or private; consumers need pull access to verify.
- [ ] **Artifact attestations** are available for this repository. GitHub has
      limited them to public repositories on some plans; if the repository is
      private and attestations are unavailable, the provenance step will fail
      and you should drop that step or change plan before tagging.
- [ ] **Bump the version first.** The release workflow refuses a tag whose version (the part before any `-`) differs from `version` in `contact-relationship-management-service-with-rust/Cargo.toml` and in `contact-relationship-management-ui-with-svelte/package.json` (`scripts/release-version.sh`). A dry-run tag such as `v0.1.2-rc.1` therefore needs both manifests at `0.1.2`.
- [ ] A **branch or tag protection** rule limits who can push `v*` tags.
- [ ] `main` is green: `scripts/ci-check.sh` passes and the CI workflow ran.
- [ ] The **CHANGELOG** has an entry for this version.

## Dry run with a pre-release tag

Use a tag you are willing to delete, such as `v0.0.1-rc.1`.

```bash
git tag v0.0.1-rc.1
git push origin v0.0.1-rc.1
```

Watch the **Release** workflow. Expect two image jobs (service, ui) and one
SBOM job.

- [ ] Both images build. The service build is slow (a full release compile).
- [ ] Both images are pushed to `ghcr.io/<owner>/<repo>/service` and `/ui`.
- [ ] The signing step succeeds (keyless: it needs the `id-token` permission).
- [ ] The provenance attestation step succeeds.
- [ ] The SBOM job signs the SBOMs and creates the release with `sbom/*`.

If a step fails, fix the workflow on `main`, delete the tag and the release,
and tag again:

```bash
git push --delete origin v0.0.1-rc.1
git tag -d v0.0.1-rc.1
gh release delete v0.0.1-rc.1 --yes
```

## Verify, as a consumer would

Run these from a machine that did not build the image, with `cosign` and the
`gh` CLI installed. Replace `<owner>/<repo>` and the tag.

```bash
cosign verify ghcr.io/<owner>/<repo>/service:v0.0.1-rc.1 \
  --certificate-identity-regexp '^https://github.com/<owner>/<repo>/\.github/workflows/release\.yml@refs/tags/v' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
gh attestation verify oci://ghcr.io/<owner>/<repo>/service:v0.0.1-rc.1 --owner <owner>
cosign verify-blob sbom/service.cdx.json \
  --bundle sbom/service.cdx.json.sigstore.json \
  --certificate-identity-regexp '^https://github.com/<owner>/<repo>/\.github/workflows/release\.yml@refs/tags/v' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

- [ ] The service image verifies. Repeat for the UI image.
- [ ] The attestation verifies.
- [ ] An SBOM verifies against its bundle.
- [ ] Negative test: change one byte of an SBOM and confirm verification
      **fails**. A verifier that cannot fail proves nothing.
- [ ] Pull the image **by digest** and run it once:
      `podman run --rm ghcr.io/<owner>/<repo>/service@sha256:<digest>` should
      exit with a clear error about `DATABASE_URL`, as non-root.

If the verify commands in the runbook need a different flag, change the
runbook, not just this page.

## After the dry run

- [ ] Delete the pre-release tag, release and (if unwanted) the images.
- [ ] Record the result in `spec/tasks.md` under CRM-T49: date, tag, which
      steps passed, any fixes made.
- [ ] Replace "has not run" with the real status in the security, GRC, risk and
      tester briefs (all locales; they say the workflow is untested).
- [ ] Tag the real first release.

## What this does not prove

- That the **action code** pinned by commit hash is safe; it was not audited.
- That consumers will verify. Signing only helps if deploy pipelines refuse
  unverified images.
- That the **build is reproducible**; two builds will not be bit-identical.
- The GDS API review still needs a human reviewer; that is the other open part
  of T49.
