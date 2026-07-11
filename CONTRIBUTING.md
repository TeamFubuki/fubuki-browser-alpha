# Contributing Guide / Fubuki Browser Alpha 貢献ガイド

**Keep the boundary clean. Build with intent.**

Fubuki Browser Alpha への貢献ありがとうございます。このドキュメントでは、開発環境、ブランチ運用、コミット規約、PR の確認項目、実装方針をまとめます。

Fubuki Browser Alpha は、macOS ネイティブホスト、CEF、Rust 製 FrostEngine、SolidJS UI を分離して構成するブラウザ基盤です。このプロジェクトで重要なのは、機能を足すことだけではありません。どのレイヤーが状態を持つのか、どの境界を通して通信するのか、どの操作が安全に実行できるのかを崩さないことです。

`Alpha` はプロダクト名・コードネームであり、リリース段階としての「アルファ版」を意味しません。Issue、PR、ドキュメントで成熟度を説明する場合は、必要に応じて `MVP`、`experimental`、`not production-ready` など、実態に近い表現を使ってください。

## Contribution Scope / 貢献できること

Contributions are welcome, but architecture comes first.

以下のような貢献を歓迎します。

- バグ修正
- テスト追加
- ドキュメント改善
- UI の改善
- Frost Protocol の整理
- FrostEngine の設計改善
- CEF / macOS Host の安定化
- CI、lint、format、audit まわりの改善
- 既知の制限を明確にする Issue / PR

一方で、以下のような変更は慎重に扱います。

- UI にブラウザ状態の source of truth を持たせる変更
- C++ ホスト側に二重の論理状態 store を作る変更
- Frost Protocol を迂回するショートカット
- capability check や audit event を避ける外部自動操作
- セキュリティ上の制限を解決済みのように見せる変更

## Development / 開発環境

### Requirements / 必要環境

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

Apple Silicon macOS を主なターゲットにしています。Intel macOS でも対応する CEF binary distribution を使えば動作する可能性はありますが、互換性は選択した CEF ビルドに依存します。

### First-time Setup / 初回セットアップ

```bash
git clone https://github.com/TeamFubuki/fubuki-browser-alpha.git
cd fubuki-browser-alpha
make bootstrap
```

`make bootstrap` は、CEF の取得、UI 依存関係のインストール、ネイティブ CMake ビルド設定をまとめて実行します。

### Build and Run / ビルドと実行

```bash
make build
make run
```

レイヤーごとに作業する場合は、以下を使います。

```bash
make ui       # SolidJS UI をビルド
make rust     # FrostEngine の Rust crate をビルド
make native   # C++ / CEF ホストをビルド
```

CEF や CMake まわりの変更後にネイティブビルドが失敗する場合は、設定を作り直してください。

```bash
make clean
make configure
make native
```

## Boundaries / プロジェクト境界

**State belongs to FrostEngine. Side effects belong to the host. UI stays a client.**

アーキテクチャ上の境界を崩さないでください。

```text
Fubuki Browser UI (SolidJS)
    ↓ Frost Protocol
FrostEngine Core (Rust)
    ↓ HostCommand / HostEvent
CEF / macOS Host (C++20)
```

### Ownership Rules / 所有権のルール

- **Engine owns state** — タブ、ウィンドウ、設定、セッション、ブラウザ所有の永続化データは FrostEngine に置きます。
- **Host owns I/O** — C++ は NSWindow、CEF Browser インスタンス、scheme 処理、ホスト側の副作用を管理します。
- **UI is a client** — SolidJS は状態を描画し、プロトコルリクエストを送信します。ブラウザ状態の信頼できる情報源にはしません。
- **Protocol first** — 機能がレイヤーをまたぐ場合は、先に関連する Request / Response / Event / Host 境界の型を更新してください。
- **Audit by default** — 外部自動操作では capability check、rate limiting、audit event を迂回しないでください。
- **No destructive GET** — 内部ページで状態を変更する場合は、安全な操作境界を使います。

## Branches / ブランチ戦略

`main` から短い topic branch を作成してください。

| Branch | Purpose |
|---|---|
| `feature/*` | ユーザー向けまたは内部向けの新機能 |
| `fix/*` | バグ修正 |
| `docs/*` | ドキュメントのみの変更 |
| `refactor/*` | 挙動を変えない内部整理 |
| `deps/*` | 依存関係の更新 |
| `ci/*` | CI またはリポジトリ自動化の変更 |

メンテナーから別の指示がない限り、プルリクエストは `main` に向けて作成してください。

## Commit Style / コミット規約

