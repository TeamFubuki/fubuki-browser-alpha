# Fubuki Browser Alpha

**macOS ファーストのブラウザシェル** — C++20 / CEF / SolidJS

Fubuki Browser Alpha は、Electron や Tauri に依存しない高速なブラウザシェルです。C++20 と Chromium Embedded Framework (CEF) をネイティブレンダリングに、SolidJS を UI レイヤーに使用し、Rust 製の状態管理コア（FrostEngine）と Frost Protocol で接続します。

## アーキテクチャ

```
Fubuki Browser UI (SolidJS)
    ↓  Frost Protocol (JSON-RPC)
FrostEngine Core (Rust)
    ↓  HostCommand / HostEvent
CEF / macOS Host (C++)
```

2 つのコンポーネントで構成され、それぞれ独立してバージョニングされます。

| コンポーネント | 場所 | 職責 |
|---|---|---|
| **Fubuki Browser UI** | `ui/` | SolidJS フロントエンド、Frost Protocol クライアント |
| **FrostEngine** | `crates/` | Rust ベースのブラウザ状態管理コア |

詳細は [docs/architecture.md](docs/architecture.md) を参照してください。

## システム要件

| ツール | バージョン |
|---|---|
| macOS | 12 以降 |
| Xcode Command Line Tools | 最新版 |
| CMake | 3.21 以降 |
| Rust | 1.96.1 以降 |
| Node.js | 20 以降 |
| pnpm | 11 以降 |
| LLVM (Homebrew) | 最新版 |

Electron、Tauri、WKWebView、WebView2 は一切使用していません。

## セットアップ

```bash
# CEF ダウンロード・UI 依存関係インストール・ネイティブビルド設定を一括実行
make bootstrap
```

### CEF の取得

```bash
make cef
```

Apple Silicon では `macosarm64`、Intel では `macosx64` のビルドが自動選択され、`third_party/cef/` にダウンロードされます。

手動で CEF を配置する場合は [CEF Automated Builds](https://cef-builds.spotifycdn.com/index.html) からダウンロードし、`third_party/cef/` に展開するか、CMake 設定時に `-DCEF_ROOT=/path/to/cef_binary` を指定してください。

> 本リポジトリは CEF バイナリをバージョン管理しません。

## ビルド

```bash
# 全コンポーネントをビルド（UI + Rust + ネイティブ）
make build
```

個別にビルドすることもできます。

```bash
make ui          # SolidJS UI
make rust        # FrostEngine（Rust）
make native      # ネイティブアプリ（C++/CEF）
```

## 実行

```bash
make run
```

`make bootstrap && make build && make run` で初回セットアップから起動まで一括実行できます。

## テスト

```bash
make test            # 全テスト実行（Rust + UI + ネイティブ）
make test-rust       # FrostEngine テスト
make test-ui         # Vitest（UI）
make test-native     # GoogleTest（ネイティブ）
```

## リント・フォーマット

```bash
make lint            # Oxlint（UI）
make lint-fix        # Oxlint（UI・自動修正）
make format          # Oxfmt（UI）
make format-check    # フォーマット確認（UI）

make lint-rust       # Clippy（Rust）
make format-rust     # rustfmt（Rust）

make lint-native     # Clang-Tidy / cppcheck（C++）
make format-native   # Clang-Format（C++）

make lint-all        # 全リント実行
make format-all      # 全フォーマット実行
```

## セキュリティ監査

```bash
make audit           # cargo-audit（Rust 依存関係の脆弱性チェック）
make audit-deny      # cargo-deny（ライセンス・アドバイザリチェック）
```

## ビルド一覧

| ターゲット | 説明 |
|---|---|
| `make bootstrap` | 初回セットアップ（CEF ダウンロード + UI 依存関係 + 設定） |
| `make cef` | CEF のダウンロード・更新 |
| `make ui` | SolidJS UI ビルド |
| `make rust` | FrostEngine（Rust）ビルド |
| `make configure` | CMake 設定 |
| `make native` | ネイティブアプリビルド |
| `make build` | 全コンポーネントビルド |
| `make run` | ビルド＆実行 |
| `make test` | 全テスト実行 |
| `make clean` | ビルド成果物の削除 |
| `make distclean` | CEF バイナリを含む全成果物の削除 |

## キーボードショートカット

| ショートカット | アクション |
|---|---|
| `Cmd+L` | URL / 検索バーにフォーカス |
| `Cmd+N` | 新規ウィンドウ |
| `Cmd+Shift+N` | 新規プライベートウィンドウ |
| `Cmd+T` | 新規タブ |
| `Cmd+W` | タブを閉じる |
| `Cmd+Shift+W` | ウィンドウを閉じる |
| `Cmd+Shift+T` | 閉じたタブを開き直す |
| `Cmd+R` | リロード |
| `Cmd+F` | ページ内検索 |
| `Cmd+[` / `Cmd+]` | 戻る / 進む |
| `Cmd+D` | ブックマークに追加 |
| `Cmd+,` | 設定を開く |
| `Cmd++` / `Cmd+-` / `Cmd+0` | ズームイン / ズームアウト / リセット |

## 実装済み機能

- ウィンドウ管理 — 新規 / プライベート / 閉じる / タブ移動
- タブ管理 — 作成 / 切り替え / 閉じる / ピン留め / 複製 / 閉じたタブを開き直す / 検索 / 並べ替え
- ナビゲーション — 戻る / 進む / リロード / ストップ / ホーム / オムニバー
- セッション復元 — ウィンドウ / タブ / アクティブタブ / ピン留め状態
- プライベートウィンドウ — オフレコード CEF リクエストコンテキスト
- ページ操作 — 検索 / ズーム / 印刷 / ソース表示 / DevTools
- ダウンロード管理 — 進行状況 / 開く / Finder で表示 / 削除
- 内蔵ページ — 履歴 / ブックマーク / ダウンロード / 設定 / 新規タブ / デバッグ
- SQLite 永続化 — 履歴 / ブックマーク / ダウンロード / 設定 / 権限 / デバッグログ
- バージョンド JSON ブリッジ（`fubuki://app/` のみアクセス可能）
- コマンドレジストリとイベントバス
- エラーページ（ナビゲーション失敗時）

## 設定データ

```text
~/Library/Application Support/Fubuki Browser Alpha/
```

CEF プロファイル（クッキー、LocalStorage、IndexedDB）と `fubuki.sqlite3`（履歴、ブックマーク、ダウンロード、設定、セッションスナップショット）が保存されます。

## 既知の制限事項

- CEF バイナリ、コード署名、ノータリゼーション、アップデート配信は未対応
- パスワードマネージャーと同期機能は未実装
- Chrome 拡張互換性は未実装
- キャッシュ / サイトデータの削除は保守的
- ブックマークフォルダーとブラウザ間インポート / エクスポートは未完成

## ドキュメント

- [アーキテクチャ](docs/architecture.md)
- [FrostEngine 計画](docs/frost-engine-plan.md)
- [ブリッジ API](docs/bridge-api.md)
- [コマンド定義](docs/commands.md)
- [イベント定義](docs/events.md)
- [内部ページ](docs/internal-pages.md)
- [既知の制限事項](docs/known-limitations.md)

## ライセンス

[MIT License](LICENSE) — Copyright (c) 2026 TeamFubuki
