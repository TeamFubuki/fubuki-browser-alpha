# Known Limitations

- **CEF version lock:** CEF and Chromium versions are selected externally through `CEF_ROOT`; a repository lock file with archive checksum and architecture validation is not implemented.
- **Renderer recovery:** a crashed CEF renderer is not yet represented as a recoverable tab state; targeted renderer recreation and reload recovery are not implemented.
- **Permission Broker:** the Alloy-style permission broker, prompt, origin-scoped settings, and private-window policy are implemented. End-to-end Allow / Block / Not now behavior still needs manual validation with a running macOS CEF app.
- **Chrome style:** no Chrome-style PoC or Alloy-versus-Chrome comparison has been completed. No runtime style has been selected for future development.
- **Distribution:** code signing, notarization, and update delivery are not implemented.
- **Browser features:** password management, sync, extension compatibility, and polished import / export are not included.
- **UI:** the interface uses native child views and compact web chrome; it does not have a fully native toolbar.
- **Session restore:** normal windows and tabs restore their last URLs, not full in-page navigation stacks.
- **Private Window:** it uses an off-the-record CEF request context and skips normal app-data writes. Files downloaded by the user still remain on disk.
- **Site data:** cache and site-data clearing is conservative and depends on CEF request-context support.
- **Bookmarks:** folder management and browser-compatible import / export need more complete UX and persistence work.
- **CEF distribution:** CEF is configured externally with `CEF_ROOT`; binaries are not vendored.
- **Platform:** Apple Silicon is the primary target; Intel support depends on the selected CEF build.
