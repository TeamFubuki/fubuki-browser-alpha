# FrostEngine 実装計画

> この文書の Phase 0–5 は移行時の履歴です。現在の責務分担と通信フローは [docs/architecture.md](architecture.md) を正とします。

## 1. コンポーネントバージョニング

Fubuki Browser Alpha は 2 つのコンポーネントで構成され、それぞれ独立してバージョニングする。

| コンポーネント | 場所 | 職責 |
|---|---|---|
| **Fubuki Browser UI** | `ui/` | SolidJS フロントエンド、Frost Protocol クライアント |
| **FrostEngine** | `crates/` | Rust ベースのブラウザ状態管理コア |

- UI のバージョンは `ui/package.json` の `version` で管理
- FrostEngine のバージョンは `crates/frost-core/Cargo.toml` の `version` で管理
- プロトコル互換性は `frost-protocol` の `protocol_version` で追跡

---

## 2. 現在のアーキテクチャ

```
Fubuki Browser UI (SolidJS)
    ↓ Frost Protocol
FrostEngine Core (Rust, logical state owner)
    ↓ HostCommand / HostEvent
Native Host / CEF (C++, side effects and CEF lifecycle)
```

### Request と HostEvent の流れ

- UI は `fubuki://app/` から Frost Protocol の request を送ります。
- FrostEngine が論理状態とポリシーを更新し、必要な CEF / OS 副作用を `HostCommand` として要求します。
- Native Host は CEF または macOS の副作用を実行し、その結果を `HostEvent` として FrostEngine に返します。
- FrostEngine が差分 event を UI へ配信します。起動時の `app.snapshot` 以外に逆向きの snapshot 同期は行いません。

Rust Core は CEF / macOS API に依存しません。C++ 側の `FrostBridge` と FFI が呼び出し境界を担当し、CEF thread affinity と browser lifecycle は Native Host に閉じます。

---

## 3. 移行履歴（Historical migration plan）

以下は移行を段階的に進めた時点の計画と実装記録です。新しい機能や現状の設計を追加するときは、上記の現在アーキテクチャと [docs/architecture.md](architecture.md) を参照してください。

### Phase 0: workspace + Protocol 型（1-2 日） - implemented

Rust workspace と Protocol の型定義だけ作る。C++ とはまだ接続しない。

| タスク | 内容 |
|---|---|
| `Cargo.toml` 作成 | workspace メンバー定義 |
| `frost-protocol` 作成 | `Request`, `Response`, `Event`, `TabState`, `WindowState` 型 |
| `frost-core` 作成 | `FrostCore::process()` の最小実装（タブ操作のみ） |
| テスト | `cargo test` で型のシリアライズ検証 |

**完了条件**: `cargo test` が通る。

Current implementation:

- `Cargo.toml` workspace
- `crates/frost-protocol`
- `crates/frost-engine-api`
- `crates/frost-core`
- `crates/frost-store`
- UI bridge normalization for `app.snapshot`
- native bridge aliases for Frost Protocol v0 method names

### Phase 1: Core + Store 実装（3-5 日） - implemented

ブラウザ状態を Rust で管理する。まだ C++ とは接続しない。

| タスク | 内容 |
|---|---|
| `TabService` | タブ作成、削除、アクティブ切替 |
| `SettingsService` | 設定読み書き |
| `frost-store` | SQLite で設定を永続化 |
| `FrostCore::run()` | チャネルからリクエストを受信し処理するループ |

**完了条件**: Rust 単体でタブ操作 + 設定保存が動く。

Current implementation:

- `TabService`, `WindowService`, `SettingsService`
- bookmark/history/download repository traits and SQLite implementation
- `BrowserCore::run()` channel loop
- Rust unit tests for core protocol handling and persistence

### Phase 2: C++ Host 接続（3-5 日） - implemented

C++ Host から Rust Core を呼び出す。既存の `NativeBridge` を置き換える。

| タスク | 内容 |
|---|---|
| `FrostBridge` 作成 | C++ で channel のラッパーを作成 |
| `NativeBridge` 書き換え | メソッドマップ削除、JSON → channel → Rust に変更 |
| 起動時に Rust Core スレッドを起動 | `std::thread` で `FrostCore::run()` を起動 |

**完了条件**: UI から `tabs.create` などが動作する。

Current implementation:

