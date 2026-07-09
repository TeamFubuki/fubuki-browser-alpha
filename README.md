# Fubuki Browser Alpha

**A macOS-first browser shell powered by C++20, CEF, Rust, and SolidJS.**

Fubuki Browser Alpha は、Chromium Embedded Framework を直接扱う macOS 向けブラウザシェルです。Electron / Tauri / WKWebView には依存せず、CEF を描画・Web 実行基盤として使い、ブラウザの論理状態は Rust 製の FrostEngine に集約します。

このリポジトリは「ブラウザ UI を作る」だけではなく、タブ・ウィンドウ・セッション・設定・永続化・ホスト連携を分離し、将来的に自動操作や別ホストへ拡張できるブラウザ基盤を作ることを目的としています。

> Status: Alpha / MVP. 実験的な実装を含みます。コード署名、ノータリゼーション、アップデーター、同期、拡張機能互換などはまだ提供していません。

## Concept

Fubuki は、単なる CEF ラッパーではありません。

- **Native-first** — macOS ネイティブホストが CEF とウィンドウを管理します。
- **Engine-owned state** — タブ、ウィンドウ、設定、セッションは FrostEngine が所有します。
- **Protocol boundary** — UI とホストは Frost Protocol を通して状態を読み書きします。
- **Replaceable layers** — UI、Engine、Host を分離し、実装の差し替えや自動操作境界を作りやすくします。
- **No heavyweight app wrapper** — Electron、Tauri、WKWebView、WebView2 は使用しません。

## Architecture

```text
Fubuki Browser UI (SolidJS)
    ↓ Frost Protocol / JSON-RPC over CEF message router
FrostEngine Core (Rust)
    ↓ HostCommand / HostEvent / JSON boundary
CEF / macOS Host (C++20)
```

| Layer | Path | Role |
|---|---|---|
| Fubuki Browser UI | `ui/` | SolidJS 製のブラウザ UI。`fubuki://app/` 上で動作します。 |
| Frost Protocol | `crates/frost-protocol/` | UI、Engine、Host 間の Request / Response / Event 型定義。 |
| FrostEngine Core | `crates/frost-core/` | タブ、ウィンドウ、設定、セッションなどの論理状態を管理します。 |
| Frost Store | `crates/frost-store/` | SQLite による履歴、ブックマーク、ダウンロード、設定、セッション永続化。 |
| Frost FFI | `crates/frost-ffi/` | C++ ホストから Rust エンジンを呼び出すための FFI 境界。 |
| macOS CEF Host | `native/macos-cef-host/`, `native/src/` | NSWindow、CEF Browser、`fubuki://` scheme、HostCommand 実行を担当します。 |

詳しい設計は [docs/architecture.md](docs/architecture.md) を参照してください。

## Requirements

| Tool | Version |
|---|---|
| macOS | 12 or later |
| Xcode Command Line Tools | latest |
| CMake | 3.21 or later |
| Rust | 1.96.1 or later |
| Node.js | 20 or later |
| pnpm | 11 or later |
| LLVM via Homebrew | latest, for `clang-format` / `clang-tidy` |

Apple Silicon を主なターゲットにしています。Intel Mac でも CEF の `macosx64` ビルドを使う構成はありますが、互換性は利用する CEF ビルドに依存します。

## Quick Start

```bash
git clone https://github.com/TeamFubuki/fubuki-browser-alpha.git
cd fubuki-browser-alpha
make bootstrap
make build
make run
```

`make bootstrap` は CEF の取得、UI 依存関係のインストール、ネイティブビルド設定をまとめて実行します。

## CEF Setup

```bash
make cef
```

`make cef` は実行環境に応じて Apple Silicon では `macosarm64`、Intel では `macosx64` の CEF binary distribution を `third_party/cef/` に配置します。

