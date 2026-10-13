"""Publish exact signed pack bytes over HTTPS; no SSH, archives or remote commands."""

import argparse
import base64
import hashlib
import json
import os
import re
import sys
import time
from pathlib import Path
from typing import TypedDict
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener

PREFIX = "/api/v1/releases/deployments"
MAX_MANIFEST = 8 * 1024 * 1024
MAX_RESPONSE = 16 * 1024 * 1024
MAX_CHUNK = 8 * 1024 * 1024


class PublishFailure(RuntimeError):
    """A sanitized operator-facing transport or activation failure."""


class FilePlan(TypedDict):
    index: int
    path: str
    sha256: str
    size: int
    uploadedBytes: int
    complete: bool
    reusable: bool


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def origin(value: str, allow_local_http: bool) -> str:
    parsed = urlsplit(value)
    try:
        parsed.port
    except ValueError:
        raise ValueError("Publishing API origin has an invalid port")
    if (
        not parsed.hostname or parsed.username or parsed.password or parsed.path not in {"", "/"}
        or parsed.query or parsed.fragment or any(ord(character) < 33 for character in value)
        or not (parsed.scheme == "https" or (
            allow_local_http and parsed.scheme == "http" and parsed.hostname in {"localhost", "127.0.0.1", "::1"}
        ))
    ):
        raise ValueError("Publishing API must be an HTTPS origin without credentials, path, query or fragment")
    return value.rstrip("/")


def string(value: dict[str, object], name: str) -> str:
    result = value.get(name)
    if not isinstance(result, str):
        raise ValueError(f"Publishing API returned an invalid {name}")
    return result


def integer(value: dict[str, object], name: str) -> int:
    result = value.get(name)
    if type(result) is not int or result < 0:
        raise ValueError(f"Publishing API returned an invalid {name}")
    return result


def object_response(raw: bytes) -> dict[str, object]:
    result = json.loads(raw)
    if not isinstance(result, dict):
        raise ValueError("Publishing API did not return a JSON object")
    return result


def plans(result: dict[str, object]) -> list[FilePlan]:
    entries = result.get("files")
    if not isinstance(entries, list) or len(entries) > 20000:
        raise ValueError("Publishing API returned an invalid file inventory")
    output = []
    for value in entries:
        if not isinstance(value, dict):
            raise ValueError("Publishing API returned an invalid file entry")
        complete, reusable = value.get("complete"), value.get("reusable")
        if type(complete) is not bool or type(reusable) is not bool:
            raise ValueError("Publishing API returned invalid file flags")
        output.append(FilePlan(
            index=integer(value, "index"), path=string(value, "path"), sha256=string(value, "sha256"),
            size=integer(value, "size"), uploadedBytes=integer(value, "uploadedBytes"),
            complete=complete, reusable=reusable,
        ))
    return output