[Conventional Commits](https://www.conventionalcommits.org/ja/v1.0.0/) を使用します。

```text
<type>(<scope>): <description>
```

### type

| type | 意味 |
|---|---|
| `feat` | 新機能 |
| `fix` | バグ修正 |
| `docs` | ドキュメントのみの変更 |
| `style` | フォーマットのみの変更 |
| `refactor` | 新機能追加やバグ修正を目的としないコード変更 |
| `perf` | パフォーマンス改善 |
| `test` | テストの追加または更新 |
| `build` | ビルドシステムまたは依存関係の変更 |
| `ci` | CI または自動化の変更 |
| `chore` | メンテナンス |
| `revert` | 以前のコミットの取り消し |

### Recommended scope / 推奨 scope

| scope | 対象 |
|---|---|
| `ui` | `ui/` 配下の SolidJS UI |
| `rust` | `crates/` 配下の Rust workspace |
| `native` | C++ / CEF / macOS ホスト |
| `protocol` | Frost Protocol の Request / Response / Event 型 |
| `store` | SQLite 永続化 |
| `docs` | ドキュメント |
| `deps` | 依存関係 |
| `ci` | GitHub Actions とリポジトリチェック |
| `release` | リリース準備 |

例:

```text
feat(protocol): add tab duplicate request
fix(native): reject destructive settings GET navigation
docs(architecture): clarify host state ownership
deps(ui): update Tailwind CSS
```

## Pull Requests / プルリクエスト

Pull requests should be small, reviewable, and honest about risk.

PR を作成する前に、以下を確認してください。

1. 最新の `main` を rebase または merge する。
2. 変更に関係するチェックを実行する。
3. 挙動、コマンド、アーキテクチャ、制限事項が変わる場合はドキュメントを更新する。
4. 見た目が変わる UI 変更には、スクリーンショットまたは短い録画を付ける。
5. 未完成の点を解決済みのように書かず、既知の制限として明記する。

A good PR includes:

- 何を変更したか
- なぜ変更したか
- 影響するレイヤー
- テスト結果
- 関連 Issue があればそのリンク
- 残っているリスクまたは follow-up

### PR Checklist / PR チェックリスト例

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

このチェックリストは目安です。実際に実行していないコマンドを、実行済みとして記載しないでください。

## Review Standard / レビュー基準

レビューでは、単に動くかどうかだけではなく、以下を確認します。

- レイヤー責務が崩れていないか
- UI、Engine、Host の境界が明示的か
- protocol change が型とドキュメントに反映されているか
- private window、download、history、settings などの永続化影響が説明されているか
- security boundary を弱めていないか
- テストまたは検証方法が十分か

議論が必要な変更は歓迎します。ただし、大きな設計変更は Issue や draft PR で先に方向性を確認してください。

## Local Checks / ローカルチェック

レビュー依頼前に、広めのチェックを実行してください。

```bash
make test
make lint-all
make format-all
```

依存関係とポリシーの確認には、以下を使います。

```bash
make audit
make audit-deny
```

全体実行が不要な場合は、レイヤーごとのコマンドも使えます。

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

## CI Coverage / CI の確認内容

`main` 向けのプルリクエストでは、以下の CI チェックを実行します。

| Job | Coverage |
|---|---|
| UI check | Oxlint、Oxfmt、TypeScript build、Vitest |
| Rust check | rustfmt、Clippy、`cargo test --workspace` |
| Rust FFI build | `frost-ffi` の release static library ビルド |
| Rust security audit | `cargo-audit` |
| Rust license & advisory deny | `cargo-deny` |
| Rust docs build | warnings denied での `cargo doc` |
| UI dependency audit | `pnpm audit --audit-level=high` |
| Native unit test | CMake、GoogleTest、CTest |
| Repository hygiene | merge conflict marker と巨大ファイルの検出 |

重い macOS native CEF ビルドは、`main` または手動実行向けです。PR の時点で完全な app bundle まで検証済みだとは考えないでください。

## Engineering Rules / コーディング方針

### Rust / FrostEngine

- 状態変更は UI や host glue ではなく、FrostEngine の service に置きます。
- ad-hoc な JSON より、型付きの Request / Response / Event model を優先します。
- `cargo fmt --all` を実行し、Clippy は warnings denied で通します。
- public API には有用な doc comment を付けます。
- エラー型は明示的に定義します。workspace では `thiserror` を使用できます。
- service の挙動や protocol-level の変更にはテストを追加します。

### Frost Protocol

- protocol message は実装詳細ではなく境界として扱います。
- Request、Response、Event の payload は、versioning と serialization を意識して設計します。
- UI 固有または CEF 固有の前提を protocol type に漏らさないでください。
- protocol message を追加または改名する場合は、ドキュメントも更新してください。

### C++ / macOS / CEF Host

- C++20 を使用します。
- ホストは薄く保ちます。描画、host command の実行、CEF callback の転送、結果報告を担当します。
- C++ 側に二重の論理状態 store を作らないでください。
- CEF の lifetime と threading のルールを慎重に扱います。
- `clang-format` で整形し、Clang-Tidy / cppcheck の指摘は実質的な問題であれば対応してください。
- `fubuki://` scheme handler は単なるルーティングではなく、セキュリティ境界として扱います。
- CEF バイナリをリポジトリに vendoring しないでください。

### SolidJS UI

- UI は protocol client として扱います。
- Solid の reactive primitive を意図的に使い、関係のない component に状態変更を隠さないでください。
- 明確な境界上の理由がない限り、`any` は避けます。
- ブラウザ操作は bridge layer を通してください。
- スタイリングには Tailwind CSS を使います。
- ネイティブホストなしで検証できる state、bridge、component の挙動には Vitest を使います。

### Persistence / 永続化

- ブラウザ所有データは Frost Store layer を通してください。
- migration は決定的で、再実行しても安全な形にします。
- private window の状態を通常の app persistence に保存しないでください。
- 破壊的なデータ削除は保守的に扱い、制限事項をドキュメント化してください。

### External Automation / 外部自動操作

外部自動操作は、宣言された capability と監査可能な command path を通して接続してください。capability check、rate limiting、audit event を迂回する ad-hoc な自動操作ショートカットは追加しないでください。

## File Map / ファイル構成

| Path | Purpose |
|---|---|
| `crates/frost-protocol/` | Request、Response、Event、Host 境界の schema |
| `crates/frost-core/` | ブラウザ状態と操作の core |
| `crates/frost-store/` | SQLite 永続化 layer |
| `crates/frost-engine-api/` | Engine adapter 境界 |
| `crates/frost-ffi/` | Rust-to-C FFI layer |
| `native/macos-cef-host/` | macOS CEF host の project file |
| `native/src/` | ネイティブホスト実装 |
| `native/tests/` | ネイティブ単体テスト |
| `ui/src/` | SolidJS ブラウザ UI |
| `ui/src/bridge/` | Frost Protocol client bridge |
| `docs/` | アーキテクチャ、protocol、command、event、制限事項のドキュメント |
| `scripts/` | bootstrap、build、CEF、cleanup 用スクリプト |

## Testing Policy / テスト方針

挙動を変更する場合は、テストを追加または更新してください。

- 新しいエンジン挙動には Rust テストを追加します。
- protocol 変更では、serialization と期待される Request / Response の挙動を確認します。
- UI 変更には、可能な範囲で Vitest の coverage を追加します。
- フルアプリを起動せずに検証できるネイティブホストロジックには、ネイティブ単体テストを使います。
- バグ修正には、現在の構造で合理的に難しい場合を除き、回帰テストを追加します。

## Documentation Policy / ドキュメント方針

以下に影響する変更では、ドキュメントを更新してください。

- セットアップまたはビルドコマンド
- アーキテクチャまたはレイヤー所有権
- Frost Protocol、HostCommand、HostEvent の契約
- 内部ページまたはブラウザ所有の画面
- 既知の制限
- セキュリティに関係する挙動

README では、実装より広いことを主張しないでください。実装が部分的であれば、部分的であると明記してください。

## Issues / Issue

バグを報告する場合は、以下を含めてください。

- 再現手順
- 期待される挙動
- 実際の挙動
- macOS バージョンとハードウェア
- ブランチまたは commit SHA
- 有用なログ、スクリーンショット、録画

機能要望では、以下を含めてください。

- 動機
- 提案する挙動
- 影響するレイヤー
- 互換性またはセキュリティ上の懸念
- スコープ外にできる範囲

## Security Notes / セキュリティメモ

このプロジェクトはブラウザに近いコードを含みます。現在の MVP スコープでも、境界はセキュリティ上重要なものとして扱ってください。

- privileged operation を通常の page navigation から露出しないでください。
- `fubuki://app/` と内部ページは、任意の Web content から分離してください。
- capability check と auditability なしに、広い host capability を追加しないでください。
- セキュリティ上の制限を、解決済みのように書かず、制限事項として記録してください。

## Documentation / ドキュメント

- [Project Overview](README.md)
- [Architecture](docs/architecture.md)
- [FrostEngine Plan](docs/frost-engine-plan.md)
- [Bridge API](docs/bridge-api.md)
- [Commands](docs/commands.md)
- [Events](docs/events.md)
- [Internal Pages](docs/internal-pages.md)
- [Known Limitations](docs/known-limitations.md)

## License / ライセンス

貢献された内容は、[MIT License](LICENSE) の下で配布されることに同意したものとみなします。
