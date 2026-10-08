# macOS CEF process sandbox

Fubuki requires Chromium's process sandbox in **all** configurations, including
Debug. There is no development opt-out. `USE_SANDBOX=OFF` fails configuration;
Main and Helper also fail compilation without `CEF_USE_SANDBOX`. External
Chromium command-line arguments are ignored by the browser, so `--no-sandbox`,
`--disable-gpu-sandbox`, feature overrides and replacement Helper paths cannot
weaken the policy. Main rejects `--type` before loading CEF; only Helpers execute
child processes. This also means external Chromium debugging switches are not
available; add reviewed development switches in `FubukiCefApp` instead.

## CEF requirements

`cef.lock` pins CEF **154.0.23+g062ebe4 / Chromium 154.0.8037.17** for arm64 and
x86_64. Since M138, macOS uses `libcef_sandbox.dylib`, rather than a statically
linked sandbox library. Each Helper initializes `CefScopedSandboxContext` before
`CefScopedLibraryLoader::LoadInHelper()` and keeps it alive through process exit.
Initialization failure terminates the Helper without falling back to an
unsandboxed process. Main retains dynamic loading and passes `nullptr` for the
Windows-only sandbox argument to CEF.

The bundle contains all five CEF Helper variants (base, Alerts, GPU, Plugin,
Renderer) under `Contents/Frameworks/`. The framework uses `Versions/A`, with
root executable, Libraries and Resources links and `Versions/Current -> A`.
The sandbox loader resolves `../../../Chromium Embedded Framework.framework/
Libraries/libcef_sandbox.dylib` relative to a Helper executable. CMake validates
the library exists in both Debug and Release distributions. Rebuilds replace
framework links rather than following directory symlinks; stale links from the
old CEF copy macro are removed to allow strict code-signature verification.

Sources: [CEF sandbox requirements](https://chromiumembedded.github.io/cef/sandbox_setup.html),
plus the pinned distribution's `include/cef_sandbox_mac.h`,
`libcef_dll/wrapper/cef_scoped_sandbox_context_mac.mm`,
`tests/cefsimple/process_helper_mac.cc`, and `cmake/cef_variables.cmake`.

## Local builds and signing

```sh
make cef
make ui
cmake -S native -B native/build -DCMAKE_BUILD_TYPE=Release -DUSE_SANDBOX=ON
cmake --build native/build --parallel 6
python3 scripts/sign_macos.py "native/build/Fubuki Browser Alpha.app"
python3 scripts/verify_macos_sandbox.py --launch "native/build/Fubuki Browser Alpha.app" --attempt-disable
```

An old build cache containing `USE_SANDBOX=OFF` must explicitly be migrated with
`-DUSE_SANDBOX=ON`. Re-sign after every native rebuild. The script signs dylibs,
then the framework, then Helpers, then Main, and verifies the entire bundle with
`codesign --verify --deep --strict`. It never uses `--deep` to sign.

The default identity `-` is **local ad-hoc signing**, with hardened runtime.
Renderer gets `com.apple.security.cs.allow-jit` for V8. Local ad-hoc Main and
Helpers additionally get `com.apple.security.cs.disable-library-validation`
because ad-hoc code has no Team ID. This local exception does not disable
Chromium's Seatbelt sandbox and is omitted for Developer ID signing.
Unsigned development binaries can be produced by CMake but are not the verified
launch path; use the ad-hoc signing step above, with the sandbox still enabled.

For distribution, sign all nested binaries with the same Developer ID identity:

```sh
python3 scripts/sign_macos.py "native/build/Fubuki Browser Alpha.app" \
  --identity "Developer ID Application: YOUR NAME (YOUR TEAM ID)"
```

Developer ID signing uses a secure timestamp and hardened runtime, with only
Renderer's JIT entitlement. No Apple App Sandbox or inheritance entitlement is
added: that is a separate system from Chromium's process sandbox. Notarization,
Gatekeeper assessment and delivery remain distribution prerequisites and are
not automated here. A Developer ID signed build must undergo the live checks
and regression checklist below before general distribution.

## Live verification

The verification script calls macOS `sandbox_check(pid, NULL, 0)` via libsandbox,
following [Chromium's Seatbelt check](https://github.com/chromium/chromium/blob/main/sandbox/mac/seatbelt.cc).
It walks only descendants of the browser PID, requires live Renderer and GPU
processes, rejects sandbox-disabling child switches and API errors, and checks
sandboxed Utility services too. Utilities explicitly marked
`--service-sandbox-type=none` are reported as Chromium exemptions; they are not
counted as sandboxed. This checks OS policy attachment, not just CEF settings or
code-signing entitlements, and does not prove every allowed operation in the
Chromium profile is appropriate.

To inspect an app while manually exercising it:

```sh
python3 scripts/verify_macos_sandbox.py --pid BROWSER_PID
```

`--launch` uses the app's normal profile; close existing Fubuki instances first.
It terminates only its newly created process group afterwards. CI runs on a clean
runner, verifies all four build configurations reject `USE_SANDBOX=OFF`, signs
Release ad-hoc and runs the live check with disabling switches supplied. The
macOS build job now runs for PRs as well as main.

Manual regression checklist (repeat for Developer ID and supported OS/CPU):

- Normal and Private Window: render internal pages and navigate to HTTPS.
- Create/switch/close tabs, reload and use back/forward; check new Renderer PIDs.
- Download a small file in both window modes and confirm its contents.
- Request a Web permission and exercise the broker without unintended grants.
- Recreate a Renderer and re-run the Seatbelt check; crash recovery remains a
  separate known limitation.

## Recorded validation — 2026-10-08

Local environment: Apple Silicon arm64, macOS 27.0.1, pinned CEF 154, Release
with ad-hoc hardened signing. Native build and strict recursive signature
verification passed. GPU, Renderers and startup Utility processes returned
`seatbelt=1`; supplying `--no-sandbox --disable-gpu-sandbox` to Main still
produced sandboxed children. Debug, Release, RelWithDebInfo and MinSizeRel all
rejected `USE_SANDBOX=OFF`. The Python verification/lock suite passed 16 tests and native unit tests passed 131 tests.
Normal HTTPS navigation, internal new-tab rendering and creation of a Private
Window/new tab were checked on the running app; newly created Renderers also
returned `seatbelt=1`. A local HTTP attachment response was received, but no
saved download was confirmed, and a notification button did not produce a
verified permission result. These attempts are not counted as passing regression
checks and require investigation on a distribution-signed build.

**Outstanding:** Developer ID credentials were not used, x86_64 and older macOS
were not run, and the full download/permission/Renderer-recreation checklist is
not yet certified. Do not consider issue #133's distribution acceptance complete.
The local pinned arm64 CEF framework declares Mach-O `minos 13.0`, despite the
CEF CMake configuration specifying 12.0. macOS 12 compatibility is therefore a
separate unresolved blocker; setting a deployment target cannot fix a prebuilt
framework's minimum OS. Ad-hoc launch also logs Chromium signature-category and
keychain errors; it cannot substitute for Developer ID distribution validation.