手動で配置する場合は、[CEF Automated Builds](https://cef-builds.spotifycdn.com/index.html) から macOS 用 binary distribution を取得し、`third_party/cef/` に展開するか、CMake 設定時に `CEF_ROOT` を指定してください。

```bash
CEF_ROOT=/path/to/cef_binary make configure
```

CEF バイナリはリポジトリには含めません。

## Build

```bash
make build
```

個別にビルドする場合は以下を使います。

```bash
make ui       # SolidJS UI
make rust     # FrostEngine Rust crates
make native   # C++ / CEF native app
```

## Run

```bash
make run
```

初回は以下の順で実行するのが安全です。

```bash
make bootstrap
make build
make run
```

## Test

```bash
make test          # Rust + UI + native tests
make test-rust     # cargo test --workspace
make test-ui       # Vitest
make test-native   # CMake + GoogleTest / CTest
```

## Lint and Format

```bash
make lint-all       # UI + Rust + C++ linters
make format-all     # UI + Rust + C++ formatters

make lint           # Oxlint for UI
make lint-rust      # Clippy with -D warnings
make lint-native    # Clang-Tidy / cppcheck

make format         # Oxfmt for UI
make format-rust    # rustfmt
make format-native  # clang-format
```

## Security and Policy Checks

```bash
make audit       # cargo-audit
make audit-deny  # cargo-deny
```

CI also runs UI checks, Rust checks, Rust FFI build verification, dependency audits, native unit tests, Rust docs, and repository hygiene checks.

## Make Targets

| Target | Description |
|---|---|
| `make bootstrap` | Download CEF, install UI dependencies, configure native build. |
| `make cef` | Download or update CEF under `third_party/cef/`. |
| `make ui` | Build the SolidJS UI. |
| `make rust` | Build FrostEngine Rust crates. |
| `make configure` | Configure the CMake native build. |
| `make native` | Build the native C++ / CEF app. |
| `make build` | Build UI, Rust, and native components. |
| `make run` | Build and launch the app. |
| `make test` | Run all local test suites. |
| `make clean` | Remove build outputs. |
| `make distclean` | Remove build outputs, CEF binaries, and local caches. |

## Implemented Scope

The Alpha currently includes:

- macOS CEF host with native window / browser instance management
- SolidJS browser UI loaded through `fubuki://app/`
- Frost Protocol request / response / event boundary
- Rust-owned browser state for windows, tabs, settings, and sessions
- HostCommand / HostEvent bridge between FrostEngine and the native host
- SQLite persistence for settings, history, bookmarks, downloads, and session snapshots
- Normal and private window handling, with private windows using an off-the-record CEF request context
- Internal pages for browser-owned surfaces such as settings, history, bookmarks, downloads, debug, and new tab
- External command boundary with capability checks, audit events, and rate limiting
- Local test, lint, format, audit, and CI workflows

## Known Limitations

Fubuki Browser Alpha is not a production browser yet.

- CEF binaries are configured externally and are not vendored.
- Code signing, notarization, update delivery, and release packaging are out of scope for the current MVP.
- Password manager, browser sync, Chrome extension compatibility, and polished import / export are not implemented.
- Session restore restores windows and last tab URLs, not full in-page navigation stacks.
- Private windows use an off-the-record request context, but downloaded files still exist on disk if the user saves them.
- Cache and site-data clearing is conservative and depends on CEF request-context behavior.
- Bookmark folders and browser-compatible import / export still need more UX and persistence work.
- The UI and content layout still uses native child views; there is no fully native toolbar yet.

See [docs/known-limitations.md](docs/known-limitations.md) for the current list.

## Data Directory

```text
~/Library/Application Support/Fubuki Browser Alpha/
```

The app stores the CEF profile data and `fubuki.sqlite3` under this directory. The SQLite database is used for browser-owned data such as settings, history, bookmarks, downloads, and session snapshots.

## Documentation

- [Architecture](docs/architecture.md)
- [FrostEngine Plan](docs/frost-engine-plan.md)
- [Bridge API](docs/bridge-api.md)
- [Commands](docs/commands.md)
- [Events](docs/events.md)
- [Internal Pages](docs/internal-pages.md)
- [Known Limitations](docs/known-limitations.md)

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## License

[MIT License](LICENSE) — Copyright (c) 2026 TeamFubuki