class ApiClient:
    def __init__(self, api_origin: str, token: str, allow_local_http: bool = False):
        self.origin = origin(api_origin, allow_local_http)
        if not re.fullmatch(r"[A-Za-z0-9_-]{32,128}", token):
            raise ValueError("Set a dedicated MARS_RELEASE_DEPLOY_TOKEN with 32..128 URL-safe characters")
        self.headers = {"Authorization": "Bearer " + token, "Accept": "application/json"}
        cf_id = os.environ.get("CF_ACCESS_CLIENT_ID", "")
        cf_secret = os.environ.get("CF_ACCESS_CLIENT_SECRET", "")
        if bool(cf_id) != bool(cf_secret):
            raise ValueError("Both optional Cloudflare Access service credentials must be configured")
        if cf_id:
            if any(ord(character) < 33 or ord(character) > 126 for character in cf_id + cf_secret):
                raise ValueError("Cloudflare Access credentials contain invalid header characters")
            self.headers.update({"CF-Access-Client-Id": cf_id, "CF-Access-Client-Secret": cf_secret})
        self.opener = build_opener(NoRedirect())

    def request(
        self, path: str, method: str = "GET", body: bytes | None = None,
        media_type: str = "application/json", private: bool = True,
    ) -> bytes:
        headers = self.headers if private else {"Cache-Control": "no-cache"}
        headers = {**headers, **({"Content-Type": media_type} if body is not None else {})}
        for attempt in range(4):
            try:
                request = Request(self.origin + path, data=body, headers=headers, method=method)
                with self.opener.open(request, timeout=120) as response:
                    raw = response.read(MAX_RESPONSE + 1)
                    if len(raw) > MAX_RESPONSE:
                        raise ValueError("Publishing API response exceeds its size limit")
                    return raw
            except HTTPError as error:
                status = error.code
                if status in {429, 502, 503, 504} and attempt < 3:
                    retry = error.headers.get("Retry-After", "5")
                    delay = min(60, max(1, int(retry))) if retry.isascii() and re.fullmatch(r"[0-9]{1,6}", retry) else 5
                    error.close()
                    time.sleep(delay)
                    continue
                raw = error.read(8192)
                error.close()
                code = "request_rejected"
                try:
                    detail = object_response(raw).get("detail")
                    if isinstance(detail, dict) and isinstance(detail.get("code"), str):
                        candidate = detail["code"]
                        if re.fullmatch(r"[a-z0-9_]{1,64}", candidate):
                            code = candidate
                except (ValueError, UnicodeError):
                    code = "non_json_response"
                raise PublishFailure(f"Publishing API rejected the request: HTTP {status}, {code}") from None
            except (URLError, TimeoutError, ConnectionError):
                if attempt == 3:
                    raise PublishFailure("Publishing API transport failed; rerun to resume verified staging") from None
                time.sleep(5)
        raise PublishFailure("Publishing request did not complete")


def payload_file(roots: list[Path], relative: str) -> Path:
    parts = relative.split("/")
    if not parts or any(part in {"", ".", ".."} for part in parts) or "\\" in relative or ":" in relative:
        raise ValueError("Publishing API returned an unsafe payload path")
    for root in roots:
        candidate = root
        for part in parts:
            candidate /= part
            if candidate.is_symlink() or candidate.is_junction():
                raise ValueError("A local payload path is redirected")
        if candidate.is_file():
            if root.resolve() not in candidate.resolve().parents:
                raise ValueError("Local payload escapes its configured root")
            return candidate
    raise FileNotFoundError(f"Required payload is not on the server or in any upload root: {relative}")


