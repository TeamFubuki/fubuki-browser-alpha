# Internal Pages

Internal pages are independent SolidJS components under `ui/src/pages/internal/` and share the
dedicated `internal.html` Vite entry point. Native CEF scheme handling keeps each `fubuki://` host,
serves the built assets, and exposes read-only page data at `/data.json`.

Keep route-level components small. Read state through `useInternalData()` and send mutations through
`actions.ts`; do not couple a page to Frost Protocol methods or navigation-based form actions.
