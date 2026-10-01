import contextlib
import hashlib
import io
import json
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


ROOT_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT_DIR / "scripts"))

import cef_lock  # noqa: E402


CEF_VERSION = "154.0.23+g062ebe4+chromium-154.0.8037.17"
CHROMIUM_VERSION = "154.0.8037.17"


def create_archive(directory, platform_name, *, header_cef_version=None, readme_chromium=None):
    archive_name = f"cef_binary_{CEF_VERSION}_{platform_name}.tar.bz2"
    distribution = directory / archive_name.removesuffix(".tar.bz2")
    if distribution.exists():
        import shutil

        shutil.rmtree(distribution)
    (distribution / "include").mkdir(parents=True)
    (distribution / "cmake").mkdir()
    (distribution / "libcef_dll").mkdir()
    (distribution / "include" / "cef_version.h").write_text(
        "\n".join(
            (
                f'#define CEF_VERSION "{header_cef_version or CEF_VERSION}"',
                "#define CHROME_VERSION_MAJOR 154",
                "#define CHROME_VERSION_MINOR 0",
                "#define CHROME_VERSION_BUILD 8037",
                "#define CHROME_VERSION_PATCH 17",
            )
        )
        + "\n",
        encoding="utf-8",
    )
    (distribution / "README.txt").write_text(
        f"CEF Version:      {CEF_VERSION}\nChromium Version: {readme_chromium or CHROMIUM_VERSION}\n",
        encoding="utf-8",
    )
    (distribution / "cmake" / "FindCEF.cmake").write_text(
        'include("cef_variables")\n', encoding="utf-8"
    )
    (distribution / "cmake" / "cef_variables.cmake").write_text(
        'set(CEF_INCLUDE_PATH "${_CEF_ROOT}")\n'
        'set(CEF_LIBCEF_DLL_WRAPPER_PATH "${_CEF_ROOT}/libcef_dll")\n',
        encoding="utf-8",
    )
    (distribution / "libcef_dll" / "CMakeLists.txt").write_text("# fixture\n", encoding="utf-8")

    archive_path = directory / archive_name
    with tarfile.open(archive_path, "w:bz2") as archive:
        archive.add(distribution, arcname=distribution.name)
    return archive_path


def make_lock(download_base_url, entries):
    return {
        "schema_version": 1,
        "channel": "stable",
        "download_base_url": download_base_url,
        "provenance": {"index_url": "https://cef-builds.spotifycdn.com/index.json"},
        "platforms": entries,
    }


def entry_for(platform_name, archive_path):
    return {
        "platform": platform_name,
        "cef_version": CEF_VERSION,
        "chromium_version": CHROMIUM_VERSION,
        "archive": archive_path.name,
        "checksum": {
            "algorithm": "sha1",
            "value": hashlib.sha1(archive_path.read_bytes()).hexdigest(),
        },
    }


class CEFLockTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.download_dir = self.root / "downloads"
        self.download_dir.mkdir()
        self.download_base_url = "https://cef-builds.spotifycdn.com"
        self.archives = {}
        self.entries = {}
        for platform_name in cef_lock.SUPPORTED_PLATFORMS:
            archive = create_archive(self.download_dir, platform_name)
            self.archives[platform_name] = archive
            self.entries[platform_name] = entry_for(platform_name, archive)
        self.lock_path = self.root / "cef.lock"
        self.write_lock(make_lock(self.download_base_url, self.entries))

    def tearDown(self):
        self.temporary.cleanup()

    def write_lock(self, lock):
        self.lock_path.write_text(json.dumps(lock, indent=2) + "\n", encoding="utf-8")

    def test_lock_validates_both_mac_architectures(self):
        lock = cef_lock.read_lock(self.lock_path)
        self.assertEqual(set(lock["platforms"]), {"macosarm64", "macosx64"})
        for platform_name, entry in lock["platforms"].items():
            self.assertEqual(cef_lock.verify_archive(self.archives[platform_name], entry), entry["checksum"]["value"])

    def test_lock_rejects_archive_name_and_checksum_mismatches(self):
        lock = json.loads(self.lock_path.read_text(encoding="utf-8"))
        lock["platforms"]["macosarm64"]["archive"] = "cef_binary_latest_macosarm64.tar.bz2"
        with self.assertRaisesRegex(cef_lock.CEFError, "archive must be"):
            cef_lock.validate_lock(lock)

        lock = json.loads(self.lock_path.read_text(encoding="utf-8"))
        lock["platforms"]["macosx64"]["checksum"]["value"] = "not-a-checksum"
        with self.assertRaisesRegex(cef_lock.CEFError, "checksum.value"):
            cef_lock.validate_lock(lock)

    def test_version_metadata_mismatch_leaves_existing_install_untouched(self):
        platform_name = "macosarm64"
        bad_archive = create_archive(
            self.download_dir,
            platform_name,
            header_cef_version="154.0.24+g062ebe4+chromium-154.0.8037.17",
        )
        entry = entry_for(platform_name, bad_archive)
        lock = make_lock(self.download_base_url, dict(self.entries))
        lock["platforms"][platform_name] = entry
        self.write_lock(lock)

        # The lock remains well-formed, and the bytes match its checksum, but
        # the archive's extracted version metadata does not match the lock.
        install = self.root / "cef-install"
        install.mkdir()
        sentinel = install / "keep-existing-install.txt"
        sentinel.write_text("preserve", encoding="utf-8")

        with patch.object(
            cef_lock.urllib.request,
            "urlopen",
            return_value=io.BytesIO(bad_archive.read_bytes()),
        ):
            with self.assertRaisesRegex(cef_lock.CEFError, "CEF version mismatch"):
                cef_lock.fetch_cef(
                    self.lock_path,
                    install,
                    self.root / "cache",
                    platform_name,
                )
        self.assertEqual(sentinel.read_text(encoding="utf-8"), "preserve")

    def test_chromium_version_metadata_mismatch_is_rejected(self):
        platform_name = "macosarm64"
        create_archive(
            self.download_dir,
            platform_name,
            readme_chromium="154.0.8037.18",
        )
        with self.assertRaisesRegex(cef_lock.CEFError, "Chromium version mismatch"):
            cef_lock.verify_distribution(
                self.download_dir / self.archives[platform_name].name.removesuffix(".tar.bz2"),
                self.entries[platform_name],
            )

    def test_checksum_mismatch_preserves_install_and_cleans_partial_download(self):
        lock = json.loads(self.lock_path.read_text(encoding="utf-8"))
        lock["platforms"]["macosarm64"]["checksum"]["value"] = "0" * 40
        self.write_lock(lock)
        install = self.root / "cef-install"
        install.mkdir()
        sentinel = install / "keep-existing-install.txt"
        sentinel.write_text("preserve", encoding="utf-8")
        cache = self.root / "cache"

        with patch.object(
            cef_lock.urllib.request,
            "urlopen",
            return_value=io.BytesIO(self.archives["macosarm64"].read_bytes()),
        ):
            with self.assertRaisesRegex(cef_lock.CEFError, "checksum mismatch"):
                cef_lock.fetch_cef(self.lock_path, install, cache, "macosarm64")
        self.assertEqual(sentinel.read_text(encoding="utf-8"), "preserve")
        self.assertEqual(list(cache.iterdir()), [])

    def test_successful_fetch_installs_marker_and_reuses_matching_install(self):
        for platform_name in cef_lock.SUPPORTED_PLATFORMS:
            with self.subTest(platform=platform_name):
                install = self.root / f"cef-install-{platform_name}"
                install.mkdir()
                sentinel = install / "old-install.txt"
                sentinel.write_text("old", encoding="utf-8")
                cache = self.root / f"cache-{platform_name}"
                with patch.object(
                    cef_lock.urllib.request,
                    "urlopen",
                    return_value=io.BytesIO(self.archives[platform_name].read_bytes()),
                ), contextlib.redirect_stdout(io.StringIO()):
                    cef_lock.fetch_cef(self.lock_path, install, cache, platform_name)
                self.assertFalse(sentinel.exists())
                self.assertTrue(cef_lock.installation_matches(install, self.entries[platform_name]))

                with contextlib.redirect_stdout(io.StringIO()) as output:
                    cef_lock.fetch_cef(self.lock_path, install, cache, platform_name)
                self.assertIn("already matches cef.lock", output.getvalue())

    def test_broken_archive_preserves_install_and_is_removed_from_cache(self):
        platform_name = "macosarm64"
        broken_bytes = b"not a tar archive"
        archive_name = self.entries[platform_name]["archive"]
        broken_archive = self.download_dir / archive_name
        broken_archive.write_bytes(broken_bytes)

        lock = json.loads(self.lock_path.read_text(encoding="utf-8"))
        lock["platforms"][platform_name]["checksum"]["value"] = hashlib.sha1(broken_bytes).hexdigest()
        self.write_lock(lock)
        install = self.root / "cef-install"
        install.mkdir()
        sentinel = install / "keep-existing-install.txt"
        sentinel.write_text("preserve", encoding="utf-8")
        cache = self.root / "cache"

        with patch.object(
            cef_lock.urllib.request,
            "urlopen",
            return_value=io.BytesIO(broken_bytes),
        ):
            with self.assertRaisesRegex(cef_lock.CEFError, "Could not extract"):
                cef_lock.fetch_cef(self.lock_path, install, cache, platform_name)
        self.assertEqual(sentinel.read_text(encoding="utf-8"), "preserve")
        self.assertEqual(list(cache.iterdir()), [])

    def test_failed_download_removes_temporary_file(self):
        destination = self.root / "failed.tar.bz2"

        class BrokenResponse:
            def __enter__(self):
                return self

            def __exit__(self, *_args):
                return False

            def read(self, _size):
                if not hasattr(self, "sent_chunk"):
                    self.sent_chunk = True
                    return b"partial archive"
                raise OSError("simulated interrupted download")

        with patch.object(cef_lock.urllib.request, "urlopen", return_value=BrokenResponse()):
            with self.assertRaisesRegex(cef_lock.CEFError, "Could not download"):
                cef_lock._download(
                    "https://example.invalid/archive.tar.bz2",
                    destination,
                    self.entries["macosarm64"],
                )
        self.assertFalse(destination.exists())
        self.assertEqual(list(self.root.glob("failed.tar.bz2.download-*.tmp")), [])

    def test_latest_candidate_is_stable_and_uses_standard_archive(self):
        beta_version = "155.0.0+g7654321+chromium-155.0.9000.1"
        newer_version = "154.0.28+g564dd6c+chromium-154.0.8037.58"
        index = {
            "macosarm64": {
                "versions": [
                    {
                        "channel": "stable",
                        "cef_version": CEF_VERSION,
                        "chromium_version": CHROMIUM_VERSION,
                        "files": [
                            {
                                "name": f"cef_binary_{CEF_VERSION}_macosarm64.tar.bz2",
                                "sha1": "1" * 40,
                            },
                            {
                                "name": f"cef_binary_{CEF_VERSION}_macosarm64_minimal.tar.bz2",
                                "sha1": "2" * 40,
                            },
                        ],
                    },
                    {
                        "channel": "beta",
                        "cef_version": beta_version,
                        "chromium_version": "155.0.9000.1",
                        "files": [
                            {
                                "name": f"cef_binary_{beta_version}_macosarm64.tar.bz2",
                                "sha1": "3" * 40,
                            }
                        ],
                    },
                    {
                        "channel": "stable",
                        "cef_version": newer_version,
                        "chromium_version": "154.0.8037.58",
                        "files": [
                            {
                                "name": f"cef_binary_{newer_version}_macosarm64.tar.bz2",
                                "sha1": "4" * 40,
                            }
                        ],
                    },
                ]
            }
        }
        selected = cef_lock.latest_stable_entry(index, "macosarm64")
        self.assertEqual(selected["cef_version"], newer_version)
        self.assertEqual(selected["checksum"]["value"], "4" * 40)

    def test_update_lock_writes_both_latest_stable_candidates(self):
        index = {}
        for platform_name in cef_lock.SUPPORTED_PLATFORMS:
            version = "154.0.28+g564dd6c+chromium-154.0.8037.58"
            index[platform_name] = {
                "versions": [
                    {
                        "channel": "stable",
                        "cef_version": version,
                        "chromium_version": "154.0.8037.58",
                        "files": [
                            {
                                "name": f"cef_binary_{version}_{platform_name}.tar.bz2",
                                "sha1": "a" * 40,
                            }
                        ],
                    }
                ]
            }
        response = io.BytesIO(json.dumps(index).encode("utf-8"))
        with patch.object(cef_lock.urllib.request, "urlopen", return_value=response):
            with contextlib.redirect_stdout(io.StringIO()) as output:
                cef_lock.update_lock(self.lock_path, "https://cef-builds.spotifycdn.com/index.json")

        updated = cef_lock.read_lock(self.lock_path)
        for platform_name in cef_lock.SUPPORTED_PLATFORMS:
            entry = updated["platforms"][platform_name]
            self.assertEqual(entry["cef_version"], "154.0.28+g564dd6c+chromium-154.0.8037.58")
            self.assertEqual(entry["checksum"]["value"], "a" * 40)
        self.assertIn("Review the generated change", output.getvalue())


if __name__ == "__main__":
    unittest.main()
