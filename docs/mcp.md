# Fubuki MCP Integration

Fubuki exposes AI browser automation through a separate `fubuki-mcp-server`
process. The browser itself does not embed MCP protocol handling.

## Process Model

```text
MCP client
  -> fubuki-mcp-server (stdio MCP)
  -> localhost IPC (127.0.0.1:42176)
  -> AutomationController
  -> BrowserAutomation / PageAutomation
  -> BrowserWindow / TabManager / BrowserDataStore
```

The IPC listener is started only when `automation.mcp.enabled=on`. The default
is `off`, and disabling the setting stops the external operation path.

## Settings

Open `fubuki://settings/` and use `MCP`.

- `automation.mcp.enabled`: starts or stops the local automation IPC server.
- `automation.mcp.confirmSensitive`: reserved for confirmation UI before
  destructive or sensitive operations.
- `automation.mcp.serverTemplate`: the selected settings template. Supported
  values are `fubuki`, `stdio`, `sse`, and `custom`.
- `automation.mcp.serverCommand`: the command shown in generated MCP client
  configuration. The default is `target/debug/fubuki-mcp-server`.
- `automation.mcp.serverArgs`: optional arguments for the generated MCP client
  configuration, stored as a JSON string array.
- `automation.mcp.clientName`: the server name used in generated MCP client
  configuration. The default is `fubuki`.
- `automation.mcp.enabledTools`: JSON string array of tools published to
  external MCP clients.

The MCP server is a local stdio MCP server. It is not an HTTP endpoint and does
not use an API key.

## Exposed Tools

The settings UI can disable individual tools through
`automation.mcp.enabledTools`.

- `browser.snapshot`
- `tabs.list`
- `tabs.create`
- `tabs.navigate`
- `tabs.activate`
- `tabs.close`
- `tabs.reload`
- `tabs.goBack`
- `tabs.goForward`
- `page.getText`
- `page.getHtml`
- `page.screenshot`
- `page.getAccessibilityTree`
- `page.click`
- `page.type`
- `page.press`
- `page.scroll`
- `page.find`
- `bookmarks.list`
- `history.list`
- `downloads.list`

## Security Rules

- MCP is off by default and must be explicitly enabled in settings.
- Private Window automation is blocked.
- `fubuki://` internal pages are not exposed to page automation.
- Page content returned through MCP is untrusted content.
- Tool calls are logged and visible from `fubuki://debug/`.
- Browser state changes continue to flow through the existing
  `CommandRegistry`, `TabManager`, `EventBus`, and FrostEngine boundaries.

## Forbidden APIs

The following are intentionally not exposed:

- `page.evaluate`
- raw JavaScript execution
- raw CDP commands
- `window.fubuki` or any automation bridge on arbitrary web pages
- direct external AI control of `fubuki://` internal pages

## Running the Server

Build the MCP server with:

```bash
cargo build -p fubuki-mcp-server
```

Configure an MCP client to run:

```bash
target/debug/fubuki-mcp-server
```

Example client configuration:

```json
{
  "mcpServers": {
    "fubuki": {
      "command": "target/debug/fubuki-mcp-server",
      "args": []
    }
  }
}
```

Fubuki must be running and `MCP` must be enabled before tools can reach the
browser.
