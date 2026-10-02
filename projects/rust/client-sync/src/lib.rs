use reqwest::{Method, blocking::Client};
use serde_json::Value;

/// Preserve HTTP status even when the error body is not JSON.
pub fn exchange(
    client: &Client,
    url: &str,
    method: Method,
    path: &str,
    token: &str,
    body: Option<&Value>,
) -> Result<(u16, Value), reqwest::Error> {
    let mut request = client.request(method, format!("{}{path}", url.trim_end_matches('/')));
    if !token.is_empty() {
        request = request.bearer_auth(token);
    }
    if let Some(body) = body {
        request = request.json(body);
    }
    let response = request.send()?;
    let status = response.status().as_u16();
    let text = response.text()?;
    let value =
        serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({"message": text}));
    Ok((status, value))
}
pub fn process_echo_line(output: &mut String, line: &str) -> bool {
    let content_without_newline = line.trim_end_matches(['\r', '\n']);
    if content_without_newline == "END" {
        if output.ends_with("\r\n") {
            output.truncate(output.len() - 2);
        } else if output.ends_with('\n') {
            output.pop();
        }
        return true;
    }
    let is_escaped_end =
        content_without_newline.starts_with('\\') && content_without_newline.ends_with("END");

    if is_escaped_end {
        if let Some(value) = line.strip_prefix('\\') {
            output.push_str(value);
        }
    } else {
        output.push_str(line);
    }

    false
}

pub fn make_echo_body(text: &str) -> serde_json::Value {
    serde_json::json!({
        "text": text
    })
}
