//! Search tool for AI assistants
//!
//! This module provides a real DuckDuckGo search tool.

use crate::tools::AiTool;
use anyhow::{Error, anyhow};
use async_trait::async_trait;
use reqwest;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, trace};

/// Parameters for the DuckDuckGo search tool.
#[derive(Deserialize)]
struct SearchParams {
    /// The search query to send to DuckDuckGo.
    query: String,
    /// Number of results to return (default: 3, max: 10)
    num_results: Option<usize>,
}

/// Represents a single search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchResult {
    title: String,
    link: String,
    snippet: String,
}

/// Tool for searching DuckDuckGo.
pub struct DDGSearchTool;

#[async_trait]
impl AiTool for DDGSearchTool {
    fn name(&self) -> &str {
        "search"
    }

    fn description(&self) -> &str {
        r#"Searches the web using DuckDuckGo. Use this tool liberally to find information you aren't certain about.
Important search operators:
cats dogs	results about cats or dogs
"cats and dogs"	exact term (avoid unless necessary)
~"cats and dogs"	semantically similar terms
cats -dogs	reduce results about dogs
cats +dogs	increase results about dogs
cats filetype:pdf	search pdfs about cats (supports doc(x), xls(x), ppt(x), html)
dogs site:example.com	search dogs on example.com
cats -site:example.com	exclude example.com from results
intitle:dogs	title contains "dogs"
inurl:cats	URL contains "cats""#
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of results to return (default: 3, max: 10)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, Error> {
        let params: SearchParams = serde_json::from_value(args.clone())
            .map_err(|_| anyhow!("Missing or invalid 'query' parameter"))?;
        let num_results = params.num_results.unwrap_or(3).clamp(1, 10);

        debug!("=== DDG SEARCH DEBUG ===");
        debug!("Query: '{}'", params.query);
        debug!("Num results: {}", num_results);

        let client = reqwest::Client::new();
        let url = format!("https://html.duckduckgo.com/html/?q={}", params.query);
        debug!("Request URL: {}", url);

        let resp = client
            .get(&url)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:140.0) Gecko/20100101 Firefox/140.0")
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Sec-GPC", "1")
            .header("Connection", "keep-alive")
            .header("Upgrade-Insecure-Requests", "1")
            .header("Sec-Fetch-Dest", "document")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", "none")
            .header("Sec-Fetch-User", "?1")
            .header("Priority", "u=0, i")
            .header("Pragma", "no-cache")
            .header("Cache-Control", "no-cache")
            .header("TE", "trailers")
            .send()
            .await
            .map_err(|e| anyhow!("Request error: {}", e))?;

        debug!("Response status: {}", resp.status());
        debug!("Response headers: {:?}", resp.headers());

        let body = resp
            .text()
            .await
            .map_err(|e| anyhow!("Body error: {}", e))?;

        debug!("Response body length: {} characters", body.len());

        // Check for potential blocking or redirection patterns
        if body.contains("blocked") || body.contains("captcha") || body.contains("verify") {
            debug!(
                "WARNING: Response may indicate blocking: contains 'blocked', 'captcha', or 'verify'"
            );
        }

        if body.len() < 1000 {
            debug!(
                "WARNING: Very short response body ({}): {}",
                body.len(),
                body.chars().take(200).collect::<String>()
            );
        }

        let document = Html::parse_document(&body);

        trace!("Parsed HTML document for query: {}", params.query);
        trace!("{:?}", body);

        let result_selector = Selector::parse(".web-result").unwrap();
        let result_title_selector = Selector::parse(".result__a").unwrap();
        let result_url_selector = Selector::parse(".result__url").unwrap();
        let result_snippet_selector = Selector::parse(".result__snippet").unwrap();

        let results = document
            .select(&result_selector)
            .filter_map(|result| {
                let title = result
                    .select(&result_title_selector)
                    .next()
                    .map(|n| n.text().collect::<Vec<_>>().join(""))
                    .unwrap_or_default();
                let link = result
                    .select(&result_url_selector)
                    .next()
                    .map(|n| n.text().collect::<Vec<_>>().join("").trim().to_string())
                    .unwrap_or_default();
                let snippet = result
                    .select(&result_snippet_selector)
                    .next()
                    .map(|n| n.text().collect::<Vec<_>>().join(""))
                    .unwrap_or_default();

                if !title.is_empty() && !link.is_empty() {
                    Some(SearchResult {
                        title,
                        link,
                        snippet,
                    })
                } else {
                    None
                }
            })
            .take(num_results)
            .collect::<Vec<_>>();

        debug!("Parsed {} search results", results.len());
        for (i, result) in results.iter().enumerate() {
            debug!(
                "Result #{}: title='{}', link='{}'",
                i + 1,
                result.title,
                result.link
            );
        }
        debug!("=== END DDG SEARCH DEBUG ===");

        Ok(serde_json::json!({ "results": results }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_metadata() {
        let tool = DDGSearchTool;

        assert_eq!(tool.name(), "search");
        assert!(!tool.description().is_empty());
        assert!(tool.description().contains("DuckDuckGo"));

        let schema = tool.schema();
        assert!(schema["type"].as_str() == Some("object"));
        assert!(schema["properties"]["query"].is_object());
        assert!(
            schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("query"))
        );
    }

    #[tokio::test]
    async fn test_parameter_validation() {
        let tool = DDGSearchTool;

        // Missing query parameter
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("query"));

        // Wrong parameter type - this will pass JSON validation but fail query parsing
        let result = tool.execute(json!({"query": 123})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_valid_query_structure() {
        let tool = DDGSearchTool;

        // Test with a simple valid query
        let result = tool.execute(json!({"query": "test"})).await;

        match result {
            Ok(response) => {
                // If successful, verify response structure
                assert!(response.is_object());
                assert!(response["results"].is_array());
            }
            Err(_) => {
                // Network failures are acceptable in test environment
            }
        }
    }

    #[tokio::test]
    async fn test_extra_parameters() {
        let tool = DDGSearchTool;

        // Extra parameters in the right structure should work
        let result = tool
            .execute(json!({
                "query": "test",
                "num_results": 5
            }))
            .await;

        // Should not fail due to the extra num_results parameter
        match result {
            Ok(_) => {} // Success
            Err(e) => {
                // Should not be a parameter validation error
                let error_msg = e.to_string().to_lowercase();
                assert!(
                    !error_msg.contains("unknown") && !error_msg.contains("unexpected"),
                    "Failed due to extra parameters: {}",
                    error_msg
                );
            }
        }
    }
}
