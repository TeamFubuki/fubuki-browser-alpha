# Contributing to Fubuki Browser Alpha

Fubuki Browser Alpha への貢献ありがとうございます。このドキュメントでは、開発環境、ブランチ運用、コミット規約、PR の確認項目、実装方針をまとめます。

Fubuki Browser Alpha は、macOS ネイティブホスト、CEF、Rust 製 FrostEngine、SolidJS UI を分離して構成するブラウザ基盤です。このプロジェクトで重要なのは、機能を足すことだけではありません。どのレイヤーが状態を持つのか、どの境界を通して通信するのか、どの操作が安全に実行できるのかを崩さないことです。

`Alpha` はプロダクト名・コードネームであり、リリース段階としての「アルファ版」を意味しません。Issue、PR、ドキュメントで成熟度を説明する場合は、必要に応じて `MVP`、`experimental`、`not production-ready` など、実態に近い表現を使ってください。

## Development Environment

### Requirements

| Tool | Version / Notes |
|---|---|
| macOS | 12 or later |
| Xcode Command Line Tools | latest |
| CMake | 3.21 or later |
| Rust | stable toolchain with `clippy` and `rustfmt` |
| Node.js | 22 or later |
| pnpm | 11.x; `ui/package.json` pins `pnpm@11.9.0` |
| LLVM via Homebrew | for `clang-format` / `clang-tidy` |
| cppcheck | for native lint checks |
| python3 / curl / tar | required by the CEF fetch script |

The primary target is Apple Silicon macOS. Intel macOS may work with the matching CEF binary distribution, but compatibility depends on the selected CEF build.

### First-time Setup

```bash
git clone https://github.com/TeamFubuki/fubuki-browser-alpha.git
cd fubuki-browser-alpha
make bootstrap
```

`make bootstrap` downloads CEF, installs UI dependencies, and configures the native CMake build.

### Build and Run

```bash
make build
make run
```

For layer-specific work:

```bash
make ui       # SolidJS UI
make rust     # FrostEngine Rust crates
make native   # C++ / CEF host
```

If the native build fails after CEF or CMake changes, try a clean configure first:

```bash
make clean
make configure
make native
```

## Project Boundaries

Keep the architecture intact.

```text
Fubuki Browser UI (SolidJS)
    ↓ Frost Protocol
FrostEngine Core (Rust)
    ↓ HostCommand / HostEvent
CEF / macOS Host (C++20)
```

### Ownership Rules

- **FrostEngine owns logical browser state.** Tabs, windows, settings, sessions, and browser-owned persistence should live in the Rust layer.
- **The native host owns I/O and rendering.** C++ should manage NSWindow, CEF browser instances, scheme handling, and host-side side effects.
- **The UI is a client.** SolidJS should render state and send protocol requests; it should not become the source of truth for browser state.
- **Protocol changes must be explicit.** If a feature crosses layers, update the relevant request / response / event / host boundary types first.
- **External automation must remain auditable.** Capability checks, rate limiting, and audit events should not be bypassed.
- **Destructive operations must not be GET navigation.** Internal pages should use safe action boundaries for changes that mutate state.

## Branch Strategy

Use short topic branches from `main`.

| Branch | Purpose |
|---|---|
| `feature/*` | New user-facing or internal functionality |
| `fix/*` | Bug fixes |
| `docs/*` | Documentation-only changes |
| `refactor/*` | Internal restructuring without behavior changes |
| `deps/*` | Dependency updates |
| `ci/*` | CI or repository automation changes |

Pull requests should target `main` unless a maintainer says otherwise.

## Commit Style

