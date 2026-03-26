use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;

use arawn_agent::Result;
use arawn_agent::tool::{Tool, ToolContext, ToolResult};

/// Web search provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum SearchProvider {
    /// Brave Search API
    Brave { api_key: String },
    /// Serper (Google Search API)
    Serper { api_key: String },
    /// Tavily Search API
    Tavily { api_key: String },
    /// DuckDuckGo (no API key needed, but limited)
    DuckDuckGo,
}

/// Configuration for web search.
#[derive(Debug, Clone)]
pub struct WebSearchConfig {
    /// Search provider configuration.
    pub provider: SearchProvider,
    /// Maximum number of results to return.
    pub max_results: usize,
    /// Request timeout.
    pub timeout: Duration,
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            provider: SearchProvider::DuckDuckGo,
            max_results: 10,
            timeout: Duration::from_secs(30),
        }
    }
}

/// A single search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Tool for searching the web.
#[derive(Debug, Clone)]
pub struct WebSearchTool {
    client: Client,
    config: WebSearchConfig,
}

impl WebSearchTool {
    /// Create a new web search tool with default configuration (DuckDuckGo).
    pub fn new() -> std::result::Result<Self, reqwest::Error> {
        Self::with_config(WebSearchConfig::default())
    }

    /// Create a web search tool with custom configuration.
    pub fn with_config(config: WebSearchConfig) -> std::result::Result<Self, reqwest::Error> {
        let client = Client::builder().timeout(config.timeout).build()?;

        Ok(Self { client, config })
    }

    /// Create a web search tool with Brave Search.
    pub fn brave(api_key: impl Into<String>) -> std::result::Result<Self, reqwest::Error> {
        Self::with_config(WebSearchConfig {
            provider: SearchProvider::Brave {
                api_key: api_key.into(),
            },
            ..Default::default()
        })
    }

    /// Create a web search tool with Serper.
    pub fn serper(api_key: impl Into<String>) -> std::result::Result<Self, reqwest::Error> {
        Self::with_config(WebSearchConfig {
            provider: SearchProvider::Serper {
                api_key: api_key.into(),
            },
            ..Default::default()
        })
    }

    /// Create a web search tool with Tavily.
    pub fn tavily(api_key: impl Into<String>) -> std::result::Result<Self, reqwest::Error> {
        Self::with_config(WebSearchConfig {
            provider: SearchProvider::Tavily {
                api_key: api_key.into(),
            },
            ..Default::default()
        })
    }

