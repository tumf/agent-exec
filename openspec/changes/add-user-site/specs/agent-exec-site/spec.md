## ADDED Requirements

### Requirement: Public landing page explains agent-exec value

The project MUST provide a public landing page that explains the durable managed-job value of `agent-exec`, demonstrates the `run` and `wait` workflow, identifies supported release platforms, and links to installation, documentation, GitHub, and the latest release. Essential content and navigation MUST work without JavaScript.

#### Scenario: New user reaches the landing page

**Given**: A user opens the public site on a mobile or desktop browser
**When**: The landing page loads
**Then**: The user can identify the product purpose and primary installation action in the initial page region
**And**: The user can navigate to documentation, GitHub, and Releases
**And**: The page states that release binaries support Linux x86_64 and macOS Apple Silicon and do not support Windows

### Requirement: Public documentation covers installation and managed-job lifecycle

The public site MUST document installation, checksum verification, quick start, the `job_id` lifecycle, CLI commands, output contracts, MCP and HTTP integration, supported AI-agent clients, troubleshooting, and release verification. Commands and response examples MUST match the current released behavior.

#### Scenario: User follows the quick start

**Given**: A user installed a supported release binary
**When**: The user follows the public quick start
**Then**: The documented command starts a managed job and returns a stable `job_id`
**And**: The documented `wait` call returns terminal state, exit code, and bounded command output
**And**: The documentation explains when to use `tail` for later or repeated log retrieval

### Requirement: Static site is accessible and responsive

The public site MUST use semantic HTML landmarks, keyboard-visible focus, readable contrast, responsive layouts, reduced-motion handling, and alternative text for meaningful images. Essential reading and navigation MUST NOT depend on client-side JavaScript.

#### Scenario: Keyboard and narrow-screen navigation

**Given**: A user navigates by keyboard or uses a 390px-wide viewport
**When**: The user moves through landing and documentation pages
**Then**: Focus is visible, content does not require horizontal page scrolling, and all primary navigation and calls to action remain operable

### Requirement: Repository validates and deploys the site

The repository MUST provide an offline validation command that rejects missing required pages, broken internal links and fragments, duplicate IDs, missing page titles, and images without alternative text. GitHub Actions MUST run validation and deploy only the static site to GitHub Pages from `main` using least-privilege Pages permissions.

#### Scenario: Broken internal link blocks deployment

**Given**: A site page references a missing local page or fragment
**When**: Site validation runs in CI
**Then**: Validation fails before the Pages artifact is deployed

#### Scenario: Valid main revision is published

**Given**: Site validation passes on `main`
**When**: The Pages workflow runs
**Then**: The static `site/` artifact is deployed through official GitHub Pages actions
**And**: The public root URL returns HTTP 200 with the expected product page title
