# Fubuki Browser Alpha

**Native browser core for macOS. Built with C++20, CEF, Rust, and SolidJS.**

Fubuki Browser Alpha は、Chromium Embedded Framework を直接扱う macOS 向けブラウザ基盤です。Electron / Tauri / WKWebView には依存せず、CEF を描画・Web 実行基盤として使い、タブ、ウィンドウ、設定、セッションなどのブラウザ状態は Rust 製の FrostEngine に集約します。

`Alpha` は、このリポジトリにおけるプロダクト名・コードネームです。リリース段階としての「アルファ版」を意味しません。現在の実装は MVP スコープで開発中のため、プロダクション用途のブラウザとしてはまだ扱いません。

## Concept / コンセプト

**Not just a CEF wrapper.**

Fubuki は、UI、エンジン、ネイティブホストを明確に分離したブラウザ基盤です。ブラウザの論理状態を FrostEngine に集約することで、UI の差し替え、ホスト境界の安定化、永続化、テスト、自動操作の拡張を前提にした構成を目指します。

- **Native-first** — macOS ネイティブホストが NSWindow と CEF Browser を管理します。
- **Engine-owned state** — タブ、ウィンドウ、設定、セッションは FrostEngine が所有します。
- **Protocol boundary** — UI と Host は Frost Protocol / HostCommand / HostEvent を通して接続します。
- **Replaceable layers** — UI、Engine、Host を分離し、将来的な置き換えや別ホスト実装を妨げません。
- **Automation-ready** — 外部操作は capability check と audit event を前提に、エンジンのコマンド層へ接続します。
- **No heavy wrapper** — Electron、Tauri、WKWebView、WebView2 は使用しません。

## Architecture / アーキテクチャ

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

## Requirements / 必要環境

| Tool | Version / Notes |
|---|---|
| macOS | 12 以降 |
| Xcode Command Line Tools | 最新版 |
| CMake | 3.21 以降 |
| Rust | stable toolchain。`clippy` と `rustfmt` を含めます。 |
| Node.js | 22 以降 |
| pnpm | 11.x。`ui/package.json` では `pnpm@11.9.0` を指定しています。 |
| LLVM via Homebrew | `clang-format` / `clang-tidy` 用 |
| cppcheck | ネイティブコードのリント用 |
| python3 / curl / tar | CEF 取得スクリプトで使用 |

Apple Silicon を主なターゲットにしています。Intel Mac でも CEF の `macosx64` ビルドを使う構成はありますが、互換性は利用する CEF ビルドに依存します。

## Quick Start / クイックスタート

```bash
git clone https://github.com/TeamFubuki/fubuki-browser-alpha.git
cd fubuki-browser-alpha
make bootstrap
make build
make run
```

`make bootstrap` は CEF の取得、UI 依存関係のインストール、ネイティブビルド設定をまとめて実行します。

## CEF Setup / CEF のセットアップ

```bash
make cef
```

`make cef` は実行環境に応じて Apple Silicon では `macosarm64`、Intel では `macosx64` の CEF binary distribution を `third_party/cef/` に配置します。対象は CEF Automated Builds の stable channel から選ばれ、取得したアーカイブは公開されている SHA-1 がある場合に検証されます。

