#!/usr/bin/env python3
"""Sign the assembled CEF bundle inside out (ad-hoc by default)."""
import argparse
import plistlib
import subprocess
import tempfile
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("app", type=Path)
    parser.add_argument("--identity", default="-", help="Developer ID Application identity or - for local testing")
    args = parser.parse_args()
    app = args.app.resolve()
    framework = app / "Contents/Frameworks/Chromium Embedded Framework.framework"
    helpers = sorted((app / "Contents/Frameworks").glob("* Helper*.app"))
    if not (framework / "Libraries/libcef_sandbox.dylib").is_file() or len(helpers) != 5:
        parser.error("expected the pinned CEF framework and all five Helper bundles")
    if args.identity != "-" and not args.identity.startswith("Developer ID Application:"):
        parser.error("distribution signing requires a Developer ID Application identity")
    with tempfile.TemporaryDirectory() as tmp:
        def sign(path, renderer=False):
            command = ["codesign", "--force", "--sign", args.identity, "--options", "runtime"]
            command += ["--timestamp"] if args.identity != "-" else ["--timestamp=none"]
            values = {"com.apple.security.cs.allow-jit": True} if renderer else {}
            # Ad-hoc signatures have no Team ID for library validation. This
            # local-only exception is unnecessary with a single Developer ID.
            if args.identity == "-" and path.suffix == ".app":
                values["com.apple.security.cs.disable-library-validation"] = True
            entitlements = Path(tmp) / "entitlements.plist"
            entitlements.write_bytes(plistlib.dumps(values))
            command += ["--entitlements", str(entitlements)]
            subprocess.run(command + [str(path)], check=True)

        for library in sorted((framework / "Versions/A/Libraries").glob("*.dylib")):
            sign(library)
        sign(framework)
        for helper in helpers:
            sign(helper, "(Renderer)" in helper.name)
        sign(app)
    subprocess.run(["codesign", "--verify", "--deep", "--strict", "--verbose=2", str(app)], check=True)


if __name__ == "__main__":
    main()
