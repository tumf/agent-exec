# Manual site check

- Date: 2026-07-20
- Local server: `python3 -m http.server 8765 --directory site`
- Browser: Chrome via `playwright-cli`

## Results

- `/`, `/docs/install.html`, and `/docs/integrations.html` at 390×844 and 1440×900: `document.documentElement.scrollWidth <= window.innerWidth` returned `true` at each width.
- First `Tab` on `/` and `/docs/integrations.html` focused the visible `Skip to content` link; `Enter` changed the URL fragment to `#main`.
- First `Tab` on `/docs/lifecycle.html`, `/docs/output-contracts.html`, `/docs/troubleshooting.html`, and `/docs/release-verification.html` focused `Skip to content`.
- Header navigation and the landing-page calls to action are native links and remained keyboard reachable after the skip-link check.

The only browser console error was the development server's expected missing `/favicon.ico` response. The site does not reference a favicon.
