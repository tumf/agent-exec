#!/usr/bin/env python3
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit
import sys

ROOT = Path(__file__).resolve().parents[1] / "site"
REQUIRED_PAGES = {
    "index.html", "docs/index.html", "docs/install.html", "docs/quick-start.html",
    "docs/lifecycle.html", "docs/cli.html", "docs/mcp-http.html", "docs/integrations.html",
    "docs/output-contracts.html", "docs/troubleshooting.html", "docs/release-verification.html",
}

class PageParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids, self.links, self.images, self.has_title = set(), [], [], False
        self.duplicate_ids = []
    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == "title": self.has_title = True
        if attrs.get("id"):
            if attrs["id"] in self.ids: self.duplicate_ids.append(attrs["id"])
            self.ids.add(attrs["id"])
        if tag in {"a", "link", "script"} and attrs.get("href"): self.links.append(attrs["href"])
        if tag == "img": self.images.append(attrs)

def parse(path):
    parser = PageParser()
    parser.feed(path.read_text(encoding="utf-8"))
    return parser

def validate(root=ROOT):
    errors = []
    pages = sorted(root.rglob("*.html"))
    found = {page.relative_to(root).as_posix() for page in pages}
    for required in sorted(REQUIRED_PAGES - found): errors.append(f"missing required page: {required}")
    parsed = {page: parse(page) for page in pages}
    for page, data in parsed.items():
        relative = page.relative_to(root)
        if not data.has_title: errors.append(f"missing title: {relative}")
        for identifier in data.duplicate_ids: errors.append(f"duplicate id {identifier}: {relative}")
        for image in data.images:
            if "alt" not in image: errors.append(f"image missing alt: {relative}")
        for href in data.links:
            target = urlsplit(href)
            if target.scheme or target.netloc or href.startswith(("mailto:", "tel:")): continue
            destination = (page.parent / unquote(target.path)).resolve() if target.path else page.resolve()
            try: destination.relative_to(root.resolve())
            except ValueError:
                errors.append(f"link escapes site: {relative} -> {href}"); continue
            if not destination.is_file(): errors.append(f"broken link: {relative} -> {href}"); continue
            if target.fragment and target.fragment not in parsed.get(destination, parse(destination)).ids:
                errors.append(f"missing fragment: {relative} -> {href}")
    return errors

if __name__ == "__main__":
    errors = validate()
    if errors:
        print("\n".join(errors), file=sys.stderr)
        sys.exit(1)
    print("site validation passed")
