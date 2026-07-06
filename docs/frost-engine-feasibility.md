# FrostEngine 実現可能性評価

## 評価結果サマリ

| 項目 | 判定 | 理由 |
|---|---|---|
| Rust ↔ C++ FFI | **実現可能** | C FFI で静的ライブラリ化、CMake からリンク |
| CEF message router 経由 | **実現可能** | 既存の `cefQuery` パスをそのまま活用 |
| パフォーマンス | **問題なし** | ブラウザ操作は CEF 呼び出しが支配的、FFI オーバーヘッドは無視できる |
| ビルド統合 | **実現可能** | Cargo build を CMake のカスタムコマンドで呼ぶ |
| セッション永続化 | **実現可能** | SQLite ファイルを Rust と C++ で共有、CEF cookie は別管理 |
| CEF スレッドモデル | **注意が必要** | CEF UI thread 制約あり、FFI 呼び出しのスレッド安全性が必要 |
| 既存コード移行 | **実現可能** | 段階的に移行可、旧 API と新 Protocol の併存期間あり |
| macOS 固有コード | **実現可能** | NSWindow/ObjC は C++ Host 側に残す |

---

## 1. Rust ↔ C++ FFI

### 方針

FrostEngine Core（`frost-core` + `frost-store`）を **C FFI 付き静的ライブラリ** としてビルドし、CMake の C++ バイナリにリンクする。

```
Cargo: frost-core → libfrost_core.a (C FFI: extern "C")
                                    ↓
CMake: C++ Host が link して呼び出す
```

### 実装方針

```rust
// frost-core/src/ffi.rs
#[no_mangle]
pub extern "C" fn frost_core_create(profile_path: *const c_char) -> *mut FrostCore {
    // ...
}

#[no_mangle]
pub extern "C" fn frost_process_request(
    core: *const FrostCore,
    request_json: *const c_char,
) -> *mut c_char {
    // JSON 文字列で受け渡し
}

#[no_mangle]
pub extern "C" fn frost_subscribe(core: *const FrostCore, callback: extern "C" fn(...)) {
    // イベントコールバック
}
```

### リスク

- **CefRefPtr の扱い**: C++ 側で CEF オブジェクトの生命周期を管理するため、Rust 側には raw pointer として渡す。Rust 側は CEF 型を直接持たない。
- **メモリ管理**: C FFI で渡す文字列は Rust 側で `CString` / `CStr` を使い、所有権を明確にする。
- **判定**: 問題なし。広く使われているパターン。

---

## 2. CEF Message Router 経由

### 現状

```
UI (fubuki.ts) → window.cefQuery() → CEF message router
  → NativeBridge::OnQuery() → methods_[method](params) → BrowserWindow
```

###移行後

```
UI (fubuki.ts) → window.cefQuery() → CEF message router
  → FrostProtocolHandler::OnQuery() → frost_process_request() (FFI)
  → BrowserCore::process() → Response → callback->Success()
```

### ポイント

- `cefQuery` の呼び出しパターンは変更不要
- `NativeBridge` のメソッドマップを削除し、JSON → `Request` パース + FFI 呼び出しに置き換えるだけ
- UI 側の `fubuki.invoke()` は変更不要（Protocol v0 は既存 API と互換）

### リスク

- なし。既存の CEF message router パスをそのまま活用可能。

---

## 3. パフォーマンス

### 分析

| 操作 | 現在のコスト | FrostEngine 後の追加コスト |
|---|---|---|
| `tabs.create` | CEF `BrowserHost::CreateBrowser()` | JSON シリアライズ + FFI 呼び出し（~μs） |
| `app.getState` | 全状態を CefDictionaryValue に変換 + JSON 化 | JSON → Request パース + Response シリアライズ（~μs） |
| イベント通知 | `EmitToUi()` → JavaScript 実行 | 変更なし（C++ Host が直接 EmitToUi） |

### 判定

- ブラウザ操作のボトルネックは CEF 呼び出し（数 ms〜数十 ms）
- FFI + JSON のオーバーヘッドは ~100μs 程度
- **問題なし**

---

## 4. ビルド統合

### 方針

```makefile
# Makefile
rust-core:
    cd crates && cargo build --release

native: rust-core
    @CEF_ROOT="$(CEF_ROOT)" NATIVE_BUILD_DIR="$(NATIVE_BUILD_DIR)" \
      BUILD_TYPE="$(BUILD_TYPE)" ./scripts/build_native.sh
```

### CMake 側

```cmake
# native/macos-cef-host/CMakeLists.txt
add_custom_command(
    OUTPUT "${RUST_LIB_PATH}/libfrost_core.a"
    COMMAND cargo build --release --target-dir "${CARGO_TARGET_DIR}"
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}/../../crates"
    COMMENT "Building FrostEngine Rust core"
)

add_library(frost_core STATIC IMPORTED)
set_target_properties(frost_core PROPERTIES
    IMPORTED_LOCATION "${RUST_LIB_PATH}/libfrost_core.a"
)

target_link_libraries(FubukiBrowserAlpha PRIVATE frost_core)
```

