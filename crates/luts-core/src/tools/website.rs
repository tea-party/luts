use anyhow::{Error, anyhow};
use serde_json::Value;
use tracing::debug;

use crate::tools::AiTool;

/// Tool that fetches a website and renders its content as HTML or Markdown.
pub struct WebsiteTool;

#[async_trait::async_trait]
impl AiTool for WebsiteTool {
    fn name(&self) -> &str {
        "website"
    }

    fn description(&self) -> &str {
        r#"Fetches a website.
Parameters:
- `website`: The URL of the website to fetch.
- `render`: Which format to render the content in. Options are "html" or "md" (default is "md").

Note: The website must start with http:// or https://. If not, https:// will be prepended automatically.
"#
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "website": {
                    "type": "string",
                    "description": "The URL of the website to fetch"
                },
                "render": {
                    "type": "string",
                    "description": "Format to render the content: 'html' or 'md' (default: 'md')"
                }
            },
            "required": ["website"]
        })
    }

    fn validate_params(&self, params: &Value) -> Result<(), Error> {
        if !params.is_object() {
            return Err(anyhow!("Parameters must be an object"));
        }
        if !params.get("website").is_some_and(|v| v.is_string()) {
            return Err(anyhow!("Missing or invalid 'website' parameter"));
        }
        if let Some(render) = params.get("render") {
            if !render.is_string() {
                return Err(anyhow!("'render' must be a string"));
            }
        }
        Ok(())
    }

    async fn execute(&self, params: Value) -> Result<Value, Error> {
        self.validate_params(&params)?;

        let client = reqwest::Client::new();
        let website = params
            .get("website")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'website' parameter"))?;
        let render = params
            .get("render")
            .and_then(|v| v.as_str())
            .unwrap_or("md");

        if !website.starts_with("http://") && !website.starts_with("https://") {
            debug!("Prepending 'https://' to website URL");
            let website = format!("https://{}", website);
            debug!("Final website URL: {}", website);
        }

        let resp = client
            .get(website)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36 Edg/114.0.1823.67a")
            .send()
            .await
            .map_err(|e| anyhow!("Request error: {}", e))?;

        debug!("Response status: {}", resp.status());

        let body = resp
            .text()
            .await
            .map_err(|e| anyhow!("Body error: {}", e))?;

        debug!("Response body length: {}", body.len());

        match render {
            "html" => Ok(serde_json::json!({ "content": body })),
            "md" => {
                let markdown = html2md::rewrite_html(&body, false);
                debug!("Converted HTML to Markdown, length: {}", markdown.len());
                Ok(serde_json::json!({ "content": markdown }))
            }
            _ => Err(anyhow!(
                "Invalid 'render' parameter, must be 'html' or 'md'"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_metadata() {
        let tool = WebsiteTool;
        
        assert_eq!(tool.name(), "website");
        assert!(!tool.description().is_empty());
        assert!(tool.description().contains("website") || tool.description().contains("content"));
        
        let schema = tool.schema();
        assert!(schema["type"].as_str() == Some("object"));
        assert!(schema["properties"]["website"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("website")));
    }

    #[tokio::test]
    async fn test_parameter_validation() {
        let tool = WebsiteTool;
        
        // Missing URL parameter
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("website"));
        
        // Wrong parameter type
        let result = tool.execute(json!({"website": 123})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_url_validation() {
        let tool = WebsiteTool;
        
        // Invalid URLs should be rejected during parameter validation
        let invalid_urls = vec![
            "not-a-url",
            "ftp://invalid-scheme",
            "javascript:alert('xss')",
        ];
        
        for url in invalid_urls {
            let result = tool.execute(json!({"website": url})).await;
            assert!(result.is_err(), "Expected rejection for invalid URL: {}", url);
        }
    }

    #[tokio::test]
    async fn test_valid_url_formats() {
        let tool = WebsiteTool;
        
        // Valid URLs (though they might not exist)
        let valid_urls = vec![
            "https://example.com",
            "http://example.com",
        ];
        
        for url in valid_urls {
            let result = tool.execute(json!({"website": url})).await;
            
            match result {
                Ok(response) => {
                    // If successful, verify response structure
                    assert!(response.is_object());
                    assert!(response["content"].is_string());
                }
                Err(e) => {
                    // Network failure is acceptable, but should not be URL validation errors
                    let error_msg = e.to_string().to_lowercase();
                    assert!(
                        !error_msg.contains("invalid url") && !error_msg.contains("scheme"),
                        "Unexpected URL validation error for valid URL {}: {}", url, error_msg
                    );
                }
            }
        }
    }
}
