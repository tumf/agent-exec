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
