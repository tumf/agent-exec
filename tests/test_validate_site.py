import importlib.util
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "validate-site.py"
spec = importlib.util.spec_from_file_location("validate_site", SCRIPT)
validate_site = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validate_site)


class ValidateSiteTests(unittest.TestCase):
    def site(self, files):
        directory = tempfile.TemporaryDirectory()
        root = Path(directory.name)
        for name, content in files.items():
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
        return directory, root

    def test_validates_repository_site(self):
        self.assertEqual(validate_site.validate(), [])

    def test_rejects_invalid_html_contracts(self):
        temporary, root = self.site({
            "index.html": "<html><head><title> </title></head><body><header></header><main><a href='target.html#missing'>x</a><img src='missing.png'><script src='missing.js'></script><p id='same'></p><p id='same'></p></main><footer></footer></body></html>",
            "target.html": "<html><head><title>Target</title></head><body><header></header><main id='present'></main><footer></footer></body></html>",
        })
        try:
            errors = validate_site.validate(root)
        finally:
            temporary.cleanup()
        self.assertTrue(any("missing title" in error for error in errors))
        self.assertTrue(any("missing fragment" in error for error in errors))
        self.assertTrue(any("broken link" in error for error in errors))
        self.assertTrue(any("image missing alt" in error for error in errors))
        self.assertTrue(any("duplicate id" in error for error in errors))

    def test_rejects_missing_landmarks(self):
        temporary, root = self.site({"index.html": "<html><head><title>Page</title></head><body><main></main></body></html>"})
        try:
            errors = validate_site.validate(root)
        finally:
            temporary.cleanup()
        self.assertTrue(any("missing named landmarks" in error for error in errors))

    def test_workflows_validate_and_deploy_only_site(self):
        workflow = (Path(__file__).resolve().parents[1] / ".github/workflows/pages.yml").read_text(encoding="utf-8")
        self.assertIn("python3 scripts/validate-site.py", workflow)
        self.assertIn("actions/upload-pages-artifact", workflow)
        self.assertIn("path: site", workflow)
        self.assertIn("actions/deploy-pages", workflow)
        self.assertIn("branches: [main]", workflow)
        self.assertNotIn("workflow_dispatch:", workflow)


if __name__ == "__main__":
    unittest.main()
