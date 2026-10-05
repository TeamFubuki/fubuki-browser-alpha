# Known Limitations

- **Renderer recovery:** crashed and unresponsive renderers are tracked per tab and can be recovered by reloading. Native macOS CEF behavior still needs manual validation under real renderer crashes and hangs.
- **Permission Broker:** the Alloy-style permission broker, prompt, origin-scoped settings, and private-window policy are implemented. End-to-end Allow / Block / Not now behavior still needs manual validation with a running macOS CEF app.
- **Chrome style:** no Chrome-style PoC or Alloy-versus-Chrome comparison has been completed. No runtime style has been selected for future development.
- **Distribution:** code signing, notarization, and update delivery are not implemented.
- **Browser features:** password management, sync, extension compatibility, and polished import / export are not included.
- **UI:** the interface uses native child views and compact web chrome; it does not have a fully native toolbar.
- **Session restore:** normal windows and tabs restore their last URLs, not full in-page navigation stacks.
- **Private Window:** it uses an off-the-record CEF request context and skips normal app-data writes. Files downloaded by the user still remain on disk.
- **Site data:** cache and site-data clearing is conservative and depends on CEF request-context support.
- **Bookmarks:** folder management and browser-compatible import / export need more complete UX and persistence work.
- **CEF distribution:** CEF / Chromium versions and per-architecture archive checksums are pinned in `cef.lock`; binaries are not vendored. CEF updates are proposed for review and are not auto-merged.
- **Platform:** Apple Silicon is the primary target; Intel support depends on the selected CEF build.
