use super::util;
use super::Handler;

pub struct CurlHandler;

impl Handler for CurlHandler {
    fn filter(&self, output: &str, _args: &[String]) -> String {
        let trimmed = output.trim();

        // Detect JSON by Content-Type hint in headers or by prefix
        let body = extract_body(trimmed);

        if let Some(json_str) = body {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                let schema = util::json_to_schema(&v);
                let schema_str = serde_json::to_string_pretty(&schema).unwrap_or_default();
                // Size guard: if schema is larger than original, pass through
                if schema_str.len() < json_str.len() {
                    return schema_str;
                }
            }
        }

        output.to_string()
    }
}

/// Extract the response body from curl output (headers + body or just body).
fn extract_body(output: &str) -> Option<&str> {
    // If output contains HTTP headers (curl -i or -v), split at the blank line
    if output.starts_with("HTTP/") {
        // Find double newline separating headers from body
        if let Some(pos) = output.find("\r\n\r\n") {
            return Some(&output[pos + 4..]);
        }
        if let Some(pos) = output.find("\n\n") {
            return Some(&output[pos + 2..]);
        }
    }

    // Whole output is the body
    let b = output.trim();
    if b.starts_with('{') || b.starts_with('[') {
        Some(output)
    } else {
        None
    }
}