- `crates/frost-ffi` static library exposes FrostEngine C ABI
- `native/src/bridge/FrostBridge.*` owns the Rust Core thread bridge
- CMake builds and links `frost-ffi` into the native host
- `NativeBridge` delegates protocol state reads to FrostEngine and unwraps `ProtocolResponse.result` for UI compatibility
- Host-backed operations are issued by FrostEngine as `HostCommand`; the Native Host returns `HostEvent` and does not synchronize snapshots in reverse

### Phase 3: イベント接続（2-3 日） - implemented

CEF callback → Rust Core へのイベント通知を接続する。

| タスク | 内容 |
|---|---|
| `FubukiClient` の callback に channel 送信を追加 | `OnTitleChange` 等で `tab.updated` イベントを送信 |
| UI 側で差分イベントを受け取る | `browserStore.ts` を差分更新対応に書き換え |

**完了条件**: タブのタイトル変更などが UI に即座に反映される。

Current implementation:

- native `EventBus` emits Frost differential tab events (`tab.created`, `tab.updated`, `tab.closed`, `tab.activated`)
- native CEF callback events enter FrostEngine through `HostEvent`, keeping title, URL, loading, navigation, permission, and window state current
- UI store applies Frost tab events incrementally
- the UI requests `app.snapshot` at startup and otherwise consumes differential events

### Phase 4: 残りの API 移行（2-3 日） - implemented

ブックマーク、履歴、ダウンロード等の API を移行する。

| タスク | 内容 |
|---|---|
| `frost-protocol` にブックマーク/履歴/ダウンロード型を追加 | Request/Response/Event に追加 |
| `frost-core` に対応サービスを追加 | BookmarkService, HistoryService, DownloadService |
| `frost-store` に永続化を追加 | 各 Repository の実装 |
| C++ Host の callback 接続 | 対応する CEF callback を channel に接続 |

Current implementation:

- `frost-protocol` includes bookmark/history/download requests and responses
- `frost-store` persists bookmarks, history, and downloads
- `frost-core` processes list/save/remove/clear requests for those domains
- native bridge exposes matching `*.list` compatibility methods

### Phase 5: テスト（1-2 日） - implemented

| テスト | 内容 |
|---|---|
| Rust unit test | 各サービスのロジック検証 |
| Rust integration test | チャネル経由のリクエスト処理検証 |
| C++ との接続テスト | 実際の CEF での動作検証 |

Current verification:

- `cargo test --workspace`
- `cd ui && pnpm exec tsc --noEmit`
- `cd ui && pnpm test`
- `make test-native`
- `make ui`
- `make native`

### Current implementation: Permission Broker

- Frost Protocol defines canonical permission types, permission records, request/resolution messages, and prompt events.
- FrostEngine normalizes origins, applies saved decisions, manages pending prompts, and keeps Private Window decisions scoped to that window.
- `frost-store` persists normal-window decisions; the UI prompt and Settings page manage those decisions through Frost Protocol.
- The Native Host maps CEF permission callbacks to `HostEvent` and resolves the pending callback through `HostCommand`.
- Automated Rust, UI, and Native checks cover policy, bridge validation, and schema behavior. End-to-end checks on a running macOS CEF app remain a separate manual verification step.

---

## 4. ファイル構成

```
crates/
  frost-protocol/
    src/
      lib.rs              # 型定義を re-export
      request.rs          # Request enum
      response.rs         # Response enum
      event.rs            # Event enum
      state.rs            # TabState, WindowState, AppState
  frost-core/
    src/
      lib.rs              # FrostCore, run()
      tab_service.rs      # タブ操作ロジック
      settings_service.rs # 設定操作ロジック
  frost-store/
    src/
      lib.rs              # Store trait
      sqlite.rs           # SQLite 実装
native/
  src/
    bridge/
      FrostBridge.h/.cc   # channel ラッパー + NativeBridge 置き換え
    browser/
      BrowserWindow.mm    # 軽量化（状態管理を削除）
      BrowserAppController.cc  # 変更不大
```

---

## 5. 注意事項

- **段階的移行**: 各 Phase で動作する状態を保つ
- **Rust Core は CEF を知らない**: JSON の入出力だけ。CEF に依存するコードは C++ Host 側に閉じる
- **フルイベントソーシングはしない**: 今の計画には未含む
- **プラグイン機構はしない**: 拡張性は Protocol / Core / Host の分離で確保
