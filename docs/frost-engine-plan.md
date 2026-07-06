# FrostEngine 実装計画

## 1. コンポーネントバージョニング

Fubuki Browser Alpha は 2 つのコンポーネントで構成され、それぞれ独立してバージョニングする。

| コンポーネント | 場所 | 職責 |
|---|---|---|
| **Fubuki Browser UI** | `ui/` | SolidJS フロントエンド、Frost Protocol クライアント |
| **FrostEngine** | `crates/` | Rust ベースのブラウザ状態管理コア |

- UI のバージョンは `ui/package.json` の `version` で管理
- FrostEngine のバージョンは `crates/frost-core/Cargo.toml` の `version` で管理
- プロトコル互換性は `frost-protocol` の `protocol_version` で追跡
- 製品としてのリリースバージョンはタグで管理（例: `v1.0.0`）

---

## 2. 現状の問題

### 現行アーキテクチャ

```
SolidJS UI → NativeBridge → BrowserWindow → TabManager / BrowserDataStore → CEF
```

- `BrowserWindow` がタブ、データストア、コマンド、ブリッジ、NSWindow をすべて所有
- `NativeBridge` が巨大なメソッドマップを持ち、直接 `BrowserWindow` の各処理に呼び出す
- `BrowserDataStore` が SQLite 操作を単一クラスで完結（履歴、ブックマーク、ダウンロード、設定、権限、ログ）
- UI が全イベントで `app.getState` 全量リフレッシュ（差分同期なし）
- ウィンドウ、セッション、設定の永続化が `BrowserAppController` と `BrowserWindow` に散在

### 将来の問題

タブ、履歴、ブックマーク、設定、ダウンロード、セッション、AI 機能、外部 API（MCP）を追加するたびに、`BrowserWindow` と `NativeBridge` にコードを追加し続け、密結合がさらに深まる。

---

## 3. 目標アーキテクチャ

```
Fubuki Browser UI (SolidJS)
    ↓ Frost Protocol (JSON-RPC over CEF message router)
FrostEngine Core (Rust)
    ↓ EngineAdapter trait
CEF/macOS Host (C++)
```

### 原則

- **状態の所有者は FrostEngine Core のみ**。CEF/macOS Host は表示と入力のみ担当
- **UI は CEF を直接操作しない**。Frost Protocol 経由で操作する
- **差分イベントで同期**。起動時だけ `app.snapshot` で全状態取得、以後はイベントで差分更新
- **EngineAdapter trait で CEF と未来のホストを切り替える**
- **FrostEngine Core は C++ / CEF に依存しない**

---

## 4. モジュール構成

```
crates/
  frost-protocol/     # プロトコル定義（request, response, event, state, command schema）
  frost-core/         # BrowserCore, TabService, WindowService, SessionService, SettingsService
  frost-store/        # SQLite永続化、repository trait、migration
  frost-engine-api/   # EngineAdapter, PageAdapter, WindowHost の trait 定義
native/
  macos-cef-host/     # CEF / macOS 固有処理（表示、入力、lifecycle、NSWindow）
ui/
  src/
    bridge/
      fubuki.ts       # 既存のブリッジを Frost Protocol クライアントに置き換え
```

---

## 5. Phase 実装計画

### Phase 0: 基盤準備（ドキュメント + workspace）

**目的**: 方針文書化、Rust workspace 設定、Frost Protocol v0 定義

| タスク | 詳細 |
|---|---|
| `docs/architecture.md` 更新 | FrostEngine 中心のアーキテクチャ記述に書き換え |
| `Cargo.toml` 作成 | Rust workspace 作成、crate メンバー定義 |
| `crates/frost-protocol` 作成 | 最小限の request/response/event schema |
| `crates/frost-core` 作成 | `BrowserCore` の骨格（タブ操作のみ） |
| `crates/frost-store` 作成 | SQLite repository trait + 実装（設定のみ） |
| `crates/frost-engine-api` 作成 | `EngineAdapter` trait 定義 |

**成果物**:
- `docs/architecture.md`（FrostEngine 版）
- `Cargo.toml`（workspace）
- `crates/frost-protocol/src/lib.rs`（schema 定義）
- `crates/frost-core/src/lib.rs`（BrowserCore + TabService）
- `crates/frost-store/src/lib.rs`（SettingsRepository）
- `crates/frost-engine-api/src/lib.rs`（EngineAdapter trait）

### Phase 1: Frost Protocol v0 定義

**目的**: UI ↔ Native 間の通信プロトコルを Rust 型で定義

