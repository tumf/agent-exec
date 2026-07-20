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
        self.ids, self.links, self.assets, self.images = set(), [], [], []
        self.duplicate_ids, self.main_count, self.landmarks = [], 0, set()
        self.title = None
        self.html_lang = None
        self._in_title = False

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == "html":
            self.html_lang = attrs.get("lang")
        if tag == "title":
            self._in_title = True
        if tag == "main":
            self.main_count += 1
        if tag in {"header", "main", "footer"} or attrs.get("role") in {"banner", "main", "contentinfo", "navigation"}:
            self.landmarks.add(attrs.get("role", tag))
        if attrs.get("id"):
            if attrs["id"] in self.ids:
                self.duplicate_ids.append(attrs["id"])
            self.ids.add(attrs["id"])
        if tag in {"a", "link"} and attrs.get("href"):
            self.links.append(attrs["href"])
        if tag in {"img", "script"} and attrs.get("src"):
            self.assets.append(attrs["src"])
        if tag == "img":
            self.images.append(attrs)

    def handle_endtag(self, tag):
        if tag == "title":
            self._in_title = False

    def handle_data(self, data):
        if self._in_title:
            self.title = (self.title or "") + data


def parse(path):
    parser = PageParser()
    parser.feed(path.read_text(encoding="utf-8"))
    return parser


def local_destination(page, root, reference):
    target = urlsplit(reference)
    if target.scheme or target.netloc or reference.startswith(("mailto:", "tel:")):
        return None, target
    destination = (page.parent / unquote(target.path)).resolve() if target.path else page.resolve()
    try:
        destination.relative_to(root.resolve())
    except ValueError:
        return False, target
    return destination, target


def validate(root=ROOT):
    errors = []
    pages = sorted(root.rglob("*.html"))
    found = {page.relative_to(root).as_posix() for page in pages}
    for required in sorted(REQUIRED_PAGES - found):
        errors.append(f"missing required page: {required}")
    parsed = {page.resolve(): parse(page) for page in pages}
    for page, data in parsed.items():
        relative = page.relative_to(root.resolve())
        if not data.title or not data.title.strip():
            errors.append(f"missing title: {relative}")
        if not data.html_lang or not data.html_lang.strip():
            errors.append(f"missing html language: {relative}")
        if data.main_count != 1:
            errors.append(f"expected one main landmark: {relative}")
        if not {"header", "footer"}.issubset(data.landmarks):
            errors.append(f"missing named landmarks: {relative}")
        for identifier in data.duplicate_ids:
            errors.append(f"duplicate id {identifier}: {relative}")
        for image in data.images:
            if "alt" not in image:
                errors.append(f"image missing alt: {relative}")
        for reference in data.links + data.assets:
            destination, target = local_destination(page, root, reference)
            if destination is None:
                continue
            if destination is False:
                errors.append(f"link escapes site: {relative} -> {reference}")
                continue
            if not destination.is_file():
                errors.append(f"broken link: {relative} -> {reference}")
                continue
            if target.fragment and destination.suffix == ".html" and target.fragment not in parsed.get(destination, parse(destination)).ids:
                errors.append(f"missing fragment: {relative} -> {reference}")
    return errors


if __name__ == "__main__":
    errors = validate()
    if errors:
        print("\n".join(errors), file=sys.stderr)
        sys.exit(1)
    print("site validation passed")
