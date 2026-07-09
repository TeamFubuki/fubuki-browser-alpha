# 貢献ガイド

Fubuki Browser Alpha へのご関与ありがとうございます。このドキュメントでは、開発環境の構築からプルリクエストの提出まで、貢献に必要な手順を説明します。

## 開発環境の構築

### 必要なツール

| ツール | バージョン |
|---|---|
| macOS | 12 以降 |
| Xcode Command Line Tools | 最新版 |
| CMake | 3.21 以降 |
| Rust | 1.96.1 以降 |
| Node.js | 20 以降 |
| pnpm | 11 以降 |
| LLVM (Homebrew) | 最新版（Clang-Format / Clang-Tidy） |

### 初回セットアップ

```bash
git clone https://github.com/TeamFubuki/fubuki-browser-alpha.git
cd fubuki-browser-alpha
make bootstrap
```

`make bootstrap` は以下の処理を一括で実行します。

1. CEF バイナリを `third_party/cef/` にダウンロード
2. `ui/` で `pnpm install` を実行
3. ネイティブビルドの CMake 設定

### ビルドと実行

```bash
make build    # UI + Rust + ネイティブを全ビルド
make run      # アプリを起動
```

## ブランチ戦略

- `main` — リリース可能な安定ブランチ
- `feature/*` — 新機能の開発ブランチ
- `fix/*` — バグ修正の開発ブランチ
- `deps/*` — 依存関係の更新

プルリクエストは `main` ブランチに向けて作成してください。

## コミット規約

[Conventional Commits](https://www.conventionalcommits.org/ja/v1.0.0/) に従ってください。

### 形式

```
<type>(<scope>): <description>
```

### タイプ

| タイプ | 説明 |
|---|---|
| `feat` | 新機能 |
| `fix` | バグ修正 |
| `docs` | ドキュメントのみの変更 |
| `style` | コードの見た目に影響する変更（スペース、セミコロンなど） |
| `refactor` | 機能の追加やバグ修正を行わないリファクタリング |
| `perf` | パフォーマンス改善 |
| `test` | テストの追加や修正 |
| `build` | ビルドシステムや外部依存関係の変更 |
| `ci` | CI/CD の設定変更 |
| `chore` | その他のメンテナンス |
| `revert` | コミットの reverted |

### スコープ

| スコープ | 対象 |
|---|---|
| `ui` | SolidJS UI (`ui/`) |
| `rust` | FrostEngine (`crates/`) |
| `native` | CEF / macOS Host (`native/`) |
| `protocol` | Frost Protocol |
| `docs` | ドキュメント |
| `deps` | 依存関係更新 |
| `ci` | CI/CD |
| `release` | リリース関連 |

### 例

```
feat(rust): タブの検索機能を実装
fix(ui): ドラッグ＆ドロップ中のズーム表示バグを修正
docs(architecture): FrostEngine の設計ドキュメントを更新
deps(ui): Tailwind CSS を v4.3.2 に更新
```

## プルリクエストの作成

### 提出手順

1. `main` から最新のコードを取得
2. ブランチを作成（`git checkout -b feature/my-feature`）
3. 変更を加えてコミット
4. `git push origin feature/my-feature`
5. GitHub でプルリクエストを作成

### プルリクエストの要件

- **タイトル**: コミット規約に準拠した形式で記述
- **説明文**: 変更内容、動機、関連 Issue を明記
- **チェック必須**: CI が全てパスしていること

### CI チェック内容

プルリクエストでは以下の CI が自動実行されます。

| ジョブ | 内容 |
|---|---|
| UI check | Oxlint / Oxfmt / TypeScript / Vitest |
| Rust check | rustfmt / Clippy / cargo test |
| Rust FFI build | frost-ffi のスタティックライブラリビルド |
| Rust security audit | cargo-audit |
| Rust license & advisory deny | cargo-deny |
| Rust docs build | rustdoc |
| UI dependency audit | pnpm audit |
| Native unit test | CMake + GoogleTest |
| Repository hygiene | 競合マーカー・巨大ファイル検出 |

> macOS ネイティブビルド（CEF ビルド含む）は `main` ブランチへのマージ時または手動実行時のみ実行されます。

## コーディング規約

### 共通

- 新機能は FrostEngine Core（`crates/`）に追加し、Protocol に型を定義する
- UI や Host への影響を最小限に抑える
- `any` 型の使用を避ける（TypeScript）
- エラーハンドリングを怠らない

### Rust（FrostEngine）

- Rust の慣用パターンに従い、所有権と借用を正しく活用する
- Clippy の警告を全て修正する（`-D warnings`）
- rustfmt でフォーマットする
- パブリック API には doc コメントを付ける
- エラータイプには `thiserror` を使用する

### C++（ネイティブ / CEF）

- C++20 標準を使用する
- CEF の API パターンに従う
- Clang-Format でフォーマットする
- Clang-Tidy / cppcheck の警告に対処する
- ネイティブは論理状態を保持しない（CEF 表示と入力のみ）

### SolidJS（UI）

- SolidJS のリアクティブモデルを活用する
- reactive primitives を多用する
- Tailwind CSS でスタイルを管理する
- TypeScript の strict モードを使用する
- Vitest でテストを書く

### ファイル構成

| パス | 説明 |
|---|---|
| `crates/frost-protocol/` | プロトコル型定義 |
| `crates/frost-core/` | ブラウザ状態管理コア |
| `crates/frost-store/` | SQLite 永続化 |
| `crates/frost-engine-api/` | EngineAdapter trait |
| `crates/frost-ffi/` | C FFI レイヤー |
| `native/macos-cef-host/` | CEF / macOS 固有処理 |
| `ui/src/` | SolidJS アプリケーション |
| `ui/src/bridge/` | Frost Protocol クライアント |
| `ui/src/components/` | UI コンポーネント |
| `ui/src/hooks/` | SolidJS フック |
| `ui/src/stores/` | ステート管理 |

## テスト

### ローカルでテストを実行

```bash
make test           # 全テスト
make test-rust      # Rust テスト
make test-ui        # UI テスト
make test-native    # ネイティブテスト
```

### テスト方針

- 新機能にはテストを付ける
- バグ修正には再現テストを付ける
- ブリッジ通信の変更にはプロトコルレベルのテストを付ける

## リントとフォーマット

変更をプルリクエストする前に、ローカルで確認してください。

```bash
make lint-all       # 全リント
make format-all     # 全フォーマット
make audit          # Rust 依存関係の脆弱性チェック
```

## Issue の作成

バグ報告や機能リクエストは Issues からお願いします。

### バグ報告テンプレート

- **再現手順**: 具体的な手順を記載
- **期待される動作**: 正しくあるべき挙動
- **実際の動作**: 実際に観察された挙動
- **環境**: macOS のバージョン、ハードウェア構成

### 機能リクエストテンプレート

- **動機**: なぜその機能が必要か
- **提案内容**: 具体的な仕様
- **影響範囲**: どのコンポーネントに影響するか

## ドキュメント

- [プロジェクト概要](README.md)
- [アーキテクチャ](docs/architecture.md)
- [FrostEngine 計画](docs/frost-engine-plan.md)
- [ブリッジ API](docs/bridge-api.md)
- [コマンド定義](docs/commands.md)
- [イベント定義](docs/events.md)
- [内部ページ](docs/internal-pages.md)

## ライセンス

本プロジェクトは [MIT License](LICENSE) の下で公開されています。貢献されたコードも MIT ライセンスの下で公開されることに同意したものとみなされます。