```rust
// frost-protocol/src/request.rs
pub enum Request {
    AppSnapshot,
    TabsList,
    TabsCreate { url: Option<String>, active: bool },
    TabsActivate { tab_id: String },
    TabsClose { tab_id: String },
    TabsNavigate { tab_id: String, input: String },
    TabsReload { tab_id: String },
    TabsGoBack { tab_id: String },
    TabsGoForward { tab_id: String },
    WindowsList,
    WindowsCreate,
    WindowsClose,
    SettingsGet { key: String },
    SettingsSet { key: String, value: String },
}

// frost-protocol/src/response.rs
pub enum Response {
    AppSnapshot(AppState),
    TabsList(Vec<TabState>),
    Ok(bool),
    Error(String),
}

// frost-protocol/src/event.rs
pub enum Event {
    TabCreated(TabState),
    TabUpdated(TabPatch),
    TabClosed { tab_id: String },
    TabActivated { tab_id: String },
    WindowCreated(WindowState),
    WindowClosed { window_id: String },
    SettingChanged { key: String, value: String },
}
```

### Phase 2: FrostEngine Core 実装

**目的**: ブラウザ状態を Rust で所有、操作ロジックを実装

| サービス | 職責 |
|---|---|
| `TabService` | タブ作成、削除、アクティブ切替、順序操作 |
| `WindowService` | ウィンドウ作成、削除、フォーカス管理 |
| `SettingsService` | 設定読み書き、デフォルト値管理 |
| `SessionService` | セッションスナップショット、復元 |
| `BrowserCore` | 上記サービスの統合、`process(Request) -> Response` |

```rust
// frost-core/src/browser_core.rs
pub struct BrowserCore {
    tabs: TabService,
    windows: WindowService,
    settings: SettingsService,
    session: SessionService,
    event_tx: broadcast::Sender<Event>,
}

impl BrowserCore {
    pub fn process(&self, request: Request) -> Response { ... }
    pub fn subscribe(&self) -> broadcast::Receiver<Event> { ... }
}
```

### Phase 3: EngineAdapter trait 定義

**目的**: CEF/macOS Host と FrostEngine の境界を定義

```rust
// frost-engine-api/src/lib.rs
pub trait EngineAdapter: Send + Sync {
    fn open_page(&self, window_id: &str, tab_id: &str, url: &str) -> Result<()>;
    fn navigate_page(&self, window_id: &str, tab_id: &str, url: &str) -> Result<()>;
    fn reload_page(&self, window_id: &str, tab_id: &str) -> Result<()>;
    fn close_page(&self, window_id: &str, tab_id: &str) -> Result<()>;
    fn go_back(&self, window_id: &str, tab_id: &str) -> Result<()>;
    fn go_forward(&self, window_id: &str, tab_id: &str) -> Result<()>;
    fn create_window(&self, window_id: &str, private: bool) -> Result<()>;
    fn close_window(&self, window_id: &str) -> Result<()>;
}

pub trait PageAdapter: Send + Sync {
    fn on_title_changed(&self, tab_id: &str, title: &str);
    fn on_url_changed(&self, tab_id: &str, url: &str);
    fn on_loading_state(&self, tab_id: &str, loading: bool, can_back: bool, can_forward: bool);
    fn on_favicon(&self, tab_id: &str, favicon_url: &str);
}
```

### Phase 4: CEF/macOS Host の FrostEngine 対応

**目的**: 既存 C++ コードを FrostEngine のクライアントに変換

| 変更 | 詳細 |
|---|---|
| `NativeBridge` 簡素化 | メソッドマップ削除、Frost Protocol の JSON パース + `BrowserCore::process()` 呼び出しに置き換え |
| `BrowserWindow` 軽量化 | タブ状態管理を削除、CEF 表示と入力のみに縮小 |
| `BrowserDataStore` 廃止 | `frost-store` に置き換え |
| `BrowserAppController` 内部化 | ウィンドウ管理は `WindowService` に移動 |
| `TabManager` 廃止 | `TabService` に置き換え |

### Phase 5: UI の Frost Protocol 対応

**目的**: UI のブリッジ層を Frost Protocol クライアントに置き換え

| 変更 | 詳細 |
|---|---|
| `bridge/fubuki.ts` 内訳 | Frost Protocol の型定義をインポート、invoke を `Request` → JSON → CEF query に変換 |
| `stores/browserStore.ts` | `app.snapshot` で初期化、差分イベントで選択的に更新 |
| 状態同期の改善 | 全量リフレッシュを廃止、イベントペイロードで直接 store を更新 |

### Phase 6: テスト・統合

| テスト | 詳細 |
|---|---|
| `frost-protocol` unit test | schema のシリアライズ/デシリアライズ |
| `frost-core` unit test | TabService, SettingsService のロジック検証 |
| `frost-store` integration test | SQLite 操作の検証 |
| `frost-engine-api` mock test | EngineAdapter のモック実装での検証 |
| UI unit test | 差分イベントハンドリングの検証 |
| E2E test | 起動 → タブ操作 → 設定変更 → 終了の一連の流れ |

---

## 6. Frost Protocol v0 API 一覧

### Request → Response

