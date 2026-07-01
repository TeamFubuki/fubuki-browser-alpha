# Known Limitations

- CEF is configured externally with `CEF_ROOT`; binaries are not vendored.
- Codesign and notarization are intentionally out of scope for the MVP.
- UI and content view layout still uses native child views with compact web chrome; there is no fully native toolbar.
- No password manager, sync, extension compatibility, updater, or import/export polish is included.
- Session restore is implemented for normal windows and tabs, but it restores last URLs rather than full in-page navigation stacks.
- Private windows use an off-the-record CEF request context and skip normal app data writes, but downloaded files still exist on disk if the user downloads them.
- Cache/site-data clearing is conservative and depends on CEF request-context support.
- Bookmark folders and browser-compatible import/export need more complete UX and persistence work.
- macOS Apple Silicon is the primary target; Intel compatibility depends on the CEF build used.
