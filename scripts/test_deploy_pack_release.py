import hashlib
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("deploy_pack_release", Path(__file__).with_name("deploy-pack-release.py"))
assert SPEC is not None and SPEC.loader is not None
DEPLOY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DEPLOY)


class DeploymentTests(unittest.TestCase):
    def setUp(self):
        self.folder = tempfile.TemporaryDirectory(prefix="mars-pack-deploy-test-")
        self.addCleanup(self.folder.cleanup)
        self.root = Path(self.folder.name)
        self.staging = self.root / ".incoming-123-1"
        self.staging.mkdir()
        (self.root / "mods").mkdir()
        (self.root / "manifest.json").write_text("old manifest", encoding="utf-8")
        (self.root / "manifest.json.sig").write_text("old signature", encoding="ascii")

    def manifest(self, entries):
        (self.staging / "manifest.json").write_text(
            json.dumps({"schemaVersion": 1, "packVersion": "1.2.5", "files": entries}), encoding="utf-8"
        )
        (self.staging / "manifest.json.sig").write_text("a" * 128, encoding="ascii")

    def entry(self, name, content):
        return {"path": name, "size": len(content), "sha256": hashlib.sha256(content).hexdigest()}

    def test_installs_verified_staged_mod_and_checks_existing_files(self):
        (self.root / "mods" / "existing.jar").write_bytes(b"existing")
        (self.staging / "mods").mkdir()
        (self.staging / "mods" / "new.jar").write_bytes(b"new mod")
        self.manifest([self.entry("mods/existing.jar", b"existing"), self.entry("mods/new.jar", b"new mod")])
        self.assertEqual(DEPLOY.install_release(self.root, self.staging, "123-1"), 2)
        self.assertEqual((self.root / "mods" / "new.jar").read_bytes(), b"new mod")
        self.assertEqual(json.loads((self.root / "manifest.json").read_text())["packVersion"], "1.2.5")
        self.assertEqual((self.root / "manifest.json.sig").read_text(), "a" * 128)

    def test_missing_payload_does_not_replace_the_previous_manifest(self):
        self.manifest([self.entry("mods/missing.jar", b"new mod")])
        with self.assertRaisesRegex(ValueError, "payload missing"):
            DEPLOY.install_release(self.root, self.staging, "123-1")
        self.assertEqual((self.root / "manifest.json").read_text(), "old manifest")
        self.assertEqual((self.root / "manifest.json.sig").read_text(), "old signature")

    def test_hash_mismatch_prevents_all_installation(self):
        (self.root / "mods" / "existing.jar").write_bytes(b"changed")
        (self.staging / "mods").mkdir()
        (self.staging / "mods" / "new.jar").write_bytes(b"new mod")
        self.manifest([self.entry("mods/new.jar", b"new mod"), self.entry("mods/existing.jar", b"original")])
        with self.assertRaises(ValueError):
            DEPLOY.install_release(self.root, self.staging, "123-1")
        self.assertFalse((self.root / "mods" / "new.jar").exists())
        self.assertEqual((self.root / "manifest.json").read_text(), "old manifest")

    def test_duplicate_and_traversal_paths_are_rejected(self):
        for paths in (["mods/../secret"], ["mods/new.jar", "mods/new.jar"], ["/mods/new.jar"], ["mods\\new.jar"]):
            self.manifest([self.entry(path, b"new mod") for path in paths])
            with self.assertRaises(ValueError):
                DEPLOY.install_release(self.root, self.staging, "123-1")

    def test_wrong_staging_root_and_suffix_are_rejected(self):
        self.manifest([])
        with self.assertRaises(ValueError):
            DEPLOY.install_release(self.root, self.staging, "123-2")
        with self.assertRaises(ValueError):
            DEPLOY.install_release(self.root, self.staging, "../outside")

    def test_preexisting_manifest_staging_file_prevents_payload_mutation(self):
        (self.staging / "mods").mkdir()
        (self.staging / "mods" / "new.jar").write_bytes(b"new mod")
        self.manifest([self.entry("mods/new.jar", b"new mod")])
        (self.root / ".manifest.json.next-123-1").write_text("prior staging", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "already staged"):
            DEPLOY.install_release(self.root, self.staging, "123-1")
        self.assertFalse((self.root / "mods" / "new.jar").exists())
        self.assertEqual((self.root / "manifest.json").read_text(), "old manifest")

    def test_redirected_payloads_are_rejected(self):
        target = self.root / "mods" / "external.jar"
        source = self.root / "outside.jar"
        source.write_bytes(b"source")
        try:
            target.symlink_to(source)
        except OSError:
            self.skipTest("Creating symbolic links requires Windows developer mode or elevation")
        self.manifest([self.entry("mods/external.jar", b"source")])
        with self.assertRaisesRegex(ValueError, "Redirected"):
            DEPLOY.install_release(self.root, self.staging, "123-1")


if __name__ == "__main__":
    unittest.main()
