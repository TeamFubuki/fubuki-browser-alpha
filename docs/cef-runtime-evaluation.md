# CEF Runtime Style Evaluation (Issue #58)

このドキュメントは `TeamFubuki/fubuki-browser-alpha#58` の検証作業を追跡するための最小成果物です。  
現時点では **全面移行は未決定** で、Alloy style 継続/Chrome style 併用/Chrome style 主軸化を比較します。

## CEFバージョン固定方針

- `scripts/fetch_cef.sh` は `CEF_VERSION` を指定すると、その `cef_version` の配布物だけを取得します。
- 例:

```bash
CEF_VERSION=<cef_version> make cef
```

## Phase 0: 現行Alloyの権限処理修正

- [x] `FubukiClient::OnShowPermissionPrompt()` の無条件拒否を廃止
- [x] Pointer Lock/Keyboard Lockを個別に判定
- [x] テスト用許可フラグを導入
  - `FUBUKI_ALLOW_POINTER_LOCK=1`
  - `FUBUKI_ALLOW_KEYBOARD_LOCK=1`
- [ ] 永続的なサイト権限保存は未実装（今後 Frost Store 設計へ統合）

## Alloy style / Chrome style 比較（検証トラッキング）

| 項目 | Alloy style (現行) | Chrome style (期待) | 検証状態 |
|---|---|---|---|
| Pointer Lock | 実装側判定が必要 | 標準許可UI利用の余地 | Phase 0完了（Alloyのみ） |
| Keyboard Lock | 実装側判定が必要 | 標準許可UI利用の余地 | Phase 0完了（Alloyのみ） |
| Gamepad API | 要検証 | 要検証 | 未着手 |
| WebHID | 要検証 | 要検証 | 未着手 |
| WebMIDI | 要検証 | 要検証 | 未着手 |
| WebUSB | 要検証 | 要検証 | 未着手 |
| カメラ/マイク権限 | 要検証 | 標準許可UI利用の余地 | 未着手 |
| 通知 | 要検証 | 標準許可UI利用の余地 | 未着手 |
| File System Access API | 要検証 | 要検証 | 未着手 |
| PWA導線 | ほぼ未実装 | 標準機能活用の余地 | 未着手 |
| 拡張機能 | ほぼ未実装 | 活用可能性あり | 未着手 |
| DevTools統合 | 利用可（限定） | 強化余地あり | 未着手 |
| 権限UI/サイト設定 | 独自実装が必要 | Chrome UI活用の余地 | 未着手 |

## 次の実装対象（PoC）

1. Chrome style最小PoCウィンドウ起動（macOS Apple Silicon）
2. HostCommand/HostEvent境界での Runtime Abstraction プロトタイプ
3. RequestContext/Cookie/履歴/ダウンロード共有可否の比較
