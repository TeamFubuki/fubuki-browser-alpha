# Internal Pages

Internal pages are SolidJS components in `ui/src/pages/InternalPages.tsx`. The
native scheme handler keeps each `fubuki://` route and serves Vite's dedicated
`internal.html` entry point plus its static assets.

Current pages:

- `fubuki://newtab/`
- `fubuki://history/`
- `fubuki://bookmarks/`
- `fubuki://downloads/`
- `fubuki://settings/`
- `fubuki://debug/`

The internal origin does not receive the Frost Protocol bridge: only `fubuki://app/` can use it.
Non-destructive settings use the existing trusted, user-gesture-gated action route. Destructive
operations such as clearing history, removing downloads, and opening DevTools submit POST requests
to `fubuki://settings/set`.

Keep these pages simple: searchable lists, useful empty states, and direct actions. Avoid complex
popover UI or a full settings clone until the backing native behavior exists.