    async fn search_brave(&self, query: &str, api_key: &str) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://api.search.brave.com/res/v1/web/search?q={}&count={}",
            urlencoding::encode(query),
            self.config.max_results
        );

        let response = self
            .client
            .get(&url)
            .header("X-Subscription-Token", api_key)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| arawn_agent::AgentError::Tool(format!("Brave search failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(arawn_agent::AgentError::Tool(format!(
                "Brave search error: {}",
                response.status()
            )));
        }

        let data: Value = response.json().await.map_err(|e| {
            arawn_agent::AgentError::Tool(format!("Failed to parse response: {}", e))
        })?;

        let results = data["web"]["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| {
                        Some(SearchResult {
                            title: r["title"].as_str()?.to_string(),
                            url: r["url"].as_str()?.to_string(),
                            snippet: r["description"].as_str().unwrap_or("").to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }

    async fn search_serper(&self, query: &str, api_key: &str) -> Result<Vec<SearchResult>> {
        let response = self
            .client
            .post("https://google.serper.dev/search")
            .header("X-API-KEY", api_key)
            .header("Content-Type", "application/json")
            .json(&json!({
                "q": query,
                "num": self.config.max_results
            }))
            .send()
            .await
            .map_err(|e| arawn_agent::AgentError::Tool(format!("Serper search failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(arawn_agent::AgentError::Tool(format!(
                "Serper search error: {}",
                response.status()
            )));
        }

        let data: Value = response.json().await.map_err(|e| {
            arawn_agent::AgentError::Tool(format!("Failed to parse response: {}", e))
        })?;

        let results = data["organic"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| {
                        Some(SearchResult {
                            title: r["title"].as_str()?.to_string(),
                            url: r["link"].as_str()?.to_string(),
                            snippet: r["snippet"].as_str().unwrap_or("").to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }

    async fn search_tavily(&self, query: &str, api_key: &str) -> Result<Vec<SearchResult>> {
        let response = self
            .client
            .post("https://api.tavily.com/search")
            .header("Content-Type", "application/json")
            .json(&json!({
                "api_key": api_key,
                "query": query,
                "max_results": self.config.max_results
            }))
            .send()
            .await
            .map_err(|e| arawn_agent::AgentError::Tool(format!("Tavily search failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(arawn_agent::AgentError::Tool(format!(
                "Tavily search error: {}",
                response.status()
            )));
        }

        let data: Value = response.json().await.map_err(|e| {
            arawn_agent::AgentError::Tool(format!("Failed to parse response: {}", e))
        })?;

        let results = data["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| {
                        Some(SearchResult {
                            title: r["title"].as_str()?.to_string(),
                            url: r["url"].as_str()?.to_string(),
                            snippet: r["content"].as_str().unwrap_or("").to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }

    async fn search_duckduckgo(&self, query: &str) -> Result<Vec<SearchResult>> {
        // DuckDuckGo instant answer API (limited but free)
        let url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
            urlencoding::encode(query)
        );

        let response = self.client.get(&url).send().await.map_err(|e| {
            arawn_agent::AgentError::Tool(format!("DuckDuckGo search failed: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(arawn_agent::AgentError::Tool(format!(
                "DuckDuckGo search error: {}",
                response.status()
            )));
        }

        let data: Value = response.json().await.map_err(|e| {
            arawn_agent::AgentError::Tool(format!("Failed to parse response: {}", e))
        })?;

        let mut results = Vec::new();

        // Add abstract if available
        if let Some(abstract_text) = data["AbstractText"].as_str()
            && !abstract_text.is_empty()
        {
            results.push(SearchResult {
                title: data["Heading"].as_str().unwrap_or("Result").to_string(),
                url: data["AbstractURL"].as_str().unwrap_or("").to_string(),
                snippet: abstract_text.to_string(),
            });
        }

        // Add related topics
        if let Some(topics) = data["RelatedTopics"].as_array() {
            for topic in topics.iter().take(self.config.max_results - results.len()) {
                if let (Some(text), Some(url)) =
                    (topic["Text"].as_str(), topic["FirstURL"].as_str())
                {
                    results.push(SearchResult {
                        title: text.chars().take(50).collect::<String>() + "...",
                        url: url.to_string(),
                        snippet: text.to_string(),
                    });
                }
            }
        }

        Ok(results)
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            tracing::error!("failed to build default HTTP client: {e}");
            Self {
                client: reqwest::Client::new(),
                config: Default::default(),
            }
        })
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for information. Returns a list of relevant results with titles, URLs, and snippets."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolResult> {
        if ctx.is_cancelled() {
            return Ok(ToolResult::error("Operation cancelled"));
        }

        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                arawn_agent::AgentError::Tool("Missing 'query' parameter".to_string())
            })?;

        let results = match &self.config.provider {
            SearchProvider::Brave { api_key } => self.search_brave(query, api_key).await,
            SearchProvider::Serper { api_key } => self.search_serper(query, api_key).await,
            SearchProvider::Tavily { api_key } => self.search_tavily(query, api_key).await,
            SearchProvider::DuckDuckGo => self.search_duckduckgo(query).await,
        };

        match results {
            Ok(results) => {
                if results.is_empty() {
                    Ok(ToolResult::text("No results found"))
                } else {
                    Ok(ToolResult::json(json!({
                        "query": query,
                        "count": results.len(),
                        "results": results
                    })))
                }
            }
            Err(e) => Ok(ToolResult::error(format!("Search failed: {}", e))),
        }
    }
}
