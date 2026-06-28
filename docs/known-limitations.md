# Known Limitations

- CEF is configured externally with `CEF_ROOT`; binaries are not vendored.
- Codesign and notarization are intentionally out of scope for the MVP.
- UI and content view layout is minimal and fixed to the MVP toolbar height.
- No password manager, sync, extension, updater, or session restore implementation is included.
- History, bookmarks, downloads, settings, and debug logs are stored as simple local JSON files for MVP readability.
- macOS Apple Silicon is the primary target; Intel compatibility depends on the CEF build used.
