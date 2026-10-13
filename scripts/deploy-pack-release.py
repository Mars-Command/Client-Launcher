"""Install a CI-verified signed pair only after checking every declared payload."""

import hashlib
import json
import os
import re
import shutil
import sys
from pathlib import Path, PurePosixPath
from typing import TypedDict

MANAGED_DIRS = {"mods", "config", "kubejs", "defaultconfigs", "resourcepacks", "shaderpacks"}


class FileEntry(TypedDict):
    path: str
    sha256: str
    size: int


def safe_path(root: Path, relative: str) -> Path:
    parts = PurePosixPath(relative).parts
    if (
        not parts
        or PurePosixPath(relative).is_absolute()
        or parts[0] not in MANAGED_DIRS
        or len(parts) < 2
        or any(part in {"", ".", ".."} for part in parts)
        or "/".join(parts) != relative
        or "\\" in relative
        or any(ord(character) < 32 for character in relative)
    ):
        raise ValueError("Manifest has an unsafe managed path")
    path = root
    for part in parts:
        path = path / part
        if path.is_symlink():
            raise ValueError(f"Redirected release path: {relative}")
    return path


def verify_file(path: Path, entry: FileEntry) -> None:
    if not path.is_file() or path.is_symlink():
        raise ValueError(f"Release payload missing: {entry['path']}")
    if path.stat().st_size != entry["size"]:
        raise ValueError(f"Release payload size differs: {entry['path']}")
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(64 * 1024):
            digest.update(chunk)
    if digest.hexdigest() != entry["sha256"]:
        raise ValueError(f"Release payload hash differs: {entry['path']}")


def install_release(release: Path, staging: Path, suffix: str) -> int:
    if not re.fullmatch(r"[0-9]+-[0-9]+", suffix):
        raise ValueError("Invalid deployment run suffix")
    if release.is_symlink() or staging.is_symlink():
        raise ValueError("Deployment roots must not be symbolic links")
    release = release.resolve(strict=True)
    staging = staging.resolve(strict=True)
    if staging != release / f".incoming-{suffix}":
        raise ValueError("Staging directory must be the named child of the release directory")
    manifest_path = staging / "manifest.json"
    signature_path = staging / "manifest.json.sig"
    if manifest_path.is_symlink() or signature_path.is_symlink():
        raise ValueError("Manifest pair must not be redirected")
    if manifest_path.stat().st_size > 8 * 1024 * 1024:
        raise ValueError("Manifest exceeds 8 MiB")
    signature = signature_path.read_text(encoding="ascii").strip()
    if not re.fullmatch(r"[a-fA-F0-9]{128}", signature):
        raise ValueError("Detached manifest signature is malformed")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("schemaVersion") != 1 or not isinstance(manifest.get("files"), list) or not manifest["files"]:
        raise ValueError("Unsupported manifest shape")
    paths: set[str] = set()
    replacements: list[tuple[Path, Path]] = []
    for entry in manifest["files"]:
        if (
            not isinstance(entry, dict)
            or not isinstance(entry.get("path"), str)
            or not isinstance(entry.get("sha256"), str)
            or not re.fullmatch(r"[a-f0-9]{64}", entry["sha256"])
            or type(entry.get("size")) is not int
            or entry["size"] < 0
            or entry["path"] in paths
        ):
            raise ValueError("Invalid or duplicate manifest file entry")
        paths.add(entry["path"])
        target = safe_path(release, entry["path"])
        staged = safe_path(staging, entry["path"])
        if staged.exists():
            verify_file(staged, entry)
            replacements.append((staged, target))
        else:
            verify_file(target, entry)

    pair = [(signature_path, release / signature_path.name), (manifest_path, release / manifest_path.name)]
    for source, target in [*replacements, *pair]:
        pending = target.with_name(f".{target.name}.next-{suffix}")
        if target.is_symlink() or pending.exists() or pending.is_symlink():
            raise ValueError("A release destination is redirected or already staged")

    for source, target in replacements:
        target.parent.mkdir(parents=True, exist_ok=True)
        pending = target.with_name(f".{target.name}.next-{suffix}")
        shutil.copyfile(source, pending)
        pending.chmod(0o644)
        os.replace(pending, target)

    # The server caches its inventory; restart only after this pair is installed.
    for source, target in pair:
        pending = release / f".{source.name}.next-{suffix}"
        shutil.copyfile(source, pending)
        pending.chmod(0o644)
        os.replace(pending, target)
    return len(paths)


if __name__ == "__main__":
    if len(sys.argv) != 4:
        raise SystemExit("Usage: deploy-pack-release.py <release-dir> <staging-dir> <run-id-attempt>")
    count = install_release(Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3])
    print(f"Installed signed manifest pair; verified {count} payload files.")