### リスク

- **クロスコンパイル**: macOS arm64/x86_64 の Cargo ターゲットと CMake のアーキテクチャを一致させる必要あり。`--target aarch64-apple-darwin` または `--target x86_64-apple-darwin` を明示。
- **判定**: 実現可能。既存の CMake + Makefile に追加するだけ。

---

## 5. セッション永続化

### 現状

- `BrowserDataStore` が `~/Library/Application Support/Fubuki Browser Alpha/` 配下に SQLite を使用
- CEF の cookie/session は CEF 自体が管理（`persist_session_cookies`）

### 移行後

- `frost-store`（Rust）が SQLite を管理
- CEF cookie は引き続き CEF 自体が管理（変更なし）
- SQLite ファイルパスは C++ Host → FFI → Rust Core に渡す

### リスク

- SQLite の同時アクセス：Rust Core と C++ Host が同時に SQLite を触る可能性は低い（Core が唯一のオーナー）。
- 既存 DB の移行：スキーマ互換性を保てば問題なし。
- **判定**: 問題なし。

---

## 6. CEF スレッドモデル（注意点）

### 制約

- CEF のコールバック（`OnTitleChange`, `OnLoadingStateChange` 等）は **CEF UI thread** で呼ばれる
- CEF UI thread はメインスレッド（macOS の main thread）
- FFI 呼び出しもこのスレッド上で行われる

### 対策

- Rust Core は `Send + Sync` として設計し、FFI 呼び出しは CEF UI thread から行う
- Rust Core 内部で `Mutex` や `channel` を使っても、CEF UI thread からの呼び出しは直列なので問題ない
- イベント通知は Rust Core → C++ Host のコールバック経由で、引き続き CEF UI thread 上で `EmitToUi()` を呼ぶ

### リスク

- Rust の `RefCell` は `Send` ではないため、FFI 呼び出しでは `Mutex` を使う
- **判定**: 注意が必要だが対策可能。

---

## 7. 既存コード移行の工数

### 段階別見積もり

| Phase | 工数（目安） | 内容 |
|---|---|---|
| Phase 0 | 1-2 日 | workspace 作成、crate スカフォールド、Protocol 型定義 |
| Phase 1 | 1-2 日 | Protocol v0 の型定義完了 |
| Phase 2 | 3-5 日 | BrowserCore + TabService + SettingsService 実装 |
| Phase 3 | 1-2 日 | EngineAdapter trait 定義 |
| Phase 4 | 5-8 日 | C++ Host 書き換え（最も大きな変更） |
| Phase 5 | 2-3 日 | UI の差分イベント対応 |
| Phase 6 | 2-3 日 | テスト作成 |
| **合計** | **15-25 日** | |

### 最も工数がかかる箇所

1. **Phase 4**: `NativeBridge` の書き換え + `BrowserWindow` からの状態管理剥がし。既存 C++ コードの 60-70% に影響。
2. **Phase 2**: Rust Core の完全実装。新規書籍としての工数。

### 最もリスクが高い箇所

1. **Phase 4 の途中経過**: 移行中は旧 API と新 Protocol が併存するため、整合性管理が複雑。
2. **CEF 回呼び出しの接続**: CEF callback → PageAdapter → Rust Core → EngineAdapter → CEF 操作 の循環を正しく実装する。

---

## 8. 代替案との比較

### 代替案 A: Rust を使わず、C++ のまま Core を分離

```
Pros: FFI が不要、ビルドが簡単
Cons: C++ の安全性・パッケージ管理の利点を失う。将来の AI/MCP 統合で Rust のエコシステムが欲しい
```

### 代替案 B: Rust を別プロセスとして実行（IPC）

```
Pros: プロセス分離で crash isolation
Cons: IPC オーバーヘッド、複雑さ増加、CEF UI thread 制約に引っかかる
```

### 代替案 C: 既存アーキテクチャのまま改善

```
Pros: 移行工数ゼロ
Cons: 密結合は解消されず、将来の拡張で崩壊リスク
```

### 判定

**Rust 静的ライブラリ + C FFI** が最適。安全性、パッケージ管理、パフォーマンスのバランスが良い。

---

## 9. 結論

### 実現可能判定: **○**

- 技術的制約はすべて対策可能
- 最大のリスクは **Phase 4 の移行工数** と **CEF スレッドモデルとの兼ね合い**
- 段階的に移行できるため、一度に書き換える必要がない
- 既存の CEF message router パスはそのまま活用可能

### 推奨する最初の一歩

Phase 0 を最小限に実装し、以下の検証を行う：

1. `frost-protocol` の型定義が C FFI 経由で正しくシリアライズ/デシリアライズされること
2. CMake から Cargo build → 静的ライブラリ → リンク が通ること
3. 最小限の `frost_process_request` が C++ から呼び出せること

この検証が通れば、後の Phase は工数の問題であり技術的リスクは低い。