Use [Conventional Commits](https://www.conventionalcommits.org/ja/v1.0.0/).

```text
<type>(<scope>): <description>
```

### Types

| Type | Meaning |
|---|---|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation-only change |
| `style` | Formatting-only change |
| `refactor` | Code change without feature or bug-fix intent |
| `perf` | Performance improvement |
| `test` | Test addition or update |
| `build` | Build system or dependency change |
| `ci` | CI / automation change |
| `chore` | Maintenance |
| `revert` | Revert a previous commit |

### Recommended Scopes

| Scope | Area |
|---|---|
| `ui` | SolidJS UI under `ui/` |
| `rust` | Rust workspace under `crates/` |
| `native` | C++ / CEF / macOS host |
| `protocol` | Frost Protocol request / response / event types |
| `store` | SQLite persistence |
| `docs` | Documentation |
| `deps` | Dependencies |
| `ci` | GitHub Actions and repository checks |
| `release` | Release preparation |

Examples:

```text
feat(protocol): add tab duplicate request
fix(native): reject destructive settings GET navigation
docs(architecture): clarify host state ownership
deps(ui): update Tailwind CSS
```

## Pull Request Standard

A pull request should be small enough to review accurately. Split unrelated work.

Before opening a PR:

1. Rebase or merge the latest `main`.
2. Run the checks relevant to your change.
3. Update docs when behavior, commands, architecture, or limitations change.
4. Include screenshots or recordings for visible UI changes.
5. Mention known limitations instead of implying they are solved.

A good PR description includes:

- what changed
- why it changed
- affected layers
- test results
- linked issue, if any
- remaining risks or follow-up work

### Suggested PR Checklist

```md
## Summary
- 

## Verification
- [ ] make test
- [ ] make lint-all
- [ ] make format-all
- [ ] make audit

## Notes
- 
```

Use the checklist as a guide. Do not mark commands as passed unless you actually ran them.

## Local Checks

Run the broad checks before asking for review.

```bash
make test
make lint-all
make format-all
```

For dependency and policy checks:

```bash
make audit
make audit-deny
```

Layer-specific commands are available when a full pass is unnecessary:

```bash
make test-rust
make test-ui
make test-native

make lint
make lint-rust
make lint-native

make format
make format-rust
make format-native
```

## CI Coverage

Pull requests to `main` run CI checks for:

| Job | Coverage |
|---|---|
| UI check | Oxlint, Oxfmt, TypeScript build, Vitest |
| Rust check | rustfmt, Clippy, `cargo test --workspace` |
| Rust FFI build | `frost-ffi` release static library build |
| Rust security audit | `cargo-audit` |
| Rust license & advisory deny | `cargo-deny` |
| Rust docs build | `cargo doc` with warnings denied |
| UI dependency audit | `pnpm audit --audit-level=high` |
| Native unit test | CMake, GoogleTest, CTest |
| Repository hygiene | merge conflict marker and large-file checks |

The heavier macOS native CEF build is intended for `main` or manual workflow runs, not every pull request. Do not assume a PR has validated the full app bundle path.

## Coding Guidelines

### Rust / FrostEngine

- Keep state changes in FrostEngine services, not in UI or host glue.
- Prefer typed request / response / event models over ad-hoc JSON.
- Run `cargo fmt --all` and Clippy with warnings denied.
- Public APIs should have useful doc comments.
- Use explicit error types; `thiserror` is available in the workspace.
- Add tests for service behavior and protocol-level changes.

### Frost Protocol

- Treat protocol messages as a boundary, not an implementation detail.
- Keep request, response, and event payloads versionable and serializable.
- Avoid leaking UI-only or CEF-only assumptions into protocol types.
- Update documentation when adding or renaming protocol messages.

### C++ / macOS / CEF Host

- Use C++20.
- Keep the host thin: render, execute host commands, forward CEF callbacks, report results.
- Do not reintroduce a second logical state store in C++.
- Follow CEF lifetime and threading rules carefully.
- Format with `clang-format` and keep Clang-Tidy / cppcheck output clean where practical.
- Treat `fubuki://` scheme handlers as a security boundary, not only as routing code.
- Do not vendor CEF binaries into the repository.

### SolidJS UI

- Treat the UI as a protocol client.
- Use Solid's reactive primitives intentionally; avoid hiding state mutations in unrelated components.
- Avoid `any` unless there is a clear boundary reason.
- Keep browser operations routed through the bridge layer.
- Use Tailwind CSS for styling.
- Use Vitest for state, bridge, and component behavior that can be tested without a native host.

### Persistence

- Browser-owned data should go through the Frost Store layer.
- Migrations must be deterministic and safe to rerun.
- Avoid storing private-window state in normal app persistence.
- Be conservative with destructive data-clearing behavior and document limitations.

### External Automation

External automation should connect through declared capabilities and auditable command paths. Do not add ad-hoc automation shortcuts that bypass capability checks, rate limiting, or audit events.

## File Map

| Path | Purpose |
|---|---|
| `crates/frost-protocol/` | Request, response, event, and host boundary schemas |
| `crates/frost-core/` | Browser state and operations core |
| `crates/frost-store/` | SQLite persistence layer |
| `crates/frost-engine-api/` | Engine adapter boundary |
| `crates/frost-ffi/` | Rust-to-C FFI layer |
| `native/macos-cef-host/` | macOS CEF host project files |
| `native/src/` | Native host implementation |
| `native/tests/` | Native unit tests |
| `ui/src/` | SolidJS browser UI |
| `ui/src/bridge/` | Frost Protocol client bridge |
| `docs/` | Architecture, protocol, command, event, and limitation docs |
| `scripts/` | Bootstrap, build, CEF, and cleanup scripts |

## Testing Policy

Add or update tests when changing behavior.

- New engine behavior should have Rust tests.
- Protocol changes should cover serialization and expected request / response behavior.
- UI changes should have Vitest coverage where practical.
- Native host logic should use native unit tests when it can be tested without launching the full app.
- Bug fixes should include a regression test unless the behavior is not reasonably testable in the current structure.

## Documentation Policy

Update documentation when a change affects:

- setup or build commands
- architecture or layer ownership
- Frost Protocol, HostCommand, or HostEvent contracts
- internal pages or browser-owned surfaces
- known limitations
- security-relevant behavior

Do not make README claims broader than the implementation. If the implementation is partial, say that it is partial.

## Issues

When filing a bug, include:

- reproduction steps
- expected behavior
- actual behavior
- macOS version and hardware
- branch or commit SHA
- logs, screenshots, or recordings when useful

For feature requests, include:

- motivation
- proposed behavior
- affected layers
- compatibility or security concerns
- what can remain out of scope

## Security Notes

This project contains browser-adjacent code. Treat boundaries as security-relevant even in the current MVP scope.

- Do not expose privileged operations through ordinary page navigation.
- Keep `fubuki://app/` and internal pages separated from arbitrary web content.
- Avoid adding broad host capabilities without capability checks and auditability.
- Document security limitations instead of presenting them as solved.

## Documentation

- [Project Overview](README.md)
- [Architecture](docs/architecture.md)
- [FrostEngine Plan](docs/frost-engine-plan.md)
- [Bridge API](docs/bridge-api.md)
- [Commands](docs/commands.md)
- [Events](docs/events.md)
- [Internal Pages](docs/internal-pages.md)
- [Known Limitations](docs/known-limitations.md)

## License

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE).