def publish(api: ApiClient, manifest_path: Path, signature_path: Path, roots: list[Path]) -> str:
    if manifest_path.stat().st_size > MAX_MANIFEST:
        raise ValueError("Manifest exceeds 8 MiB")
    raw = manifest_path.read_bytes()
    signature = signature_path.read_text(encoding="ascii").strip()
    if not re.fullmatch(r"[a-fA-F0-9]{128}", signature):
        raise ValueError("Manifest signature must be the detached Ed25519 hex signature")
    manifest = object_response(raw)
    manifest_hash = hashlib.sha256(raw).hexdigest()
    expected_files = manifest.get("files")
    if not isinstance(expected_files, list) or not expected_files or len(expected_files) > 20000:
        raise ValueError("Manifest does not contain a supported hashed file inventory")
    reserve = json.dumps({"manifestBase64": base64.b64encode(raw).decode("ascii"), "signature": signature}).encode()
    reply = object_response(api.request(PREFIX, "POST", reserve))
    identifier = string(reply, "deploymentId")
    if identifier != manifest_hash or string(reply, "manifestSha256") != manifest_hash:
        raise ValueError("Publishing API reserved a different signed manifest")
    if string(reply, "packVersion") != string(manifest, "packVersion"):
        raise ValueError("Publishing API returned a different pack version")
    if reply.get("status") == "published" and reply.get("active") is not True:
        raise PublishFailure("The requested release has already been superseded")
    file_plans = plans(reply)
    if len(file_plans) != len(expected_files):
        raise ValueError("Publishing API changed the signed file inventory")
    chunk_bytes = min(MAX_CHUNK, integer(reply, "chunkBytes"))
    if chunk_bytes == 0:
        raise ValueError("Publishing API returned a zero chunk limit")
    for index, entry in enumerate(file_plans):
        expected = expected_files[index]
        if not isinstance(expected, dict) or (
            entry["index"] != index or entry["path"] != expected.get("path")
            or entry["sha256"] != expected.get("sha256") or entry["size"] != expected.get("size")
            or entry["uploadedBytes"] > entry["size"]
        ):
            raise ValueError("Publishing API file plan differs from the signed manifest")
        if entry["complete"] or entry["reusable"]:
            continue
        path = payload_file(roots, entry["path"])
        digest = hashlib.sha256()
        size = 0
        with path.open("rb") as source:
            for chunk in iter(lambda: source.read(64 * 1024), b""):
                digest.update(chunk)
                size += len(chunk)
            if size != entry["size"] or digest.hexdigest() != entry["sha256"]:
                raise ValueError(f"Local payload differs from the signed manifest: {entry['path']}")
            offset = entry["uploadedBytes"]
            source.seek(offset)
            complete = False
            while not complete:
                chunk = source.read(min(chunk_bytes, size - offset))
                if not chunk and offset < size:
                    raise ValueError("Local payload changed during upload")
                response = object_response(api.request(
                    f"{PREFIX}/{identifier}/files/{index}?offset={offset}", "PUT", chunk,
                    "application/octet-stream",
                ))
                received = integer(response, "uploadedBytes")
                complete = response.get("complete") is True
                if received != offset + len(chunk) or (received == offset and received < size) or (complete != (received == size)):
                    raise ValueError("Publishing API did not acknowledge the uploaded byte range")
                offset = received
        print(f"Verified upload: {entry['path']} ({size} bytes)")

    result = object_response(api.request(f"{PREFIX}/{identifier}/commit", "POST", b"{}"))
    if (
        result.get("status") != "published" or result.get("deploymentId") != identifier
        or result.get("manifestSha256") != manifest_hash
        or result.get("fileCount") != len(expected_files)
    ):
        raise ValueError("Publishing API did not confirm the complete signed release")

    for attempt in range(5):
        suffix = "?deployment=" + quote(identifier, safe="")
        live_manifest = api.request("/api/v1/manifest.json" + suffix, private=False)
        live_signature = api.request("/api/v1/manifest.json.sig" + suffix, private=False)
        if live_manifest == raw and live_signature.decode("ascii").strip() == signature.lower():
            print(f"Pack {manifest['packVersion']} published; live signed bytes match deployment {identifier}.")
            return identifier
        if attempt < 4:
            time.sleep(2)
    raise PublishFailure("Publication completed but public endpoints do not match; check proxy/cache configuration")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--api-origin", required=True)
    parser.add_argument("--manifest", type=Path, default=Path("manifest-dist/manifest.json"))
    parser.add_argument("--signature", type=Path, default=Path("manifest-dist/manifest.json.sig"))
    parser.add_argument("--payload-root", type=Path, action="append", default=[])
    parser.add_argument("--allow-local-http", action="store_true", help="Loopback integration tests only")
    args = parser.parse_args()
    try:
        api = ApiClient(args.api_origin, os.environ.get("MARS_RELEASE_DEPLOY_TOKEN", ""), args.allow_local_http)
        publish(api, args.manifest, args.signature, args.payload_root)
    except PublishFailure as error:
        print(str(error), file=sys.stderr)
        return 1
    except (OSError, ValueError):
        # Do not print transport exceptions, headers, credentials, or arbitrary server responses.
        print("Signed pack publication failed. Verify deployment configuration, declared payloads and API status; "
              "the server retains its last active release until commit succeeds.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
