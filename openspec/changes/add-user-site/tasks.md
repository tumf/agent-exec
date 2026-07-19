## Implementation Tasks

- [ ] Create the framework-free static site structure under `site/`, with shared semantic navigation, responsive CSS, visible focus states, reduced-motion handling, and landing-page sections for value, workflow, comparison, platforms, integrations, and calls to action. (verification: integration - `python3 scripts/validate-site.py` validates `site/index.html`, shared assets, required landmarks, titles, IDs, and links)
- [ ] Add documentation pages for installation, quick start, job lifecycle, CLI, MCP/HTTP, agent integrations, output contracts, troubleshooting, and release verification, using commands consistent with v0.2.27 behavior. (verification: integration - `python3 scripts/validate-site.py` checks the required page set and all internal references; command examples are cross-checked by targeted assertions in the validator)
- [ ] Implement a Python standard-library site validator that rejects broken internal links, missing fragment targets, duplicate IDs, missing titles, and images without alternative text. (verification: integration - `python3 -m unittest tests/test_validate_site.py` runs positive and deliberately broken fixtures against `scripts/validate-site.py`)
- [ ] Add CI validation and a GitHub Pages deployment workflow that uploads only `site/` from `main` after validation, using official GitHub Pages actions and no external build service. (verification: integration - `go run github.com/rhysd/actionlint/cmd/actionlint@v1.7.11 .github/workflows/*.yml` and workflow assertions in `tests/test_validate_site.py`)
- [ ] Update `README.md` and Cargo package metadata with the final public Pages URL after deployment succeeds. (verification: e2e - fetch the public Pages URL, assert HTTP 200 and expected title, then verify `README.md` and `Cargo.toml` contain that exact URL)
- [ ] Check representative mobile and desktop layouts, keyboard navigation, contrast, reduced motion, and no-JavaScript usability. (verification: manual - serve `site/` with `python3 -m http.server --directory site 8000`, capture `/`, `/docs/install.html`, and `/docs/integrations.html` at 390px and 1440px, and record the keyboard walkthrough in `openspec/changes/add-user-site/evidence/manual-site-check.md`)

## Future Work

- Custom domain selection and DNS changes require a separate explicit decision.
- Analytics or telemetry require a separate privacy review and are not part of this site.

## Final Validation

Expected archive gate: `cflx openspec validate add-user-site --archive-gate`

Run `prek run -a`, the site validator, actionlint, and a public Pages read-back before archive.
