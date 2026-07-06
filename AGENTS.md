# Fubuki Browser Alpha - エージェントガイド

## プロジェクト概要

Fubuki Browser Alpha は macOS ファーストのブラウザシェルです。Electron や Tauri は使わず、C++20/CEF と SolidJS を組み合わせて高速なブラウジング体験を提供します。

## 技術スタック

### ネイティブ側 (`native/`)
- **言語**: C++20
- **ビルド**: CMake 3.21+
- **ブラウザエンジン**: Chromium Embedded Framework (CEF)
- **対応OS**: macOS 12+

### UI 側 (`ui/`)
- **フレームワーク**: SolidJS
- **ビルドツール**: Vite 8.x
- **CSS**: Tailwind CSS 4.x
- **言語**: TypeScript 6.x
- **テスト**: Vitest
- **パッケージマネージャ**: pnpm 11+

## ディレクトリ構成

```
/
├── native/           # C++ ネイティブアプリ
│   ├── src/          # ソースコード（app, bridge, browser, cef, commands, events）
│   └── CMakeLists.txt
├── ui/               # SolidJS ブラウザUI
│   ├── src/
│   │   ├── bridge/   # ネイティブとの通信層
│   │   ├── components/
│   │   ├── hooks/
│   │   ├── stores/
│   │   └── styles/
│   └── package.json
├── third_party/      # CEF バイナリ配置先
├── scripts/          # ビルド・セットアップスクリプト
└── docs/             # ドキュメント
```

## 実装ルール

### コーディング規約

1. **ネイティブ側**: C++20 標準を使用し、CEF の API パターンに従う
2. **UI 側**: SolidJS のリアクティブモデルを活用し、reactive primitives を多用する
3. **型安全性**: TypeScript の strict モードを使用し、any 型の使用を避ける
4. **命名規則**: キャメルケース（変数・関数）、パスカルケース（クラス・コンポーネント）

### ブリッジ通信

- `fubuki://app/` のみがネイティブブリッジにアクセス可能
- ブリッジはバージョニングされており、後方互換性を維持する
- 機能追加時は `commands.list` で UI 側にコマンド一覧を提供する

### アーキテクチャ

- **イベント駆動**: ウィンドウ、タブ、ナビゲーション等の状態変更はイベントバスで伝播
- **コマンドレジストリ**: ブラウザ操作は統一されたコマンドとして登録・実行（`commands.list` と `commands.execute` で UI 側に一覧提供・操作実行）
- **セッション管理**: 通常ウィンドウとプライベートウィンドウでリクエストコンテキストを分離

### ビルドとテスト

```bash
# UI 開発
cd ui && pnpm dev

# UI テスト
cd ui && pnpm test

# ネイティブビルド
make native

# 一括ビルド＆実行
make bootstrap && make build && make run
```

### 注意事項

- CEF バイナリはリポジトリに含めず、`make cef` でダウンロード
- プロファイルデータは `~/Library/Application Support/Fubuki Browser Alpha/` に保存
- 新機能実装時は既存のイベントバスやコマンドレジストリを活用すること
