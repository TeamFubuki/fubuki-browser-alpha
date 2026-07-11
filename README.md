# Fubuki Browser Alpha

[![CI](https://img.shields.io/github/actions/workflow/status/TeamFubuki/fubuki-browser-alpha/ci.yml?branch=main&style=flat-square&label=CI)](https://github.com/TeamFubuki/fubuki-browser-alpha/actions/workflows/ci.yml)
[![License](https://img.shields.io/github/license/TeamFubuki/fubuki-browser-alpha?style=flat-square&label=License)](LICENSE)
![Platform](https://img.shields.io/badge/platform-macOS-111111?style=flat-square)
![C++20](https://img.shields.io/badge/C%2B%2B-20-00599C?style=flat-square)
![Rust](https://img.shields.io/badge/Rust-stable-b7410e?style=flat-square)
![SolidJS](https://img.shields.io/badge/UI-SolidJS-2c4f7c?style=flat-square)
![CEF](https://img.shields.io/badge/CEF-external-555555?style=flat-square)
![Node.js](https://img.shields.io/badge/Node.js-22%2B-3c873a?style=flat-square)

**Native browser core for macOS. Built with C++20, CEF, Rust, and SolidJS.**

Fubuki Browser Alpha は、Chromium Embedded Framework を直接扱う macOS 向けブラウザ基盤です。Electron / Tauri / WKWebView には依存せず、CEF を描画・Web 実行基盤として使い、タブ、ウィンドウ、設定、セッションなどのブラウザ状態は Rust 製の FrostEngine に集約します。

`Alpha` はプロダクト名・コードネームであり、リリース段階としての「アルファ版」を意味しません。現在の実装は MVP スコープで開発中のため、プロダクション用途のブラウザとしてはまだ扱いません。

## What is this? / これは何か

Fubuki は、単なる CEF ラッパーではなく、UI、エンジン、ネイティブホストを分離したブラウザ基盤です。

- **Native-first** — macOS ネイティブホストが NSWindow と CEF Browser を管理します。
- **Engine-owned state** — タブ、ウィンドウ、設定、セッションは FrostEngine が所有します。
- **Protocol boundary** — UI と Host は Frost Protocol / HostCommand / HostEvent を通して接続します。
- **No heavy wrapper** — Electron、Tauri、WKWebView、WebView2 は使用しません。

```text
UI renders. Engine decides. Host executes.
```

## Purpose / 作成目的

Fubuki Browser Alpha の目的は、既存ブラウザの見た目だけを再現することではありません。ブラウザを「UI 付きアプリ」ではなく、明確な状態管理・コマンド境界・ホスト実行層を持つ制御可能な基盤として設計することです。

具体的には、次のような目的で作成されています。

- macOS 上で CEF を直接扱う、軽量で理解しやすいネイティブブラウザ基盤を作る。
- タブ、ウィンドウ、セッション、設定などの論理状態を Rust 側に集約し、UI や C++ ホストに状態が散らばらない構造にする。
- UI、Engine、Host の境界を Protocol と HostCommand / HostEvent で固定し、後から機能追加しても責務が崩れにくい構成にする。
- 将来的な外部操作、自動化、デバッグ、検証用途に向けて、ブラウザ内部を安全に制御できる command boundary を用意する。
- Electron や汎用 WebView ラッパーではなく、ブラウザそのものの構造を小さく実装して検証できる土台にする。

Fubuki は、現時点では Chrome や Safari を置き換える日常利用ブラウザではありません。目的は、プロダクションブラウザの完成度よりも、ブラウザアーキテクチャ、状態管理、ネイティブホスト連携、自動操作境界を正しく分離した実験・開発基盤を作ることです。

## Differentiation / 何と差別化されているか

Fubuki は、以下のような既存カテゴリと意図的に違う位置付けにあります。

| Compared with | Difference |
|---|---|
| Chrome / Safari / Firefox / Arc | 完成済みの一般利用ブラウザを置き換えることより、ブラウザ内部の状態管理・ホスト連携・自動操作境界を検証しやすい構造を重視します。 |
| Electron / Tauri apps | Web アプリをデスクトップアプリ化するためのシェルではありません。CEF を直接扱い、ブラウザとしてのタブ、ウィンドウ、ナビゲーション、永続化を自前の Engine と Host で管理します。 |
| WKWebView wrappers | Apple の WebView に乗るのではなく、CEF / Chromium 系の実行基盤を使います。macOS ネイティブアプリでありつつ、Chromium ベースのブラウザ制御を前提にしています。 |
| CEF sample apps | CEF を表示するだけではなく、Rust 製 FrostEngine、SQLite Store、Frost Protocol、FFI、HostCommand / HostEvent によって、状態と副作用の境界を明確に分けます。 |
| Playwright / Selenium style automation | 既存ブラウザを外側から操作するだけではなく、ブラウザ内部に command boundary を設け、能力チェックや監査イベントを通した制御を前提にします。 |
| Minimal browser demos | 最小表示デモではなく、セッション、履歴、ブックマーク、ダウンロード、設定、内部ページ、テスト、CI まで含めた拡張可能な基盤を目指します。 |

要するに、Fubuki は「完成済みブラウザの代替」ではなく、**ブラウザを自分たちで制御・拡張・検証するための macOS-first なブラウザコア**です。

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
| UI | `ui/` | SolidJS 製のブラウザ UI。`fubuki://app/` 上で動作します。 |
| Protocol | `crates/frost-protocol/` | UI、Engine、Host 間の Request / Response / Event 型定義。 |
| Engine | `crates/frost-core/` | タブ、ウィンドウ、設定、セッションなどの論理状態を管理します。 |
| Store | `crates/frost-store/` | SQLite による履歴、ブックマーク、ダウンロード、設定、セッション永続化。 |
| FFI | `crates/frost-ffi/` | C++ ホストから Rust エンジンを呼び出すための FFI 境界。 |
| Host | `native/macos-cef-host/`, `native/src/` | NSWindow、CEF Browser、`fubuki://` scheme、HostCommand 実行を担当します。 |

詳しい設計は [docs/architecture.md](docs/architecture.md) を参照してください。

## Requirements / 必要環境

- macOS 12 以降
- Xcode Command Line Tools
- CMake 3.21 以降
- Rust stable toolchain with `clippy` and `rustfmt`
- Node.js 22 以降
- pnpm 11.x
- LLVM via Homebrew, cppcheck, python3, curl, tar

Apple Silicon を主なターゲットにしています。Intel Mac でも CEF の `macosx64` ビルドを使う構成はありますが、互換性は利用する CEF ビルドに依存します。

## Quick Start / クイックスタート

```bash
git clone https://github.com/TeamFubuki/fubuki-browser-alpha.git
cd fubuki-browser-alpha
make bootstrap
make build
make run
```

`make bootstrap` は CEF の取得、UI 依存関係のインストール、ネイティブビルド設定をまとめて実行します。CEF バイナリはリポジトリには含めません。

手動で CEF を指定する場合は、CMake 設定時に `CEF_ROOT` を渡してください。

```bash
CEF_ROOT=/path/to/cef_binary make configure
```

## Common Commands / よく使うコマンド

```bash
make cef          # CEF を取得または更新
make build        # UI / Rust / native をまとめてビルド
make run          # アプリをビルドして起動
make test         # Rust / UI / native のテストを実行
make lint-all     # UI / Rust / C++ のリントを実行
make format-all   # UI / Rust / C++ のフォーマットを実行
make audit        # cargo-audit を実行
make audit-deny   # cargo-deny を実行
```

## Current Scope / 現在の実装範囲

現在の実装には、macOS CEF ホスト、SolidJS UI、Frost Protocol、Rust 側のブラウザ状態管理、SQLite 永続化、通常 / プライベートウィンドウ、内部ページ、外部コマンド境界、ローカルテストと CI ワークフローが含まれます。

まだ含まれていないものとして、コード署名、ノータリゼーション、アップデート配信、完成度の高いインポート / エクスポート、Chrome 拡張機能互換、ブラウザ同期などがあります。詳しくは [docs/known-limitations.md](docs/known-limitations.md) を参照してください。

## Documentation / ドキュメント

- [Architecture](docs/architecture.md)
- [FrostEngine Plan](docs/frost-engine-plan.md)
- [Bridge API](docs/bridge-api.md)
- [Commands](docs/commands.md)
- [Events](docs/events.md)
- [Internal Pages](docs/internal-pages.md)
- [Known Limitations](docs/known-limitations.md)

## Contributing / 貢献

Contributions are welcome, but architecture boundaries matter.

プルリクエストを作成する前に [CONTRIBUTING.md](CONTRIBUTING.md) を読んでください。新機能、バグ修正、ドキュメント改善、設計上の指摘はいずれも歓迎します。ただし、UI、Engine、Host の責務分離を崩す変更は慎重に扱います。

## License / ライセンス

[MIT License](LICENSE) — Copyright (c) 2026 TeamFubuki
