# Fubuki Browser Alpha - エージェントガイド

## プロジェクト概要

Fubuki Browser Alpha は macOS ファーストのブラウザシェルです。Electron や Tauri は使わず、C++20/CEF と SolidJS を組み合わせて高速なブラウジング体験を提供します。

2 つのコンポーネントで構成され、それぞれ独立してバージョニングします。

| コンポーネント | 場所 | 職責 |
|---|---|---|
| **Fubuki Browser UI** | `ui/` | SolidJS フロントエンド、Frost Protocol クライアント |
| **FrostEngine** | `crates/` | Rust ベースのブラウザ状態管理コア |

## アーキテクチャ

```
Fubuki Browser UI (SolidJS)
    ↓  Frost Protocol (JSON-RPC over CEF message router)
FrostEngine Core (Rust)
    ↓  EngineAdapter trait
CEF / macOS Host (C++)
```

詳細は `docs/architecture.md` と `docs/frost-engine-plan.md` を参照してください。

## 技術スタック

### FrostEngine (`crates/`)
- **言語**: Rust
- **crate**:
  - `frost-protocol` — プロトコル型定義（request, response, event, state）
  - `frost-core` — BrowserCore, TabService, WindowService, SettingsService, SessionService
  - `frost-store` — SQLite 永続化、repository trait、migration
  - `frost-engine-api` — EngineAdapter, PageAdapter, WindowHost の trait 定義

### CEF / macOS Host (`native/macos-cef-host/`)
- **言語**: C++20
- **ビルド**: CMake 3.21+
- **ブラウザエンジン**: Chromium Embedded Framework (CEF)
- **対応OS**: macOS 12+

### Fubuki Browser UI (`ui/`)
- **フレームワーク**: SolidJS
- **ビルドツール**: Vite 8.x
- **CSS**: Tailwind CSS 4.x
- **言語**: TypeScript 6.x
- **テスト**: Vitest
- **パッケージマネージャ**: pnpm 11+

## ディレクトリ構成

```
/
├── crates/                  # Rust crate（FrostEngine）
│   ├── frost-protocol/      # プロトコル型定義
│   ├── frost-core/          # ブラウザ状態管理コア
│   ├── frost-store/         # SQLite 永続化
│   └── frost-engine-api/    # EngineAdapter trait
├── native/
│   ├── macos-cef-host/      # CEF / macOS 固有処理
│   └── CMakeLists.txt
├── ui/                      # SolidJS ブラウザUI
│   ├── src/
│   │   ├── bridge/          # Frost Protocol クライアント
│   │   ├── components/
│   │   ├── hooks/
│   │   ├── stores/
│   │   └── styles/
│   └── package.json
├── third_party/             # CEF バイナリ配置先
├── scripts/                 # ビルド・セットアップスクリプト
└── docs/                    # ドキュメント
```

## 実装ルール

### コーディング規約

1. **FrostEngine**: Rust の慣用パターンに従い、所有権と借用を正しく活用する
2. **CEF / macOS Host**: C++20 標準を使用し、CEF の API パターンに従う
3. **UI 側**: SolidJS のリアクティブモデルを活用し、reactive primitives を多用する
4. **型安全性**: TypeScript の strict モードを使用し、any 型の使用を避ける
5. **命名規則**: Rust はスネークケース、C++/TypeScript はキャメルケース（変数・関数）、パスカルケース（クラス・コンポーネント）

### ブリッジ通信

- `fubuki://app/` のみが Frost Protocol ブリッジにアクセス可能
- ブリッジは Frost Protocol v0 に従う（`docs/architecture.md` 参照）
- 機能追加時は `frost-protocol` に Request/Response 型を追加する
- Native は論理状態を所有しない。`BrowserDataStore` は削除され、永続化はエンジン所有の `frost-store`（SQLite）へ `FrostStore` FFI 経由で委譲する
- `host.syncSnapshot` 等の逆向き同期は行わない。FrostEngine が source of truth
- External / MCP クライアントは `frost-core::external_router` 経由で接続し、capability ゲート・audit・rate limit を通す。CEF/NSWindow へ直接触れてはならない
- destructive action は URL GET や UI 内リンクで発火させない。`fubuki://settings/set?...` の GET は scheme handler で 403 拒否する

### アーキテクチャ原則

- **状態の所有者は FrostEngine Core のみ**。CEF/macOS Host は表示と入力のみ担当
- **差分イベントで同期**。起動時だけ `app.snapshot` で全状態取得、以後はイベントで差分更新
- **EngineAdapter trait でホストを切り替える**。CEF に依存するコードは Host 側に閉じる
- **新機能は Core に追加する**。UI や Host への影響を最小化する
- **内部ページはキャッシュを使用**。`fubuki://settings/` 等のページはLRUキャッシュで最適化

### ビルドとテスト

```bash
# UI 開発
cd ui && pnpm dev

# UI テスト
cd ui && pnpm test

# Rust テスト
cargo test --workspace

# Rust リント
cargo clippy --workspace -- -D warnings

# Rust フォーマット
cargo fmt --all

# ネイティブビルド
make native

# FrostEngine ビルド
make rust

# 一括ビルド＆実行
make bootstrap && make build && make run
```

### 注意事項

- CEF バイナリはリポジトリに含めず、`make cef` でダウンロード
- プロファイルデータは `~/Library/Application Support/Fubuki Browser Alpha/` に保存
- 新機能実装時は FrostEngine Core に追加し、Protocol に型を定義すること
- 移行中は旧 Bridge API と Frost Protocol が併存する場合がある
