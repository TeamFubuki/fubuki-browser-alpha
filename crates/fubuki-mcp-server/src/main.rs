use serde_json::{Value, json};
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

const FUBUKI_AUTOMATION_ADDR: &str = "127.0.0.1:42176";

const TOOLS: &[&str] = &[
    "browser.snapshot",
    "tabs.list",
    "tabs.create",
    "tabs.navigate",
    "tabs.activate",
    "tabs.close",
    "tabs.reload",
    "tabs.goBack",
    "tabs.goForward",
    "page.getText",
    "page.getHtml",
    "page.screenshot",
    "page.getAccessibilityTree",
    "page.click",
    "page.type",
    "page.press",
    "page.scroll",
    "page.find",
    "bookmarks.list",
    "history.list",
    "downloads.list",
];

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = serde_json::from_str(&line).unwrap_or_else(|_| json!({}));
        if let Some(response) = handle_json_rpc(request) {
            writeln!(stdout, "{response}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

fn handle_json_rpc(request: Value) -> Option<Value> {
    let is_notification = request.get("id").is_none();
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": { "name": "fubuki-mcp-server", "version": env!("CARGO_PKG_VERSION") },
            "capabilities": { "tools": {} }
        }),
        "notifications/initialized" => return None,
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tool_descriptors() }),
        "tools/call" => {
            let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match call_fubuki(name, arguments) {
                Ok(value) => json!({
                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string()) }],
                    "isError": value.get("ok").and_then(Value::as_bool) == Some(false)
                }),
                Err(err) => json!({
                    "content": [{ "type": "text", "text": err }],
                    "isError": true
                }),
            }
        }
        _ => {
            if is_notification {
                return None;
            }
            return Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Unknown method: {method}") }
            }));
        }
    };
    if is_notification {
        None
    } else {
        Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
    }
}

fn tool_descriptors() -> Vec<Value> {
    TOOLS
        .iter()
        .map(|name| {
            json!({
                "name": name,
                "description": tool_description(name),
                "inputSchema": {
                    "type": "object",
                    "additionalProperties": true,
                    "properties": tool_properties(name)
                }
            })
        })
        .collect()
}

fn tool_description(name: &str) -> &'static str {
    match name {
        "page.getHtml" => "Return the current page HTML without running page JavaScript.",
        "page.getText" => "Return visible page text without running page JavaScript.",
        "page.click" => "Click page coordinates in the selected tab.",
        "page.type" => "Type trusted input into the focused page element.",
        "page.press" => "Press a key in the selected tab.",
        "page.scroll" => "Scroll the selected tab.",
        "page.find" => "Use browser find-in-page.",
        "page.screenshot" => "Capture a screenshot when native capture support is available.",
        "page.getAccessibilityTree" => "Return a safe accessibility summary.",
        "tabs.create" => "Create a browser tab.",
        "tabs.navigate" => "Navigate an existing tab.",
        _ => "Call a safe Fubuki browser automation command.",
    }
}

fn tool_properties(name: &str) -> Value {
    match name {
        "tabs.create" => json!({ "url": { "type": "string" } }),
        "tabs.navigate" => json!({ "tabId": { "type": "string" }, "url": { "type": "string" } }),
        "tabs.activate" | "tabs.close" | "tabs.reload" | "tabs.goBack" | "tabs.goForward" => {
            json!({ "tabId": { "type": "string" } })
        }
        "page.click" => json!({
            "tabId": { "type": "string" },
            "x": { "type": "integer" },
            "y": { "type": "integer" }
        }),
        "page.type" => json!({ "tabId": { "type": "string" }, "text": { "type": "string" } }),
        "page.press" => json!({ "tabId": { "type": "string" }, "key": { "type": "string" } }),
        "page.scroll" => json!({
            "tabId": { "type": "string" },
            "x": { "type": "integer" },
            "y": { "type": "integer" },
            "deltaX": { "type": "integer" },
            "deltaY": { "type": "integer" }
        }),
        "page.find" => json!({ "tabId": { "type": "string" }, "query": { "type": "string" } }),
        "page.getText" | "page.getHtml" | "page.screenshot" | "page.getAccessibilityTree" => {
            json!({ "tabId": { "type": "string" } })
        }
        _ => json!({}),
    }
}

fn call_fubuki(method: &str, params: Value) -> Result<Value, String> {
    if !TOOLS.contains(&method) {
        return Err(format!("Forbidden or unknown tool: {method}"));
    }
    let mut stream = TcpStream::connect(FUBUKI_AUTOMATION_ADDR)
        .map_err(|err| format!("Fubuki automation IPC unavailable: {err}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|err| err.to_string())?;
    let request = json!({ "method": method, "params": params });
    writeln!(stream, "{request}").map_err(|err| err.to_string())?;
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader
        .read_line(&mut response)
        .map_err(|err| err.to_string())?;
    serde_json::from_str(&response).map_err(|err| err.to_string())
}
