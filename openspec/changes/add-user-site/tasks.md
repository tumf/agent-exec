## Implementation Tasks

- [x] Create the framework-free static site structure under `site/`, with shared semantic navigation, responsive CSS, visible focus states, reduced-motion handling, and landing-page sections for value, workflow, comparison, platforms, integrations, and calls to action. (verification: integration - `python3 scripts/validate-site.py` passed)
- [x] Add documentation pages for installation, quick start, job lifecycle, CLI, MCP/HTTP, agent integrations, output contracts, troubleshooting, and release verification, using commands consistent with v0.2.27 behavior. (verification: integration - `python3 scripts/validate-site.py` passed for required page set and internal references)
- [x] Implement a Python standard-library site validator that rejects broken internal links, missing fragment targets, duplicate IDs, missing titles, and images without alternative text. (verification: integration - `python3 -m unittest tests/test_validate_site.py` passed; includes positive and deliberately broken fixtures)
- [x] Add CI validation and a GitHub Pages deployment workflow that uploads only `site/` from `main` after validation, using official GitHub Pages actions and no external build service. (verification: integration - actionlint and `python3 -m unittest tests/test_validate_site.py` passed)


## Future Work

- Deploy the Pages workflow from `main`, fetch `https://tumf.github.io/agent-exec/`, and confirm HTTP 200 with the expected title before archive. The URL returned 404 before the first deployment.
- Capture `/`, `/docs/install.html`, and `/docs/integrations.html` at 390px and 1440px, then record the keyboard walkthrough in `openspec/changes/add-user-site/evidence/manual-site-check.md`.
- Custom domain selection and DNS changes require a separate explicit decision.
- Analytics or telemetry require a separate privacy review and are not part of this site.

## Final Validation

Expected archive gate: `cflx openspec validate add-user-site --archive-gate`

Run `prek run -a`, the site validator, actionlint, and a public Pages read-back before archive.

## Acceptance #1 Failure Follow-up
- [x] Restore terminal newlines and run `prek run -a`. (verification: `prek run -a` passed)
- [x] Strengthen the validator for exactly one main landmark, named landmarks, nonempty titles, relative asset links, and independent missing-fragment coverage. (verification: `python3 -m unittest tests/test_validate_site.py` passed)
- [x] Complete CLI and MCP/HTTP documentation, including endpoint, bind, authentication, CORS, and read-endpoint exposure behavior. (verification: `python3 scripts/validate-site.py` passed)
- [x] Capture 390px/1440px navigation and keyboard evidence; prevent page-level table overflow. (verification: `openspec/changes/add-user-site/evidence/manual-site-check.md`)

## Implementation Blocker #1
- category: external_non_mockable
- summary: GitHub Pages cannot be deployed from this worktree because deployment requires merging to `main` and GitHub Actions execution.
- evidence:
  - `https://tumf.github.io/agent-exec/` returned 404 before the first deployment.
  - `.github/workflows/pages.yml` deploys only from `main`.
- impact: The public HTTP 200 read-back required by proposal.md:48 and spec.md:47-52 cannot be completed locally.
- unblock_actions:
  - Merge the Pages workflow and site changes to `main`.
  - Confirm the GitHub Pages workflow succeeds, then fetch the public URL and record its HTTP 200 title.
- owner: repository maintainer
- decision_due: 2026-07-27