手動で配置する場合は、[CEF Automated Builds](https://cef-builds.spotifycdn.com/index.html) から macOS 用 binary distribution を取得し、`third_party/cef/` に展開するか、CMake 設定時に `CEF_ROOT` を指定してください。

```bash
CEF_ROOT=/path/to/cef_binary make configure
```

CEF バイナリはリポジトリには含めません。

## Build / ビルド

```bash
make build
```

個別にビルドする場合は以下を使います。

```bash
make ui       # SolidJS UI をビルド
make rust     # FrostEngine の Rust crate をビルド
make native   # C++ / CEF ネイティブアプリをビルド
```

## Run / 実行

```bash
make run
```

初回は以下の順で実行するのが安全です。

```bash
make bootstrap
make build
make run
```

## Test / テスト

```bash
make test          # Rust / UI / native のテストを実行
make test-rust     # cargo test --workspace
make test-ui       # Vitest
make test-native   # CMake + GoogleTest / CTest
```

## Quality Gates / リント・フォーマット・監査

```bash
make lint-all       # UI / Rust / C++ のリントを実行
make format-all     # UI / Rust / C++ のフォーマットを実行

make lint           # UI に Oxlint を実行
make lint-rust      # Clippy を -D warnings 付きで実行
make lint-native    # Clang-Tidy / cppcheck を実行

make format         # UI に Oxfmt を実行
make format-rust    # rustfmt を実行
make format-native  # clang-format を実行

make audit          # cargo-audit を実行
make audit-deny     # cargo-deny を実行
```

CI では、UI チェック、Rust チェック、Rust FFI ビルド検証、Rust 依存関係監査、UI 依存関係監査、ネイティブ単体テスト、Rust ドキュメント生成、リポジトリ衛生チェックを実行します。重い macOS CEF ビルドは `main` への反映時または手動実行時に行います。

## Make Targets / Make ターゲット

| Target | Description |
|---|---|
| `make bootstrap` | CEF を取得し、UI 依存関係をインストールし、ネイティブビルドを設定します。 |
| `make cef` | `third_party/cef/` に CEF を取得または更新します。 |
| `make ui` | SolidJS UI をビルドします。 |
| `make rust` | FrostEngine の Rust crate をビルドします。 |
| `make configure` | CMake によるネイティブビルドを設定します。 |
| `make native` | C++ / CEF ネイティブアプリをビルドします。 |
| `make build` | UI、Rust、ネイティブコンポーネントをまとめてビルドします。 |
| `make run` | アプリをビルドして起動します。 |
| `make test` | ローカルの全テストを実行します。 |
| `make clean` | ビルド成果物を削除します。 |
| `make distclean` | ビルド成果物、CEF バイナリ、ローカルキャッシュを削除します。 |

## Current Scope / 現在の実装範囲

Current implementation includes:

- ネイティブウィンドウと CEF Browser インスタンスを管理する macOS CEF ホスト
- `fubuki://app/` から読み込まれる SolidJS 製ブラウザ UI
- Frost Protocol による Request / Response / Event 境界
- ウィンドウ、タブ、設定、セッションを管理する Rust 側のブラウザ状態
- FrostEngine とネイティブホストをつなぐ HostCommand / HostEvent ブリッジ
- 設定、履歴、ブックマーク、ダウンロード、セッションスナップショットの SQLite 永続化
- 通常ウィンドウとプライベートウィンドウの扱い。プライベートウィンドウでは CEF の off-the-record request context を使用します。
- 設定、履歴、ブックマーク、ダウンロード、デバッグ、新規タブなどの内部ページ
- capability check、audit event、rate limiting を備えた外部コマンド境界
- ローカルのテスト、リント、フォーマット、監査、CI ワークフロー

## Known Limitations / 既知の制限

Fubuki Browser Alpha は、まだプロダクション用途のブラウザではありません。

- CEF バイナリは外部で設定し、リポジトリには含めません。
- コード署名、ノータリゼーション、アップデート配信、リリースパッケージングは現在の MVP スコープ外です。
- パスワードマネージャー、ブラウザ同期、Chrome 拡張機能互換、アップデーター、完成度の高いインポート / エクスポートは含まれていません。
- セッション復元はウィンドウと最後のタブ URL を復元しますが、ページ内の完全な履歴スタックまでは復元しません。
- プライベートウィンドウは off-the-record request context を使用し、通常のアプリデータ書き込みを避けます。ただし、ユーザーが保存したダウンロードファイルはディスク上に残ります。
- キャッシュとサイトデータの削除は保守的で、CEF request context の挙動に依存します。
- ブックマークフォルダとブラウザ互換のインポート / エクスポートには、より完全な UX と永続化処理が必要です。
- UI とコンテンツのレイアウトはまだネイティブ child view を使っています。完全なネイティブツールバーはありません。
- macOS Apple Silicon が主なターゲットです。Intel 互換性は利用する CEF ビルドに依存します。

現在の制限は [docs/known-limitations.md](docs/known-limitations.md) も参照してください。

## Data Directory / データディレクトリ

```text
~/Library/Application Support/Fubuki Browser Alpha/
```

アプリは、このディレクトリ配下に CEF プロファイルデータと `fubuki.sqlite3` を保存します。SQLite データベースは、設定、履歴、ブックマーク、ダウンロード、セッションスナップショットなどのブラウザ所有データに使います。

## Documentation / ドキュメント

- [Architecture](docs/architecture.md)
- [FrostEngine Plan](docs/frost-engine-plan.md)
- [Bridge API](docs/bridge-api.md)
- [Commands](docs/commands.md)
- [Events](docs/events.md)
- [Internal Pages](docs/internal-pages.md)
- [Known Limitations](docs/known-limitations.md)

## Contributing / 貢献

プルリクエストを作成する前に [CONTRIBUTING.md](CONTRIBUTING.md) を読んでください。

## License / ライセンス

[MIT License](LICENSE) — Copyright (c) 2026 TeamFubuki
