# Design: user-facing landing page and documentation site

## Decision

Use static HTML and CSS in `site/`, validated by a Python standard-library script and published by GitHub Pages.

## Why this design

- The product is a Rust CLI. A Node/SPA toolchain would add an unrelated dependency and release surface.
- The content is mostly stable reference material already present in repository Markdown.
- Static files render without JavaScript, are fast, portable, and simple to audit.
- GitHub Pages is already colocated with source and releases. It avoids a new hosting account and backend.

## Information architecture

```text
site/
  index.html                 landing page
  docs/
    index.html               documentation overview
    install.html             install and verify
    quickstart.html          run → wait workflow
    lifecycle.html           job_id, status, tail, wait, kill
    cli.html                 CLI surface and JSON contract
    integrations.html        Claude, Codex, OpenCode, Hermes, CLI fallback
    mcp-http.html            MCP and HTTP server usage
    troubleshooting.html     common recovery paths
  assets/
    site.css
    site.js                  optional progressive enhancement only
```

Every page has a skip link, header navigation, main landmark, footer, title, canonical relative navigation, and GitHub/Release links.

## Content source and correctness

- Commands come from `README.md`, `docs/one-minute-demo.md`, `docs/agent-integrations.md`, CLI `--help`, and integration tests.
- Version-specific archive names are not copied into the general installation guide. The guide resolves the latest release dynamically or links to Releases.
- The quick start teaches that `wait` returns final bounded output in v0.2.27. `tail` remains for repeated or later log retrieval.
- Platform support states Linux x86_64 and macOS Apple Silicon only. Windows binaries are explicitly unavailable.

## Validation

`scripts/validate-site.py` uses `html.parser`, `pathlib`, and `urllib.parse` from the Python standard library. It verifies local static-site invariants without an HTTP server:

- required pages and `<title>`
- one `<main>` and named landmarks
- internal file links and fragment targets
- no duplicate IDs
- images have `alt`
- relative asset links resolve inside `site/`

Unit fixtures cover both valid and broken HTML. Browser checks cover visual and keyboard behavior that static parsing cannot prove.

## Deployment

A dedicated Pages workflow runs on `main` when `site/**`, validator code, or Pages workflow changes. It validates first, then configures Pages, uploads `site/`, and deploys with the official GitHub Pages actions.

The workflow must have least-privilege `contents: read`, `pages: write`, and `id-token: write` permissions. It does not build release binaries or run with repository secrets.

## Rollout

1. Land the site and validation workflow.
2. Enable GitHub Pages source as GitHub Actions in repository settings if it is not automatically configured.
3. Read back the deployed URL and only then add the stable site URL to README and Cargo metadata.

The Pages setting is an external configuration action. The repository can be fully prepared and validated before that action.

## Non-goals

No framework, CMS, server, accounts, search backend, analytics, custom domain, or docs.rs replacement.
