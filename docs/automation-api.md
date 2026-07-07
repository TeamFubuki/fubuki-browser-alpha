# Automation API

Fubuki's native automation API is a local IPC protocol used by
`fubuki-mcp-server`. It is not a web-exposed bridge.

## Transport

- Address: `127.0.0.1:42176`
- Framing: one JSON request per line, one JSON response per line
- Availability: only while `automation.mcp.enabled=on`

Request shape:

```json
{ "method": "tabs.navigate", "params": { "tabId": "tab-1", "url": "https://example.com" } }
```

Success response:

```json
{ "ok": true, "result": true }
```

Error response:

```json
{ "ok": false, "error": { "code": "disabled", "message": "MCP automation is disabled" } }
```

## Safety Model

`AutomationController` checks global enablement, blocks Private Window access,
routes known methods only, and rejects forbidden APIs such as `page.evaluate`
and raw CDP. Page operations use browser-native input and frame text/source
APIs; they do not inject JavaScript into page context.

Destructive and sensitive operations must be added behind
`automation.mcp.confirmSensitive` confirmation before becoming externally
callable.

## Method Groups

- Browser state: `browser.snapshot`
- Tabs: `tabs.list`, `tabs.create`, `tabs.navigate`, `tabs.activate`,
  `tabs.close`, `tabs.reload`, `tabs.goBack`, `tabs.goForward`
- Page read/input: `page.getText`, `page.getHtml`,
  `page.getAccessibilityTree`, `page.click`, `page.type`, `page.press`,
  `page.scroll`, `page.find`
- Local records: `bookmarks.list`, `history.list`, `downloads.list`

`page.screenshot` is reserved in the protocol and currently returns a structured
error until the native capture pipeline is wired.
