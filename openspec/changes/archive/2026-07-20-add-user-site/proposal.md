---
change_type: implementation
priority: high
dependencies: []
references:
  - README.md
  - docs/one-minute-demo.md
  - docs/agent-integrations.md
  - .github/workflows/ci.yml
---

# Add a user-facing landing page and documentation site

**Change Type**: implementation

## Problem / Context

The repository currently explains `agent-exec` through a long README and two Markdown guides. GitHub Pages is not configured, there is no user-facing product landing page, and new users must discover installation, the managed-job lifecycle, and agent-specific integration details inside the repository.

The site should improve discovery and onboarding without adding a JavaScript framework or a second package ecosystem to this Rust repository.

## Proposed Solution

Add a lightweight static site under `site/` using semantic HTML and CSS with only minimal native JavaScript where progressive enhancement requires it. Publish the generated static files through GitHub Pages.

The landing page will explain the core value, show the one-call `wait` result workflow, compare `agent-exec` with synchronous subprocesses and `nohup`, list supported platforms and integrations, and provide direct calls to action for installation, documentation, GitHub, and the latest release.

The documentation section will cover installation, quick start, job lifecycle, CLI commands, AI-agent integrations, MCP/HTTP usage, output contracts, troubleshooting, and release verification. Existing README and `docs/*.md` content remain source material, but the public site must not depend on GitHub's Markdown renderer at runtime.

GitHub Actions will validate internal links and deploy the static site. Deployment must use only repository contents and GitHub Pages; no external CMS, analytics service, or runtime backend is introduced.

## Acceptance Criteria

- The root page communicates the product's value and provides a working installation path within one viewport and one primary CTA.
- A user can navigate from the landing page to installation, quick start, lifecycle, CLI/API/MCP, integrations, troubleshooting, GitHub, and the latest release.
- Installation content accurately states Linux x86_64 and macOS Apple Silicon support and that Windows release binaries are not provided.
- The quick start demonstrates `run` followed by `wait`, with completion state, exit code, and bounded output returned by `wait`.
- The site is usable at mobile and desktop widths, with keyboard-visible focus, semantic landmarks, sufficient contrast, reduced-motion support, and no essential JavaScript dependency.
- A local validation command checks HTML structure and internal links; CI runs that validation.
- A Pages workflow builds and deploys only the static site from `main` after validation.
- README and Cargo package metadata link to the public site after its final Pages URL is known.

## Explicit Completion Conditions

- `site/index.html`, documentation pages, shared CSS, and any minimal assets exist and render without a package install or network fetch.
- Repository-native validation fails on missing required pages, broken internal links, duplicate IDs, missing page titles, or missing image alternative text.
- Browser checks confirm the landing page and documentation navigation at representative mobile and desktop widths.
- GitHub Actions validation succeeds, the Pages deployment succeeds, and the public URL is fetched back with HTTP 200 and expected page title/content.
- `prek run -a` and `cflx openspec validate add-user-site --strict --evidence warn` pass.

## Out of Scope

- User accounts, search backend, comments, analytics, cookies, or telemetry.
- A CMS, React/Vue/Svelte framework, Node package manager, or server-side application.
- Translations beyond English in the first release.
- Replacing docs.rs API documentation.
- Custom domain setup; GitHub Pages is the initial host.
- Automatically publishing externally before the Pages workflow and content are reviewed.
