## Implementation Tasks

- [x] Create the framework-free static site structure under `site/`, with shared semantic navigation, responsive CSS, visible focus states, reduced-motion handling, and landing-page sections for value, workflow, comparison, platforms, integrations, and calls to action. (verification: integration - `python3 scripts/validate-site.py` passed)
- [x] Add documentation pages for installation, quick start, job lifecycle, CLI, MCP/HTTP, agent integrations, output contracts, troubleshooting, and release verification, using commands consistent with v0.2.27 behavior. (verification: integration - `python3 scripts/validate-site.py` passed for required page set and internal references)
- [x] Implement a Python standard-library site validator that rejects broken internal links, missing fragment targets, duplicate IDs, missing titles, and images without alternative text. (verification: integration - `python3 -m unittest tests/test_validate_site.py` passed; includes positive and deliberately broken fixtures)
- [x] Add CI validation and a GitHub Pages deployment workflow that uploads only `site/` from `main` after validation, using official GitHub Pages actions and no external build service. (verification: integration - actionlint and `python3 -m unittest tests/test_validate_site.py` passed)


## Future Work

- Deploy the Pages workflow from `main`, fetch the configured GitHub Pages URL, and confirm HTTP 200 with the expected title before archive. The public URL returned 404 before the first deployment.
- Public deployment acceptance from Acceptance #2: merge this change to `main`, wait for the Pages workflow, then record the HTTP 200 and expected title. This requires repository-maintainer access outside this worktree.
- Browser evidence was captured in `evidence/manual-site-check.md`; repeat it after material visual changes.
- Custom domain selection and DNS changes require a separate explicit decision.
- Analytics or telemetry require a separate privacy review and are not part of this site.

## Final Validation

Expected archive gate: `cflx openspec validate add-user-site --archive-gate`

Run `prek run -a`, the site validator, actionlint, and a public Pages read-back before archive.

## Acceptance #1 Failure Follow-up
- [x] Restore terminal newlines and run `prek run -a`. (verification: integration - command `prek run -a` passed; `prek.toml` defines the repository hooks)
- [x] Strengthen the validator for exactly one main landmark, named landmarks, nonempty titles, relative asset links, and independent missing-fragment coverage. (verification: integration - `python3 -m unittest tests/test_validate_site.py` passed; broken fixtures cover the validator contracts)
- [x] Complete CLI and MCP/HTTP documentation, including endpoint, bind, authentication, CORS, and read-endpoint exposure behavior. (verification: integration - `python3 scripts/validate-site.py` passed for `site/docs/cli.html` and `site/docs/mcp-http.html`)
- [x] Capture 390px/1440px navigation and keyboard evidence; prevent page-level table overflow. (verification: manual - evidence file `openspec/changes/add-user-site/evidence/manual-site-check.md` records viewport and keyboard checks)

## Implementation Blocker #1

Category: external_non_mockable

Summary: GitHub Pages cannot be deployed from this worktree because deployment requires merging to `main` and GitHub Actions execution.

Evidence: the public site returned HTTP 404 before first deployment; `.github/workflows/pages.yml` deploys only from `main`.

Impact: The public HTTP 200 read-back required by proposal.md:48 and spec.md:47-52 cannot be completed locally.

Unblock actions: merge the Pages workflow and site changes to `main`, confirm the workflow succeeds, then fetch and record the public HTTP 200 title.

Owner: repository maintainer. Decision due: 2026-07-27.


## Acceptance #2 Failure Follow-up
- [x] HTTP integration文書が実行可能な要求形式を欠く。site/docs/mcp-http.html:22-30はエンドポイントと既定値のみで、POST /execのJSON本文や `command` が非空argv配列であることを示さない。実契約はsrc/serve.rs:239-249,272-305に定義され、README.md:637-651には完全な例がある。spec.md:17を満たすリクエスト例と型を掲載すること。（verification: integration - `python3 -m unittest tests/test_validate_site.py` passed; `site/docs/mcp-http.html` documents a runnable `POST /exec` JSON request and field contract）
- [x] Pages workflowはmain限定になっていない。.github/workflows/pages.yml:3-6のworkflow_dispatchは任意refから実行でき、deploy job .github/workflows/pages.yml:17-36にもmain制約がないため、spec.md:39の「from main」に反してfeature branchやtagを配備できる。manual dispatchを削除するか、deploy jobを `refs/heads/main` に制限し、tests/test_validate_site.py:48-54もその経路を検証すること。（verification: integration - `python3 -m unittest tests/test_validate_site.py` passed; `test_workflows_validate_and_deploy_only_site` rejects `workflow_dispatch` and `.github/workflows/pages.yml` has only `push.branches: [main]`）
- [x] ブラウザ検証の完了記録が不実。tasks.md:12 は `/`、`/docs/install.html`、`/docs/integrations.html` の390px/1440px確認とキーボード walkthrough を未完了として残す一方、tasks.md:26は完了扱いしている。manual-site-check.md:9-12は一部ページ・幅と最初のTabしか記録していない。受入側の再検証では横スクロールは解消済みだが、site/docs/{integrations,lifecycle,output-contracts,troubleshooting,release-verification}.html:1 に設計上必須のskip linkがなく、design.md:33とspec.md:31-35を満たさない。全対象ページのキーボード導線を修正・記録すること。（verification: manual - `evidence/manual-site-check.md` records 390×844 and 1440×900 checks for `/`, `/docs/install.html`, and `/docs/integrations.html`, and skip-link keyboard checks for all affected pages）
- [x] 公開ドキュメントが現行レスポンス契約と一致しない。site/docs/quick-start.html:4 と site/index.html:15-18 は完全なJSON例として `schema_version`、`ok`、`type`、`encoding`、range、total-byte fieldsを欠くが、src/schema.rs:38-56,235-254とtests/integration.rs:407-418では必須である。さらにsite/docs/output-contracts.html:1はHTTP serverをresponse envelopeではないと記載するが、src/serve.rs:183-198,228-234ではHTTP応答もenvelopeである。spec.md:15-25に一致する例と説明へ修正すること。（verification: integration - `python3 scripts/validate-site.py` verifies updated pages and internal links; examples in `site/index.html`, `site/docs/quick-start.html`, and `site/docs/output-contracts.html` include the current envelope fields）
## Final Validation

- `cflx openspec validate add-user-site --strict --evidence warn` failed before validation cleanup because verification metadata was incomplete and the blocker bullets were parsed as tasks. The metadata and blocker format were corrected; rerun is required.
- `cflx openspec validate add-user-site --archive-gate` remains blocked until the Pages deployment is merged to `main` and read back publicly.
