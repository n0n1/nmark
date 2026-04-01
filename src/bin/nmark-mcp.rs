use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use nmark::app::{App, ConvertRequest};
use nmark::cli::{RunOptions, SourceArg};
use nmark::settings::resolve_config;
use serde::Deserialize;
use serde_json::{json, Value};

const SERVER_NAME: &str = "nmark";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const MCP_PROTOCOL_VERSION: &str = "2025-06-18";

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Deserialize, Default)]
struct ToolCallParams {
    name: String,
    #[serde(default)]
    arguments: ConvertArgs,
}

#[derive(Debug, Deserialize, Default)]
struct ConvertArgs {
    url: String,
    #[serde(default)]
    output_dir: Option<String>,
    #[serde(default)]
    include_frontmatter: Option<bool>,
    #[serde(default)]
    save_to_file: Option<bool>,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let message = match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(message) => message,
            Err(error) => {
                eprintln!("Invalid JSON-RPC message: {error}");
                continue;
            }
        };

        if message.jsonrpc != "2.0" {
            if let Some(id) = message.id {
                write_json(
                    &mut stdout,
                    &jsonrpc_error(id, -32600, "jsonrpc must be `2.0`", None),
                )?;
            }
            continue;
        }

        let response = match message.method.as_str() {
            "initialize" => message
                .id
                .map(|id| jsonrpc_result(id, initialize_result(&message.params))),
            "notifications/initialized" => None,
            "ping" => message.id.map(|id| jsonrpc_result(id, json!({}))),
            "tools/list" => message.id.map(|id| jsonrpc_result(id, list_tools_result())),
            "tools/call" => match message.id {
                Some(id) => Some(handle_tool_call(id, &message.params).await),
                None => None,
            },
            _ => message
                .id
                .map(|id| jsonrpc_error(id, -32601, "method not found", None)),
        };

        if let Some(response) = response {
            write_json(&mut stdout, &response)?;
        }
    }

    Ok(())
}

async fn handle_tool_call(id: Value, params: &Value) -> Value {
    let params: ToolCallParams = match serde_json::from_value(params.clone()) {
        Ok(params) => params,
        Err(error) => {
            return jsonrpc_error(
                id,
                -32602,
                "invalid tools/call params",
                Some(json!({ "details": error.to_string() })),
            );
        }
    };

    if params.name != "nmark_convert" {
        return jsonrpc_error(
            id,
            -32602,
            "unknown tool",
            Some(json!({ "tool": params.name })),
        );
    }

    if params.arguments.url.trim().is_empty() {
        return jsonrpc_error(
            id,
            -32602,
            "tool argument `url` is required",
            None,
        );
    }

    let save_to_file = params.arguments.save_to_file.unwrap_or(false);
    let run_options = RunOptions {
        source: Some(SourceArg::Url(params.arguments.url.clone())),
        output_dir: params.arguments.output_dir.map(PathBuf::from),
        write_to_stdout: Some(!save_to_file),
        include_frontmatter: params.arguments.include_frontmatter,
    };

    match resolve_config(run_options) {
        Ok(config) => {
            let request = ConvertRequest::from_config(&config, &params.arguments.url);
            let app = match App::new(&config.http) {
                Ok(app) => app,
                Err(error) => {
                    return jsonrpc_result(
                        id,
                        json!({
                            "content": [{
                                "type": "text",
                                "text": error.to_string()
                            }],
                            "isError": true
                        }),
                    );
                }
            };

            match app.convert(&request).await {
                Ok(result) => {
                    let text = match &result.output_path {
                        Some(path) => format!(
                            "Saved markdown to {}\n\n{}",
                            path.display(),
                            result.markdown
                        ),
                        None => result.markdown.clone(),
                    };

                    jsonrpc_result(
                        id,
                        json!({
                            "content": [{
                                "type": "text",
                                "text": text
                            }],
                            "structuredContent": {
                                "url": result.url,
                                "title": result.title,
                                "author": result.author,
                                "tags": result.tags,
                                "markdown": result.markdown,
                                "outputPath": result.output_path.map(|path| path.display().to_string())
                            }
                        }),
                    )
                }
                Err(error) => jsonrpc_result(
                    id,
                    json!({
                        "content": [{
                            "type": "text",
                            "text": error.to_string()
                        }],
                        "isError": true
                    }),
                ),
            }
        }
        Err(error) => jsonrpc_result(
            id,
            json!({
                "content": [{
                    "type": "text",
                    "text": error.to_string()
                }],
                "isError": true
            }),
        ),
    }
}

fn initialize_result(params: &Value) -> Value {
    let requested = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(MCP_PROTOCOL_VERSION);

    let protocol_version = if matches!(requested, "2025-06-18" | "2025-03-26") {
        requested
    } else {
        MCP_PROTOCOL_VERSION
    };

    json!({
        "protocolVersion": protocol_version,
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "version": SERVER_VERSION
        }
    })
}

fn list_tools_result() -> Value {
    json!({
        "tools": [{
            "name": "nmark_convert",
            "title": "Convert Article To Markdown",
            "description": "Download an article URL, extract readable content, and return Markdown. Optionally save the result to a file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "Article URL to download and convert."
                    },
                    "output_dir": {
                        "type": "string",
                        "description": "Optional output directory used when save_to_file is true."
                    },
                    "include_frontmatter": {
                        "type": "boolean",
                        "description": "Override frontmatter generation. Defaults to config.toml behavior."
                    },
                    "save_to_file": {
                        "type": "boolean",
                        "description": "When true, also save the generated Markdown to a file. Defaults to false."
                    }
                },
                "required": ["url"]
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string" },
                    "title": { "type": "string" },
                    "author": { "type": ["string", "null"] },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "markdown": { "type": "string" },
                    "outputPath": { "type": ["string", "null"] }
                },
                "required": ["url", "title", "tags", "markdown"]
            }
        }]
    })
}

fn jsonrpc_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn jsonrpc_error(id: Value, code: i64, message: &str, data: Option<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
            "data": data
        }
    })
}

fn write_json(stdout: &mut impl Write, value: &Value) -> io::Result<()> {
    serde_json::to_writer(&mut *stdout, value)
        .map_err(|error| io::Error::other(error.to_string()))?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}

#[cfg(test)]
mod tests {
    use super::{initialize_result, list_tools_result, MCP_PROTOCOL_VERSION};
    use serde_json::json;

    #[test]
    fn initialize_negotiates_supported_version() {
        let result = initialize_result(&json!({ "protocolVersion": "2025-03-26" }));
        assert_eq!(result["protocolVersion"], "2025-03-26");
    }

    #[test]
    fn initialize_falls_back_to_current_version() {
        let result = initialize_result(&json!({ "protocolVersion": "2024-11-05" }));
        assert_eq!(result["protocolVersion"], MCP_PROTOCOL_VERSION);
    }

    #[test]
    fn exposes_nmark_convert_tool() {
        let tools = list_tools_result();
        assert_eq!(tools["tools"][0]["name"], "nmark_convert");
    }
}
