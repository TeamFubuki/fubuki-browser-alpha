# Internal pages

The six internal pages are a standalone SolidJS application in `internal-pages/`, with their own
package, lockfile, TypeScript configuration, styles, tests, and Vite build. They do not import the
browser shell (`ui/`) or its stores and bridge client.

| URL | Component |
| --- | --- |
| `fubuki://newtab/` | `pages/NewTab.tsx` |
| `fubuki://history/` | `pages/Records.tsx` |
| `fubuki://bookmarks/` | `pages/Records.tsx` |
| `fubuki://downloads/` | `pages/Records.tsx` |
| `fubuki://settings/` | `pages/Settings.tsx` |
| `fubuki://debug/` | `pages/Debug.tsx` |

## Build and develop

```sh
make internal-pages
cd internal-pages
pnpm dev
pnpm test
```

For a local preview select a page with `?page=settings` (or another hostname from the table).
The development server serves `dev/preview.json` as `/data.json`; edit this fixture to preview
records and settings. This middleware is development-only and is not included in the production
build. The development server cannot access the native profile or execute browser actions. For integration testing, build the app and open the existing `fubuki://` URLs in Fubuki.
`make build`, `make run`, and `make bootstrap` also build internal pages. `make test` includes their
tests, and CI runs their lint, formatting, typecheck, build, and tests independently.

## Host contract

`native/src/cef/FubukiSchemeHandler.cc` serves `internal-pages/dist/index.html` on every internal
page origin, plus the built `assets/` files and logo. Vite uses relative asset URLs, so scripts and
styles load from the current page origin. Missing assets return 404 rather than an HTML fallback.
The handler does not generate page markup or styles.

Each page fetches its same-origin `/data.json`. This read-only JSON projection contains language,
appearance, an allowlist of settings, and the records needed by that page. The existing engine-owned
`frost-engine.sqlite3` read queries remain in the host during migration; the host does not create,
migrate, or write this database. Debug adds host diagnostics. JSON is encoded with CEF's serializer,
never interpolated into scripts or HTML. Solid renders stored text with escaping, and stored URLs
are checked before creating links. No CORS access is granted to other origins.

JSON responses use the existing bounded LRU cache with a two-second TTL; existing mutation paths
invalidate the same origin prefixes. Debug and unsuccessful database reads are not cached. Read
failures return 503 and show a retry control instead of masquerading as an empty list.

Settings and other actions submit native HTML **POST forms** to `fubuki://settings/set`, including
encoded parameters in both the form body and URL for CEF versions that omit custom-scheme POST
bodies. `FubukiClient` validates the trusted source and HTTP method before handing the action to
FrostEngine's existing service/store paths. Direct requests to the scheme action endpoint still
return 403. Search uses the existing `fubuki://newtab/search` navigation handler.

Only `fubuki://app/` receives the Frost Protocol bridge. Internal pages receive no `cefQuery`
privilege, and adding a new page must not widen that boundary. New stateful features belong in
FrostEngine Core with request/response types in `frost-protocol`.
