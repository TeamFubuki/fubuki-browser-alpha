#!/usr/bin/env python3
"""Validate, fetch, and explicitly update the repository's CEF lock."""

from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import os
import platform as host_platform
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.parse
import urllib.request
import uuid
from pathlib import Path
from typing import Any


ROOT_DIR = Path(__file__).resolve().parent.parent
DEFAULT_LOCK_FILE = ROOT_DIR / "cef.lock"
DEFAULT_INDEX_URL = "https://cef-builds.spotifycdn.com/index.json"
SUPPORTED_PLATFORMS = ("macosarm64", "macosx64")
CHECKSUM_LENGTHS = {"sha1": 40, "sha256": 64}
LOCK_MARKER = ".fubuki-cef-lock.json"
CHROMIUM_VERSION_PATTERN = re.compile(r"^\d+\.\d+\.\d+\.\d+$")
CEF_VERSION_PATTERN = re.compile(
    r"^\d+\.\d+\.\d+\+g[0-9a-fA-F]{7,40}\+chromium-\d+\.\d+\.\d+\.\d+$"
)


class CEFError(RuntimeError):
    """A user-actionable CEF lock or distribution error."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise CEFError(message)


def validate_lock(lock: Any) -> dict[str, Any]:
    _require(isinstance(lock, dict), "cef.lock must contain a JSON object")
    _require(lock.get("schema_version") == 1, "cef.lock has an unsupported schema_version")
    _require(lock.get("channel") == "stable", "cef.lock channel must be 'stable'")

    base_url = lock.get("download_base_url")
    parsed_base_url = urllib.parse.urlparse(base_url) if isinstance(base_url, str) else None
    _require(
        parsed_base_url is not None
        and parsed_base_url.scheme == "https"
        and bool(parsed_base_url.netloc),
        "cef.lock download_base_url must be an absolute HTTP(S) URL",
    )

    provenance = lock.get("provenance")
    _require(isinstance(provenance, dict), "cef.lock provenance must be an object")
    index_url = provenance.get("index_url")
    parsed_index_url = urllib.parse.urlparse(index_url) if isinstance(index_url, str) else None
    _require(
        parsed_index_url is not None
        and parsed_index_url.scheme == "https"
        and bool(parsed_index_url.netloc),
        "cef.lock provenance.index_url must be an absolute HTTP(S) URL",
    )

    platforms = lock.get("platforms")
    _require(isinstance(platforms, dict), "cef.lock platforms must be an object")
    _require(
        set(platforms) == set(SUPPORTED_PLATFORMS),
        "cef.lock must contain exactly macosarm64 and macosx64 entries",
    )

    for platform_name in SUPPORTED_PLATFORMS:
        entry = platforms[platform_name]
        prefix = f"cef.lock platforms.{platform_name}"
        _require(isinstance(entry, dict), f"{prefix} must be an object")
        _require(entry.get("platform") == platform_name, f"{prefix}.platform does not match its key")

        cef_version = entry.get("cef_version")
        _require(
            isinstance(cef_version, str) and CEF_VERSION_PATTERN.fullmatch(cef_version) is not None,
            f"{prefix}.cef_version is missing or malformed",
        )
        chromium_version = entry.get("chromium_version")
        _require(
            isinstance(chromium_version, str)
            and CHROMIUM_VERSION_PATTERN.fullmatch(chromium_version) is not None,
            f"{prefix}.chromium_version is missing or malformed",
        )
        _require(
            cef_version.endswith(f"+chromium-{chromium_version}"),
            f"{prefix}.cef_version and chromium_version disagree",
        )

        expected_archive = f"cef_binary_{cef_version}_{platform_name}.tar.bz2"
        _require(
            entry.get("archive") == expected_archive,
            f"{prefix}.archive must be {expected_archive}",
        )

        checksum = entry.get("checksum")
        _require(isinstance(checksum, dict), f"{prefix}.checksum must be an object")
        algorithm = checksum.get("algorithm")
        value = checksum.get("value")
        _require(
            isinstance(algorithm, str) and algorithm in CHECKSUM_LENGTHS,
            f"{prefix}.checksum.algorithm must be sha1 or sha256",
        )
        _require(
            isinstance(value, str)
            and len(value) == CHECKSUM_LENGTHS[algorithm]
            and re.fullmatch(r"[0-9a-f]+", value) is not None,
            f"{prefix}.checksum.value must be a lowercase {algorithm} digest",
        )

    return lock


def read_lock(lock_file: Path) -> dict[str, Any]:
    try:
        with lock_file.open(encoding="utf-8") as stream:
            return validate_lock(json.load(stream))
    except FileNotFoundError as error:
        raise CEFError(f"CEF lock file is missing: {lock_file}") from error
    except json.JSONDecodeError as error:
        raise CEFError(f"CEF lock file is not valid JSON: {lock_file}: {error}") from error


def checksum_file(path: Path, algorithm: str) -> str:
    digest = hashlib.new(algorithm)
    try:
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as error:
        raise CEFError(f"Cannot read CEF archive {path}: {error}") from error
    return digest.hexdigest()


def verify_archive(path: Path, entry: dict[str, Any]) -> str:
    algorithm = entry["checksum"]["algorithm"]
    expected = entry["checksum"]["value"]
    actual = checksum_file(path, algorithm)
    if actual != expected:
        raise CEFError(
            f"CEF archive checksum mismatch for {path.name}: expected {algorithm} {expected}, got {actual}"
        )
    return actual


def _read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise CEFError(f"Cannot read CEF distribution metadata {path}: {error}") from error


def _header_string(header: str, name: str, source: Path) -> str:
    match = re.search(rf'^\s*#define\s+{re.escape(name)}\s+"([^"]+)"\s*$', header, re.MULTILINE)
    if match is None:
        raise CEFError(f"{source} is missing #define {name}")
    return match.group(1)


def _header_integer(header: str, name: str, source: Path) -> str:
    match = re.search(rf"^\s*#define\s+{re.escape(name)}\s+(\d+)\s*$", header, re.MULTILINE)
    if match is None:
        raise CEFError(f"{source} is missing #define {name}")
    return match.group(1)


def verify_distribution(root: Path, entry: dict[str, Any], *, check_archive_root: bool = False) -> None:
    if not root.is_dir():
        raise CEFError(f"CEF distribution directory does not exist: {root}")
    expected_root = entry["archive"][: -len(".tar.bz2")]
    if check_archive_root and root.name != expected_root:
        raise CEFError(
            f"CEF archive root mismatch: expected {expected_root}, got {root.name}"
        )

    header_path = root / "include" / "cef_version.h"
    readme_path = root / "README.txt"
    cmake_variables_path = root / "cmake" / "cef_variables.cmake"
    cmake_find_path = root / "cmake" / "FindCEF.cmake"
    for required in (header_path, readme_path, cmake_variables_path, cmake_find_path):
        if not required.is_file():
            raise CEFError(f"CEF distribution is incomplete; missing {required.relative_to(root)}")

    header = _read_text(header_path)
    cef_version = _header_string(header, "CEF_VERSION", header_path)
    if cef_version != entry["cef_version"]:
        raise CEFError(
            f"CEF version mismatch in {header_path}: expected {entry['cef_version']}, got {cef_version}"
        )

    chromium_version = ".".join(
        _header_integer(header, name, header_path)
        for name in (
            "CHROME_VERSION_MAJOR",
            "CHROME_VERSION_MINOR",
            "CHROME_VERSION_BUILD",
            "CHROME_VERSION_PATCH",
        )
    )
    if chromium_version != entry["chromium_version"]:
        raise CEFError(
            f"Chromium version mismatch in {header_path}: expected {entry['chromium_version']}, got {chromium_version}"
        )

    readme = _read_text(readme_path)
    readme_cef = re.search(r"^CEF Version:\s*(\S+)\s*$", readme, re.MULTILINE)
    readme_chromium = re.search(r"^Chromium Version:\s*(\S+)\s*$", readme, re.MULTILINE)
    if readme_cef is None or readme_cef.group(1) != entry["cef_version"]:
        actual = readme_cef.group(1) if readme_cef else "missing"
        raise CEFError(
            f"CEF version mismatch in {readme_path}: expected {entry['cef_version']}, got {actual}"
        )
    if readme_chromium is None or readme_chromium.group(1) != entry["chromium_version"]:
        actual = readme_chromium.group(1) if readme_chromium else "missing"
        raise CEFError(
            f"Chromium version mismatch in {readme_path}: expected {entry['chromium_version']}, got {actual}"
        )

    # CEF's CMake package has no version constants, so version checks use the
    # header and README above while these files are checked for completeness.
    if not (root / "libcef_dll" / "CMakeLists.txt").is_file():
        raise CEFError("CEF CMake metadata references a missing libcef_dll/CMakeLists.txt")


def _marker_for(entry: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "platform": entry["platform"],
        "archive": entry["archive"],
        "cef_version": entry["cef_version"],
        "chromium_version": entry["chromium_version"],
        "checksum": entry["checksum"],
    }


def installation_matches(root: Path, entry: dict[str, Any]) -> bool:
    try:
        verify_distribution(root, entry)
        with (root / LOCK_MARKER).open(encoding="utf-8") as stream:
            marker = json.load(stream)
        return marker == _marker_for(entry)
    except (CEFError, OSError, json.JSONDecodeError):
        return False


def _download(url: str, destination: Path, entry: dict[str, Any]) -> None:
    temporary = destination.with_name(f"{destination.name}.download-{uuid.uuid4().hex}.tmp")
    digest = hashlib.new(entry["checksum"]["algorithm"])
    try:
        request = urllib.request.Request(url, headers={"User-Agent": "FubukiBrowserAlpha/CEF-lock"})
        with urllib.request.urlopen(request, timeout=60) as response, temporary.open("wb") as output:
            while True:
                chunk = response.read(1024 * 1024)
                if not chunk:
                    break
                output.write(chunk)
                digest.update(chunk)
            output.flush()
            os.fsync(output.fileno())
        actual = digest.hexdigest()
        expected = entry["checksum"]["value"]
        if actual != expected:
            raise CEFError(
                f"CEF archive checksum mismatch for {destination.name}: "
                f"expected {entry['checksum']['algorithm']} {expected}, got {actual}"
            )
        os.replace(temporary, destination)
    except CEFError:
        raise
    except OSError as error:
        raise CEFError(f"Could not download CEF archive from {url}: {error}") from error
    finally:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass


def _install_atomically(staged: Path, destination: Path) -> None:
    backup: Path | None = None
    if os.path.lexists(destination):
        backup = destination.with_name(f".{destination.name}.backup-{uuid.uuid4().hex}")
        os.replace(destination, backup)
    try:
        os.replace(staged, destination)
    except Exception:
        if backup is not None and not os.path.lexists(destination):
            os.replace(backup, destination)
        raise
    if backup is not None:
        try:
            if backup.is_symlink() or not backup.is_dir():
                backup.unlink()
            else:
                shutil.rmtree(backup)
        except OSError as error:
            print(f"Warning: installed locked CEF but could not remove backup {backup}: {error}", file=sys.stderr)


def fetch_cef(
    lock_file: Path,
    cef_root: Path,
    cache_dir: Path,
    platform_name: str,
    *,
    force: bool = False,
) -> None:
    lock = read_lock(lock_file)
    if platform_name not in lock["platforms"]:
        raise CEFError(
            f"Unsupported CEF platform {platform_name!r}; choose one of {', '.join(SUPPORTED_PLATFORMS)}"
        )
    entry = lock["platforms"][platform_name]
    if not force and installation_matches(cef_root, entry):
        print(
            f"CEF already matches cef.lock at {cef_root}: "
            f"{entry['cef_version']} / Chromium {entry['chromium_version']} ({platform_name})"
        )
        return

    cache_dir.mkdir(parents=True, exist_ok=True)
    cef_root.parent.mkdir(parents=True, exist_ok=True)
    archive = cache_dir / entry["archive"]
    if archive.is_file():
        try:
            verify_archive(archive, entry)
            print(f"Using verified cached archive {archive}")
        except CEFError as error:
            print(f"{error}; discarding cached archive", file=sys.stderr)
            archive.unlink(missing_ok=True)

    if not archive.is_file():
        encoded_name = urllib.parse.quote(entry["archive"], safe="._-")
        download_url = f"{lock['download_base_url'].rstrip('/')}/{encoded_name}"
        print(f"Downloading locked CEF {entry['cef_version']} / Chromium {entry['chromium_version']}")
        print(f"Archive: {entry['archive']}")
        _download(download_url, archive, entry)
        print(f"Checksum verified: {entry['checksum']['algorithm']} {entry['checksum']['value']}")

    with tempfile.TemporaryDirectory(prefix=".cef-extract-", dir=cef_root.parent) as temporary_name:
        extraction_root = Path(temporary_name)
        try:
            subprocess.run(
                ["tar", "-xjf", str(archive), "-C", str(extraction_root)],
                check=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
        except FileNotFoundError as error:
            raise CEFError("tar is required to extract the locked CEF archive") from error
        except subprocess.CalledProcessError as error:
            archive.unlink(missing_ok=True)
            detail = error.stderr.strip() or error.stdout.strip() or str(error)
            raise CEFError(f"Could not extract {entry['archive']}: {detail}") from error

        extracted = extraction_root / entry["archive"][: -len(".tar.bz2")]
        verify_distribution(extracted, entry, check_archive_root=True)
        (extracted / LOCK_MARKER).write_text(
            json.dumps(_marker_for(entry), indent=2) + "\n", encoding="utf-8"
        )
        _install_atomically(extracted, cef_root)

    print(
        f"Installed locked CEF {entry['cef_version']} / Chromium {entry['chromium_version']} "
        f"({platform_name}) to {cef_root}"
    )


def _standard_archive(platform_name: str, cef_version: str) -> str:
    return f"cef_binary_{cef_version}_{platform_name}.tar.bz2"


def latest_stable_entry(index: Any, platform_name: str) -> dict[str, Any]:
    if not isinstance(index, dict) or platform_name not in index:
        raise CEFError(f"CEF index does not include platform {platform_name}")

    candidates: list[tuple[tuple[tuple[int, ...], tuple[int, ...]], dict[str, Any]]] = []
    for version in index[platform_name].get("versions", []):
        if version.get("channel") != "stable":
            continue
        cef_version = version.get("cef_version", "")
        chromium_version = version.get("chromium_version", "")
        if not CEF_VERSION_PATTERN.fullmatch(cef_version):
            continue
        if not CHROMIUM_VERSION_PATTERN.fullmatch(chromium_version):
            continue
        archive = _standard_archive(platform_name, cef_version)
        asset = next(
            (item for item in version.get("files", []) if item.get("name") == archive),
            None,
        )
        if asset is None:
            continue
        checksum = asset.get("sha1", "")
        if not isinstance(checksum, str) or re.fullmatch(r"[0-9a-f]{40}", checksum) is None:
            continue
        entry = {
            "platform": platform_name,
            "cef_version": cef_version,
            "chromium_version": chromium_version,
            "archive": archive,
            "checksum": {"algorithm": "sha1", "value": checksum},
        }
        candidates.append(
            (
                (
                    tuple(map(int, chromium_version.split("."))),
                    tuple(map(int, cef_version.split("+", 1)[0].split("."))),
                ),
                entry,
            )
        )

    if not candidates:
        raise CEFError(f"CEF index has no checksummed standard stable archive for {platform_name}")
    return max(candidates, key=lambda item: item[0])[1]


def update_lock(lock_file: Path, index_url: str) -> None:
    current = read_lock(lock_file)
    parsed_index_url = urllib.parse.urlparse(index_url)
    _require(
        parsed_index_url.scheme == "https" and bool(parsed_index_url.netloc),
        "CEF update index URL must be an absolute HTTPS URL",
    )
    try:
        request = urllib.request.Request(index_url, headers={"User-Agent": "FubukiBrowserAlpha/CEF-lock"})
        with urllib.request.urlopen(request, timeout=60) as response:
            index = json.load(response)
    except (OSError, json.JSONDecodeError) as error:
        raise CEFError(f"Could not read CEF index {index_url}: {error}") from error

    updated = {**current, "platforms": dict(current["platforms"])}
    for platform_name in SUPPORTED_PLATFORMS:
        updated["platforms"][platform_name] = latest_stable_entry(index, platform_name)
    validate_lock(updated)

    old_text = lock_file.read_text(encoding="utf-8")
    new_text = json.dumps(updated, indent=2, ensure_ascii=False) + "\n"
    print("Latest stable CEF candidates from the official index:")
    for platform_name in SUPPORTED_PLATFORMS:
        before = current["platforms"][platform_name]
        after = updated["platforms"][platform_name]
        print(f"  {platform_name}:")
        print(
            f"    CEF:      {before['cef_version']} -> {after['cef_version']}"
        )
        print(
            f"    Chromium: {before['chromium_version']} -> {after['chromium_version']}"
        )
        print(f"    Archive:  {after['archive']}")
        print(
            f"    Checksum: {after['checksum']['algorithm']} {before['checksum']['value']} -> "
            f"{after['checksum']['value']}"
        )

    if new_text == old_text:
        print("cef.lock is already current; no changes written.")
        return

    lock_file.parent.mkdir(parents=True, exist_ok=True)
    temporary_path: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w", encoding="utf-8", dir=lock_file.parent, prefix=f".{lock_file.name}.", delete=False
        ) as stream:
            temporary_path = Path(stream.name)
            stream.write(new_text)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary_path, lock_file)
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)

    print("Updated cef.lock. Review the generated change and submit it in a pull request:")
    diff = difflib.unified_diff(
        old_text.splitlines(), new_text.splitlines(), fromfile="cef.lock (before)", tofile="cef.lock (after)", lineterm=""
    )
    for line in diff:
        print(line)


def _default_platform() -> str:
    machine = host_platform.machine().lower()
    if machine in ("arm64", "aarch64"):
        return "macosarm64"
    if machine in ("x86_64", "amd64"):
        return "macosx64"
    raise CEFError(f"Unsupported host architecture: {host_platform.machine()}")


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Manage the pinned macOS CEF distributions.")
    subparsers = parser.add_subparsers(dest="command", required=True)

    validate_parser = subparsers.add_parser("validate", help="validate cef.lock without network access")
    validate_parser.add_argument("--lock-file", type=Path, default=DEFAULT_LOCK_FILE)

    fetch_parser = subparsers.add_parser("fetch", help="install the archive pinned for this platform")
    fetch_parser.add_argument("--lock-file", type=Path, default=DEFAULT_LOCK_FILE)
    fetch_parser.add_argument("--cef-root", type=Path, default=ROOT_DIR / "third_party" / "cef")
    fetch_parser.add_argument("--cache-dir", type=Path, default=ROOT_DIR / ".cache" / "cef")
    fetch_parser.add_argument("--platform", choices=SUPPORTED_PLATFORMS)
    fetch_parser.add_argument("--force", action="store_true")

    update_parser = subparsers.add_parser("update", help="update cef.lock to the latest stable CEF")
    update_parser.add_argument("--lock-file", type=Path, default=DEFAULT_LOCK_FILE)
    update_parser.add_argument("--index-url", default=DEFAULT_INDEX_URL)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        if args.command == "validate":
            lock = read_lock(args.lock_file)
            for platform_name in SUPPORTED_PLATFORMS:
                entry = lock["platforms"][platform_name]
                print(
                    f"{platform_name}: CEF {entry['cef_version']} / Chromium {entry['chromium_version']} "
                    f"({entry['checksum']['algorithm']})"
                )
            return 0
        if args.command == "fetch":
            fetch_cef(
                args.lock_file,
                args.cef_root,
                args.cache_dir,
                args.platform or _default_platform(),
                force=args.force,
            )
            return 0
        if args.command == "update":
            update_lock(args.lock_file, args.index_url)
            return 0
        raise CEFError(f"Unsupported command: {args.command}")
    except CEFError as error:
        print(f"CEF lock error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
