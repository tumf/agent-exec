## Implementation Tasks

- [x] Create the framework-free static site under `site/`, with semantic navigation, responsive CSS, visible focus states, reduced-motion handling, landing-page sections, and no JavaScript dependency. (verification: integration - `python3 scripts/validate-site.py` passed)
- [x] Add documentation for installation, quick start, job lifecycle, CLI, MCP/HTTP, agent integrations, output contracts, troubleshooting, and release verification. (verification: integration - `python3 scripts/validate-site.py` passed for the required page set and internal references)
- [x] Implement a Python standard-library validator for internal links, fragment targets, duplicate IDs, titles, HTML language, landmarks, relative assets, and image alternative text. (verification: integration - `python3 -m unittest tests/test_validate_site.py` passed with positive and broken fixtures)
- [x] Add CI validation and a GitHub Pages workflow that deploys only `site/` from `main` with official GitHub Pages actions. (verification: integration - `actionlint` and `python3 -m unittest tests/test_validate_site.py` passed)
- [x] Align CLI, MCP/HTTP, quick-start, and output-contract examples with the current v0.2.27 implementation. (verification: integration - `python3 -m unittest tests/test_validate_site.py` passed)

## Future Work

- After integration to `main`, confirm the Pages workflow succeeds and read back `https://tumf.github.io/agent-exec/` with HTTP 200 and the expected title.
- Custom domains, DNS changes, analytics, and telemetry require separate decisions.

## Final Validation

Archive validation is the authoritative final OpenSpec gate.
Expected archive gate: `cflx openspec validate add-user-site --archive-gate`
