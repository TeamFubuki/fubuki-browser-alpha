# Permission Broker verification

Tested on macOS Apple Silicon with the locally installed CEF distribution using a fresh `http://localhost:18763` origin.

- Camera and microphone requests display the origin and correct permission in the top-right prompt. Previously the host revealed a top-right rectangle while CSS placed the prompt at the bottom, hiding the controls.
- Camera **Not now** completes with `Permission denied`; requesting again displays another prompt.
- Camera **Block** dismisses the prompt and denies the request.
- Earlier response attempts expired under the original 10-second timeout. The timeout now gives users 60 seconds. Camera Block was re-tested within that interval and its origin-scoped SQLite row was confirmed. Microphone Allow and successful device access remain unverified.
- Notifications return `denied` without a prompt. Pointer Lock reports a Chromium error. Geolocation and Keyboard Lock remain to be verified; this issue is still open.
- Automated tests cover all supported CEF permission masks, unknown masks, device-only media capture, callback reentry, cancellation, recycled prompt IDs with stale timers, origin/type-scoped SQLite persistence after reopening, duplicate responses, and late answers after dismissal/window close. Existing private-window tests cover non-persistence.

![Camera permission prompt](permission-prompt.png)
