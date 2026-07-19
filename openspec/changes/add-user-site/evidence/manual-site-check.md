# Manual site check

- Date: 2026-07-20
- Local server: `python3 -m http.server 8765 --directory site`
- Browser: Chrome via `playwright-cli`

## Results

- `/` at 390×844: no horizontal page scrolling.
- First `Tab` on `/`: focused the visible `.skip-link`.
- `/docs/cli.html` at 1440×900: no horizontal page scrolling.
- `/docs/mcp-http.html`: title was `MCP and HTTP — agent-exec`.

The only browser console error was the development server's expected missing `/favicon.ico` response. The site does not reference a favicon.