| Request | Response | 備考 |
|---|---|---|
| `app.snapshot` | `AppState` | 起動時・再同期時に使用 |
| `tabs.list` | `Vec<TabState>` | 全タブ一覧 |
| `tabs.create { url?, active }` | `Ok(bool)` | タブ作成 |
| `tabs.activate { tab_id }` | `Ok(bool)` | タブ切替 |
| `tabs.close { tab_id }` | `Ok(bool)` | タブ削除 |
| `tabs.navigate { tab_id, input }` | `Ok(bool)` | ナビゲーション |
| `tabs.reload { tab_id }` | `Ok(bool)` | リロード |
| `tabs.goBack { tab_id }` | `Ok(bool)` | 戻る |
| `tabs.goForward { tab_id }` | `Ok(bool)` | 進む |
| `windows.list` | `Vec<WindowState>` | ウィンドウ一覧 |
| `windows.create` | `Ok(bool)` | ウィンドウ作成 |
| `windows.close` | `Ok(bool)` | アクティブウィンドウ削除 |
| `settings.get { key }` | `Ok(String)` | 設定読み込み |
| `settings.set { key, value }` | `Ok(bool)` | 設定保存 |

### Events（差分）

| Event | Payload | 備考 |
|---|---|---|
| `tab.created` | `TabState` | 新規タブ |
| `tab.updated` | `TabPatch` | タブ属性変更（title, url, loading, etc.） |
| `tab.closed` | `{ tab_id }` | タブ削除 |
| `tab.activated` | `{ tab_id }` | アクティブ切替 |
| `window.created` | `WindowState` | ウィンドウ作成 |
| `window.closed` | `{ window_id }` | ウィンドウ削除 |
| `setting.changed` | `{ key, value }` | 設定変更 |

---

## 7. 移行手順

### Step 1: Phase 0-1 を実装（Rust crate 作成）
1. `Cargo.toml` と各 crate のスカフォールド作成
2. `frost-protocol` に Request/Response/Event 型を定義
3. `frost-core` に BrowserCore の最小実装（タブ操作のみ）
4. `frost-store` に SettingsRepository を実装
5. `frost-engine-api` に EngineAdapter trait を定義

### Step 2: Phase 2 を実装（Core ロジック）
1. TabService の完全実装
2. SettingsService の完全実装
3. BrowserCore::process() の実装
4. 単体テスト作成

### Step 3: Phase 3-4 を実装（CEF Host 対応）
1. CefHost（EngineAdapter の C++ 実装）を作成
2. NativeBridge を Frost Protocol の薄い入口に書き換え
3. BrowserWindow から状態管理を削除
4. BrowserDataStore を frost-store に置き換え

### Step 4: Phase 5 を実装（UI 対応）
1. `bridge/fubuki.ts` を Frost Protocol クライアントに書き換え
2. `stores/browserStore.ts` を差分イベント対応に書き換え
3. 既存テストを更新

### Step 5: Phase 6 を実装（テスト・統合）
1. 全 crate のテスト作成
2. E2E テスト作成
3. パフォーマンス検証

---

## 8. ファイル構成（最終目標）

```
/
├── Cargo.toml                          # Rust workspace
├── crates/
│   ├── frost-protocol/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── request.rs
│   │       ├── response.rs
│   │       ├── event.rs
│   │       ├── state.rs               # TabState, WindowState, AppState
│   │       └── command.rs             # CommandSchema
│   ├── frost-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── browser_core.rs
│   │       ├── tab_service.rs
│   │       ├── window_service.rs
│   │       ├── settings_service.rs
│   │       └── session_service.rs
│   ├── frost-store/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── repository.rs          # trait 定義
│   │       ├── sqlite_store.rs        # SQLite 実装
│   │       └── migration.rs
│   └── frost-engine-api/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── engine_adapter.rs      # trait 定義
│           ├── page_adapter.rs        # trait 定義
│           └── window_host.rs         # trait 定義
├── native/
│   ├── macos-cef-host/
│   │   ├── CMakeLists.txt
│   │   └── src/
│   │       ├── cef_host.h / .cc
│   │       ├── cef_page_adapter.h / .cc
│   │       ├── scheme_handler.h / .cc
│   │       ├── window_manager.h / .cc
│   │       └── main.cc
│   ├── CMakeLists.txt
│   └── ...
├── ui/
│   ├── package.json                    # Fubuki Browser UI バージョン
│   └── src/
│       ├── bridge/
│       │   ├── fubuki.ts
│       │   └── protocol.ts
│       ├── stores/
│       │   └── browserStore.ts
│       └── ...
└── docs/
    ├── architecture.md
    └── frost-engine-plan.md
```

---

## 9. 注意事項

- **段階的移行**: 既存機能を一度に書き換えない。Phase 0-1 → 2 → 3-4 → 5 の順で段階的に移行
- **後方互換**: 移行中は旧 Bridge API と Frost Protocol が併存できるようにする
- **フルイベントソーシングはしない**: Phase 0-6 では未実装。将来的に必要な場合のみ追加
- **プラグイン機構はしない**: 拡張性は Protocol / Core / Adapter の分離で確保
- **UI の公式提供はしない**: FrostEngine API に従えば別 UI も動く構造にするが、公式 UI は今の SolidJS UI を継続
