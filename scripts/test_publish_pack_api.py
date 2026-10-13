import base64
import hashlib
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("publish_pack_api", Path(__file__).with_name("publish-pack-api.py"))
assert SPEC is not None and SPEC.loader is not None
API = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(API)


class FakeApi:
    def __init__(self, raw, entry):
        self.raw = raw
        self.identifier = hashlib.sha256(raw).hexdigest()
        self.entry = entry
        self.received = b""
        self.committed = False
        self.reusable = False
        self.change_inventory = False
        self.public_changed = False

    def request(self, path, method="GET", body=None, media_type="application/json", private=True):
        if path == API.PREFIX:
            request = json.loads(body)
            assert base64.b64decode(request["manifestBase64"]) == self.raw
            entry = {**self.entry, "index": 0, "uploadedBytes": len(self.received),
                     "complete": False, "reusable": self.reusable}
            if self.change_inventory:
                entry["sha256"] = "b" * 64
            return json.dumps({
                "deploymentId": self.identifier, "manifestSha256": self.identifier, "packVersion": "1.2.5",
                "chunkBytes": 4, "status": "staging", "active": False, "files": [entry],
            }).encode()
        if "/files/" in path:
            offset = int(path.split("offset=")[1])
            assert offset == len(self.received)
            self.received += body
            return json.dumps({"uploadedBytes": len(self.received),
                               "complete": len(self.received) == self.entry["size"]}).encode()
        if path.endswith("/commit"):
            self.committed = True
            return json.dumps({
                "deploymentId": self.identifier, "manifestSha256": self.identifier,
                "status": "published", "fileCount": 1,
            }).encode()
        if path.startswith("/api/v1/manifest.json.sig"):
            return ("a" * 128).encode()
        if path.startswith("/api/v1/manifest.json"):
            return self.raw + (b" " if self.public_changed else b"")
        raise AssertionError(path)


class PublishingClientTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="mars-https-publish-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "mods").mkdir()
        self.payload = b"new mod fixture"
        self.entry = {"path": "mods/new.jar", "size": len(self.payload),
                      "sha256": hashlib.sha256(self.payload).hexdigest()}
        self.raw = json.dumps({"packVersion": "1.2.5", "files": [self.entry]}).encode()
        (self.root / "manifest.json").write_bytes(self.raw)
        (self.root / "manifest.json.sig").write_text("a" * 128, encoding="ascii")
        (self.root / "mods" / "new.jar").write_bytes(self.payload)
        self.client = FakeApi(self.raw, self.entry)

    def publish(self):
        return API.publish(self.client, self.root / "manifest.json", self.root / "manifest.json.sig", [self.root])

    def test_chunked_upload_commit_and_public_verification(self):
        self.assertEqual(self.publish(), self.client.identifier)
        self.assertEqual(self.client.received, self.payload)
        self.assertTrue(self.client.committed)

    def test_upload_resumes_from_acknowledged_offset(self):
        self.client.received = self.payload[:5]
        self.publish()
        self.assertEqual(self.client.received, self.payload)

    def test_matching_server_files_need_no_upload(self):
        self.client.reusable = True
        (self.root / "mods" / "new.jar").unlink()
        self.publish()
        self.assertEqual(self.client.received, b"")
        self.assertTrue(self.client.committed)

    def test_server_cannot_change_the_signed_upload_inventory(self):
        self.client.change_inventory = True
        with self.assertRaisesRegex(ValueError, "signed manifest"):
            self.publish()
        self.assertFalse(self.client.committed)

    def test_missing_and_tampered_local_payload_fail_before_commit(self):
        (self.root / "mods" / "new.jar").write_bytes(b"tampered")
        with self.assertRaisesRegex(ValueError, "differs"):
            self.publish()
        (self.root / "mods" / "new.jar").unlink()
        with self.assertRaises(FileNotFoundError):
            self.publish()
        self.assertFalse(self.client.committed)

    def test_public_pair_must_match_exact_signed_bytes(self):
        original_sleep = API.time.sleep
        API.time.sleep = lambda seconds: None
        self.addCleanup(setattr, API.time, "sleep", original_sleep)
        self.client.public_changed = True
        with self.assertRaisesRegex(RuntimeError, "public endpoints"):
            self.publish()

    def test_unsafe_origins_and_invalid_credentials_fail_without_echoing_secrets(self):
        for url in ("http://api.example", "https://user:secret@api.example", "https://api.example?token=secret",
                    "https://api.example/path", "file:///secret"):
            with self.assertRaises(ValueError) as error:
                API.ApiClient(url, "fixture-" + "x" * 40)
            self.assertNotIn("user:secret", str(error.exception))
        with self.assertRaises(ValueError):
            API.ApiClient("https://api.example", "short")
        self.assertEqual(API.origin("http://localhost:8000", True), "http://localhost:8000")

    def test_payload_lookup_rejects_paths_outside_the_declared_roots(self):
        for relative in ("../secret", "/secret", "mods/../secret", "mods\\secret"):
            with self.assertRaises(ValueError):
                API.payload_file([self.root], relative)


if __name__ == "__main__":
    unittest.main()
