use serde_json::json;
use tracing::{debug, error, info};
use base64::Engine;

use crate::auth::AuthManager;
use crate::error::GitHubMcpError;
use crate::github::GitHubClient;
use crate::models::*;

pub struct McpHandler {
    github_client: GitHubClient,
    auth_manager: AuthManager,
    initialized: bool,
    protocol_version: String,
    client_capabilities: Option<ClientCapabilities>,
}

impl McpHandler {
    pub fn new(github_client: GitHubClient) -> Self {
        Self {
            github_client,
            auth_manager: AuthManager::new(),
            initialized: false,
            protocol_version: "2024-11-05".to_string(),
            client_capabilities: None,
        }
    }
    
    pub async fn handle_initialize(&mut self, params: InitializeParams) -> Result<InitializeResult, GitHubMcpError> {
        debug!("Handling MCP initialize request from client: {}", params.client_info.name);
        
        // Validate protocol version compatibility
        if !self.is_protocol_version_compatible(&params.protocol_version) {
            return Err(GitHubMcpError::McpError(
                format!("Unsupported protocol version: {}. Supported versions: {}", 
                        params.protocol_version, self.protocol_version)
            ));
        }
        
        // Store client capabilities for future reference
        self.client_capabilities = Some(params.capabilities.clone());
        
        // Mark as initialized
        self.initialized = true;
        
        info!(
            client_name = %params.client_info.name,
            client_version = %params.client_info.version,
            protocol_version = %params.protocol_version,
            "MCP server initialized successfully"
        );
        
        let response = InitializeResult {
            protocol_version: self.protocol_version.clone(),
            capabilities: ServerCapabilities {
                experimental: None,
                logging: None,
                prompts: None,
                resources: None,
                tools: Some(json!({})), // We support tools
            },
            server_info: ServerInfo {
                name: "github-mcp-server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };
        
        Ok(response)
    }
    
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    pub fn get_protocol_version(&self) -> &str {
        &self.protocol_version
    }
    
    pub fn get_client_capabilities(&self) -> Option<&ClientCapabilities> {
        self.client_capabilities.as_ref()
    }
    
    fn is_protocol_version_compatible(&self, client_version: &str) -> bool {
        // For now, we only support the specific version we're built for
        // In the future, this could be more flexible to support version ranges
        client_version == self.protocol_version
    }
    
    fn ensure_initialized(&self) -> Result<(), GitHubMcpError> {
        if !self.initialized {
            return Err(GitHubMcpError::McpError(
                "Server not initialized. Call initialize first.".to_string()
            ));
        }
        Ok(())
    }
    
    pub async fn list_tools(&self) -> Result<ListToolsResult, GitHubMcpError> {
        self.ensure_initialized()?;
        
        debug!("Listing available MCP tools");
        
        // Use the comprehensive tool schemas from models
        let tools = create_tool_schemas();
        
        info!("Returning {} available tools", tools.len());
        
        Ok(ListToolsResult {
            tools,
            next_cursor: None, // We don't use pagination for tools currently
        })
    }
    
    pub async fn handle_tool_call(&mut self, params: CallToolParams) -> Result<CallToolResult, GitHubMcpError> {
        self.ensure_initialized()?;
        
        debug!("Handling tool call: {}", params.name);
        
        let start_time = std::time::Instant::now();
        
        let result = match params.name.as_str() {
            // Authentication
            "github_auth" => self.handle_auth_tool(params.arguments.unwrap_or_default()).await,
            
            // Repository operations
            "github_list_repos" => self.handle_list_repos_tool(params.arguments.unwrap_or_default()).await,
            "github_search_repos" => self.handle_search_repos_tool(params.arguments.unwrap_or_default()).await,
            "github_get_file" => self.handle_get_file_tool(params.arguments.unwrap_or_default()).await,
            "github_list_directory" => self.handle_list_directory_tool(params.arguments.unwrap_or_default()).await,
            
            // Issue operations
            "github_list_issues" => self.handle_list_issues_tool(params.arguments.unwrap_or_default()).await,
            "github_create_issue" => self.handle_create_issue_tool(params.arguments.unwrap_or_default()).await,
            "github_update_issue" => self.handle_update_issue_tool(params.arguments.unwrap_or_default()).await,
            
            // Pull request operations
            "github_list_prs" => self.handle_list_prs_tool(params.arguments.unwrap_or_default()).await,
            "github_create_pr" => self.handle_create_pr_tool(params.arguments.unwrap_or_default()).await,
            "github_get_pr_details" => self.handle_get_pr_details_tool(params.arguments.unwrap_or_default()).await,
            "github_merge_pr" => self.handle_merge_pr_tool(params.arguments.unwrap_or_default()).await,
            
            _ => {
                error!("Unknown tool requested: {}", params.name);
                Err(GitHubMcpError::InvalidRequest(format!("Unknown tool: {}", params.name)))
            }
        };
        
        let duration = start_time.elapsed();
        crate::log_mcp_tool_call!(&params.name, duration.as_millis());
        
        // Convert legacy response format to new format
        match result {
            Ok(legacy_response) => {
                let content = legacy_response.content.into_iter()
                    .map(|c| ToolContent::Text { text: c.text })
                    .collect();
                
                Ok(CallToolResult {
                    content,
                    is_error: legacy_response.is_error,
                })
            },
            Err(e) => {
                error!("Tool call failed: {}", e);
                Ok(CallToolResult {
                    content: vec![ToolContent::Text { 
                        text: format!("Error: {}", e) 
                    }],
                    is_error: Some(true),
                })
            }
        }
    }
    
    async fn handle_auth_tool(&mut self, arguments: serde_json::Value) -> Result<ToolCallResponse, GitHubMcpError> {
        let token = arguments.get("token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing token parameter".to_string()))?;
        
        // Authenticate with GitHub
        match self.github_client.authenticate(token).await {
            Ok(user) => {
                self.auth_manager.set_token(token.to_string()).await?;
                self.auth_manager.set_authenticated_user(user.clone());
                
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Successfully authenticated as {}", user.login),
                    }],
                    is_error: Some(false),
                })
            },
            Err(e) => {
                error!("Authentication failed: {}", e);
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Authentication failed: {}", e),
                    }],
                    is_error: Some(true),
                })
            }
        }
    }
    
    // Repository tool handlers
    async fn handle_list_repos_tool(&mut self, arguments: serde_json::Value) -> Result<ToolCallResponse, GitHubMcpError> {
        let token = self.get_authenticated_token()?;
        
        let params = ListReposParams {
            visibility: arguments.get("visibility").and_then(|v| v.as_str()).map(|s| s.to_string()),
            sort: arguments.get("sort").and_then(|v| v.as_str()).map(|s| s.to_string()),
            direction: arguments.get("direction").and_then(|v| v.as_str()).map(|s| s.to_string()),
            per_page: arguments.get("per_page").and_then(|v| v.as_u64()).map(|n| n as u32),
            page: arguments.get("page").and_then(|v| v.as_u64()).map(|n| n as u32),
        };
        
        match self.github_client.list_repositories(&token, &params).await {
            Ok(repositories) => {
                let repo_list = repositories.iter()
                    .map(|repo| format!("- {} ({}): {}", repo.full_name, repo.visibility, repo.description.as_deref().unwrap_or("No description")))
                    .collect::<Vec<_>>()
                    .join("\n");
                
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Found {} repositories:\n{}", repositories.len(), repo_list),
                    }],
                    is_error: Some(false),
                })
            },
            Err(e) => {
                error!("Failed to list repositories: {}", e);
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Failed to list repositories: {}", e),
                    }],
                    is_error: Some(true),
                })
            }
        }
    }
    
    async fn handle_search_repos_tool(&mut self, arguments: serde_json::Value) -> Result<ToolCallResponse, GitHubMcpError> {
        let token = self.get_authenticated_token()?;
        
        let query = arguments.get("q")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: q".to_string()))?;
        
        let sort = arguments.get("sort").and_then(|v| v.as_str());
        let order = arguments.get("order").and_then(|v| v.as_str());
        let per_page = arguments.get("per_page").and_then(|v| v.as_u64()).map(|n| n as u32);
        let page = arguments.get("page").and_then(|v| v.as_u64()).map(|n| n as u32);
        
        match self.github_client.search_repositories(&token, query, sort, order, per_page, page).await {
            Ok(repositories) => {
                let repo_list = repositories.iter()
                    .map(|repo| format!("- {} ⭐{}: {}", repo.full_name, repo.stargazers_count, repo.description.as_deref().unwrap_or("No description")))
                    .collect::<Vec<_>>()
                    .join("\n");
                
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Found {} repositories matching '{}':\n{}", repositories.len(), query, repo_list),
                    }],
                    is_error: Some(false),
                })
            },
            Err(e) => {
                error!("Failed to search repositories: {}", e);
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Failed to search repositories: {}", e),
                    }],
                    is_error: Some(true),
                })
            }
        }
    }
    
    async fn handle_get_file_tool(&mut self, arguments: serde_json::Value) -> Result<ToolCallResponse, GitHubMcpError> {
        let token = self.get_authenticated_token()?;
        
        let owner = arguments.get("owner")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: owner".to_string()))?;
        let repo = arguments.get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: repo".to_string()))?;
        let path = arguments.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: path".to_string()))?;
        let ref_name = arguments.get("ref").and_then(|v| v.as_str());
        
        match self.github_client.get_file_content(&token, owner, repo, path, ref_name).await {
            Ok(file_content) => {
                let content = if let Some(content) = &file_content.content {
                    match base64::engine::general_purpose::STANDARD.decode(content.replace('\n', "")) {
                        Ok(decoded) => String::from_utf8_lossy(&decoded).to_string(),
                        Err(_) => format!("Binary file (size: {} bytes)", file_content.size),
                    }
                } else {
                    "No content available".to_string()
                };
                
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("File: {}/{}/{}\nSize: {} bytes\n\n{}", owner, repo, path, file_content.size, content),
                    }],
                    is_error: Some(false),
                })
            },
            Err(e) => {
                error!("Failed to get file content: {}", e);
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Failed to get file content: {}", e),
                    }],
                    is_error: Some(true),
                })
            }
        }
    }
    
    async fn handle_list_directory_tool(&mut self, arguments: serde_json::Value) -> Result<ToolCallResponse, GitHubMcpError> {
        let token = self.get_authenticated_token()?;
        
        let owner = arguments.get("owner")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: owner".to_string()))?;
        let repo = arguments.get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: repo".to_string()))?;
        let path = arguments.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let ref_name = arguments.get("ref").and_then(|v| v.as_str());
        
        match self.github_client.list_directory(&token, owner, repo, path, ref_name).await {
            Ok(items) => {
                let item_list = items.iter()
                    .map(|item| {
                        let icon = match item.item_type.as_str() {
                            "dir" => "📁",
                            "file" => "📄",
                            _ => "❓",
                        };
                        format!("{} {} ({})", icon, item.name, item.item_type)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                
                let path_display = if path.is_empty() { "root" } else { path };
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Directory listing for {}/{}/{} ({} items):\n{}", owner, repo, path_display, items.len(), item_list),
                    }],
                    is_error: Some(false),
                })
            },
            Err(e) => {
                error!("Failed to list directory: {}", e);
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Failed to list directory: {}", e),
                    }],
                    is_error: Some(true),
                })
            }
        }
    }
    
    // Issue tool handlers
    async fn handle_list_issues_tool(&mut self, arguments: serde_json::Value) -> Result<ToolCallResponse, GitHubMcpError> {
        let token = self.get_authenticated_token()?;
        
        let owner = arguments.get("owner")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: owner".to_string()))?;
        let repo = arguments.get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: repo".to_string()))?;
        
        let params = ListIssuesParams {
            state: arguments.get("state").and_then(|v| v.as_str()).map(|s| s.to_string()),
            labels: arguments.get("labels").and_then(|v| v.as_str()).map(|s| s.to_string()),
            assignee: arguments.get("assignee").and_then(|v| v.as_str()).map(|s| s.to_string()),
            sort: arguments.get("sort").and_then(|v| v.as_str()).map(|s| s.to_string()),
            direction: arguments.get("direction").and_then(|v| v.as_str()).map(|s| s.to_string()),
            per_page: arguments.get("per_page").and_then(|v| v.as_u64()).map(|n| n as u32),
            page: arguments.get("page").and_then(|v| v.as_u64()).map(|n| n as u32),
        };
        
        match self.github_client.list_issues(&token, owner, repo, &params).await {
            Ok(issues) => {
                let issue_list = issues.iter()
                    .map(|issue| {
                        let state_icon = match issue.state {
                            IssueState::Open => "🟢",
                            IssueState::Closed => "🔴",
                        };
                        format!("{} #{}: {}", state_icon, issue.number, issue.title)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Found {} issues in {}/{}:\n{}", issues.len(), owner, repo, issue_list),
                    }],
                    is_error: Some(false),
                })
            },
            Err(e) => {
                error!("Failed to list issues: {}", e);
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Failed to list issues: {}", e),
                    }],
                    is_error: Some(true),
                })
            }
        }
    }
    
    async fn handle_create_issue_tool(&mut self, arguments: serde_json::Value) -> Result<ToolCallResponse, GitHubMcpError> {
        let token = self.get_authenticated_token()?;
        
        let owner = arguments.get("owner")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: owner".to_string()))?;
        let repo = arguments.get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: repo".to_string()))?;
        let title = arguments.get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: title".to_string()))?;
        
        let request = CreateIssueRequest {
            title: title.to_string(),
            body: arguments.get("body").and_then(|v| v.as_str()).map(|s| s.to_string()),
            labels: arguments.get("labels")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()),
            assignees: arguments.get("assignees")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()),
        };
        
        match self.github_client.create_issue(&token, owner, repo, &request).await {
            Ok(issue) => {
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Created issue #{}: {}\nURL: {}", issue.number, issue.title, issue.html_url),
                    }],
                    is_error: Some(false),
                })
            },
            Err(e) => {
                error!("Failed to create issue: {}", e);
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Failed to create issue: {}", e),
                    }],
                    is_error: Some(true),
                })
            }
        }
    }
    
    async fn handle_update_issue_tool(&mut self, arguments: serde_json::Value) -> Result<ToolCallResponse, GitHubMcpError> {
        let token = self.get_authenticated_token()?;
        
        let owner = arguments.get("owner")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: owner".to_string()))?;
        let repo = arguments.get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: repo".to_string()))?;
        let issue_number = arguments.get("issue_number")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: issue_number".to_string()))? as u32;
        
        let state = arguments.get("state")
            .and_then(|v| v.as_str())
            .and_then(|s| match s {
                "open" => Some(IssueState::Open),
                "closed" => Some(IssueState::Closed),
                _ => None,
            });
        
        let request = UpdateIssueRequest {
            title: arguments.get("title").and_then(|v| v.as_str()).map(|s| s.to_string()),
            body: arguments.get("body").and_then(|v| v.as_str()).map(|s| s.to_string()),
            state,
            labels: arguments.get("labels")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()),
            assignees: arguments.get("assignees")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()),
        };
        
        match self.github_client.update_issue(&token, owner, repo, issue_number, &request).await {
            Ok(issue) => {
                let state_icon = match issue.state {
                    IssueState::Open => "🟢",
                    IssueState::Closed => "🔴",
                };
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Updated issue #{}: {} {}\nURL: {}", issue.number, state_icon, issue.title, issue.html_url),
                    }],
                    is_error: Some(false),
                })
            },
            Err(e) => {
                error!("Failed to update issue: {}", e);
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Failed to update issue: {}", e),
                    }],
                    is_error: Some(true),
                })
            }
        }
    }
    
    // Pull request tool handlers
    async fn handle_list_prs_tool(&mut self, arguments: serde_json::Value) -> Result<ToolCallResponse, GitHubMcpError> {
        let token = self.get_authenticated_token()?;
        
        let owner = arguments.get("owner")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: owner".to_string()))?;
        let repo = arguments.get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: repo".to_string()))?;
        
        let state = arguments.get("state").and_then(|v| v.as_str()).unwrap_or("open");
        let head = arguments.get("head").and_then(|v| v.as_str());
        let base = arguments.get("base").and_then(|v| v.as_str());
        let sort = arguments.get("sort").and_then(|v| v.as_str());
        let direction = arguments.get("direction").and_then(|v| v.as_str());
        let per_page = arguments.get("per_page").and_then(|v| v.as_u64()).map(|n| n as u32);
        let page = arguments.get("page").and_then(|v| v.as_u64()).map(|n| n as u32);
        
        match self.github_client.list_pull_requests(&token, owner, repo, state, head, base, sort, direction, per_page, page).await {
            Ok(prs) => {
                let pr_list = prs.iter()
                    .map(|pr| {
                        let state_icon = match pr.state {
                            PullRequestState::Open => "🟢",
                            PullRequestState::Closed => {
                                if pr.merged_at.is_some() { "🟣" } else { "🔴" }
                            },
                        };
                        format!("{} #{}: {} ({}→{})", state_icon, pr.number, pr.title, pr.head.ref_name, pr.base.ref_name)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Found {} pull requests in {}/{}:\n{}", prs.len(), owner, repo, pr_list),
                    }],
                    is_error: Some(false),
                })
            },
            Err(e) => {
                error!("Failed to list pull requests: {}", e);
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Failed to list pull requests: {}", e),
                    }],
                    is_error: Some(true),
                })
            }
        }
    }
    
    async fn handle_create_pr_tool(&mut self, arguments: serde_json::Value) -> Result<ToolCallResponse, GitHubMcpError> {
        let token = self.get_authenticated_token()?;
        
        let owner = arguments.get("owner")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: owner".to_string()))?;
        let repo = arguments.get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: repo".to_string()))?;
        let title = arguments.get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: title".to_string()))?;
        let head = arguments.get("head")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: head".to_string()))?;
        let base = arguments.get("base")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: base".to_string()))?;
        
        let request = CreatePullRequestRequest {
            title: title.to_string(),
            body: arguments.get("body").and_then(|v| v.as_str()).map(|s| s.to_string()),
            head: head.to_string(),
            base: base.to_string(),
            draft: arguments.get("draft").and_then(|v| v.as_bool()),
        };
        
        match self.github_client.create_pull_request(&token, owner, repo, &request).await {
            Ok(pr) => {
                let draft_text = if pr.draft { " (Draft)" } else { "" };
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Created pull request #{}: {}{}\nURL: {}", pr.number, pr.title, draft_text, pr.html_url),
                    }],
                    is_error: Some(false),
                })
            },
            Err(e) => {
                error!("Failed to create pull request: {}", e);
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Failed to create pull request: {}", e),
                    }],
                    is_error: Some(true),
                })
            }
        }
    }
    
    async fn handle_get_pr_details_tool(&mut self, arguments: serde_json::Value) -> Result<ToolCallResponse, GitHubMcpError> {
        let token = self.get_authenticated_token()?;
        
        let owner = arguments.get("owner")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: owner".to_string()))?;
        let repo = arguments.get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: repo".to_string()))?;
        let pull_number = arguments.get("pull_number")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: pull_number".to_string()))? as u32;
        
        match self.github_client.get_pull_request(&token, owner, repo, pull_number).await {
            Ok(pr) => {
                let state_icon = match pr.state {
                    PullRequestState::Open => "🟢",
                    PullRequestState::Closed => "🔴",
                    PullRequestState::Merged => "🟣",
                };
                let draft_text = if pr.draft { " (Draft)" } else { "" };
                let mergeable_text = match pr.mergeable {
                    Some(true) => "✅ Mergeable",
                    Some(false) => "❌ Not mergeable",
                    None => "❓ Mergeable status unknown",
                };
                
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!(
                            "Pull Request #{}: {}{}\n{}\nBranches: {} → {}\nAuthor: {}\nCreated: {}\n{}\nURL: {}",
                            pr.number, pr.title, draft_text, state_icon, pr.head.ref_name, pr.base.ref_name,
                            pr.user.login, pr.created_at, mergeable_text, pr.html_url
                        ),
                    }],
                    is_error: Some(false),
                })
            },
            Err(e) => {
                error!("Failed to get pull request details: {}", e);
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Failed to get pull request details: {}", e),
                    }],
                    is_error: Some(true),
                })
            }
        }
    }
    
    async fn handle_merge_pr_tool(&mut self, arguments: serde_json::Value) -> Result<ToolCallResponse, GitHubMcpError> {
        let token = self.get_authenticated_token()?;
        
        let owner = arguments.get("owner")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: owner".to_string()))?;
        let repo = arguments.get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: repo".to_string()))?;
        let pull_number = arguments.get("pull_number")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| GitHubMcpError::InvalidRequest("Missing required parameter: pull_number".to_string()))? as u32;
        
        let commit_title = arguments.get("commit_title").and_then(|v| v.as_str());
        let commit_message = arguments.get("commit_message").and_then(|v| v.as_str());
        let merge_method = arguments.get("merge_method").and_then(|v| v.as_str()).unwrap_or("merge");
        
        match self.github_client.merge_pull_request(&token, owner, repo, pull_number, commit_title, commit_message, merge_method).await {
            Ok(merge_result) => {
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Successfully merged pull request #{} using {} method\nMerge commit: {}", 
                                    pull_number, merge_method, merge_result.get("sha").and_then(|v| v.as_str()).unwrap_or("unknown")),
                    }],
                    is_error: Some(false),
                })
            },
            Err(e) => {
                error!("Failed to merge pull request: {}", e);
                Ok(ToolCallResponse {
                    content: vec![ToolResponseContent {
                        content_type: "text".to_string(),
                        text: format!("Failed to merge pull request: {}", e),
                    }],
                    is_error: Some(true),
                })
            }
        }
    }
    
    // Helper method to get authenticated token
    fn get_authenticated_token(&self) -> Result<String, GitHubMcpError> {
        self.auth_manager.get_token()
            .map(|t| t.to_string())
            .ok_or_else(|| GitHubMcpError::AuthenticationError("Not authenticated. Please use github_auth tool first.".to_string()))
    }
    
    pub async fn handle_mcp_request(&mut self, request: McpRequest) -> McpResponse {
        let response_id = request.id.clone();
        
        match request.method.as_str() {
            "initialize" => {
                match request.params {
                    Some(params) => {
                        match serde_json::from_value::<InitializeParams>(params) {
                            Ok(init_params) => {
                                match self.handle_initialize(init_params).await {
                                    Ok(result) => McpResponse {
                                        jsonrpc: "2.0".to_string(),
                                        id: response_id,
                                        result: Some(serde_json::to_value(result).unwrap_or_default()),
                                        error: None,
                                    },
                                    Err(e) => McpResponse {
                                        jsonrpc: "2.0".to_string(),
                                        id: response_id,
                                        result: None,
                                        error: Some(e.to_mcp_error()),
                                    },
                                }
                            },
                            Err(e) => McpResponse {
                                jsonrpc: "2.0".to_string(),
                                id: response_id,
                                result: None,
                                error: Some(McpError {
                                    code: -32602,
                                    message: format!("Invalid initialize parameters: {}", e),
                                    data: None,
                                }),
                            },
                        }
                    },
                    None => McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: response_id,
                        result: None,
                        error: Some(McpError {
                            code: -32602,
                            message: "Missing initialize parameters".to_string(),
                            data: None,
                        }),
                    },
                }
            },
            "tools/list" => {
                match self.list_tools().await {
                    Ok(result) => McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: response_id,
                        result: Some(serde_json::to_value(result).unwrap_or_default()),
                        error: None,
                    },
                    Err(e) => McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: response_id,
                        result: None,
                        error: Some(e.to_mcp_error()),
                    },
                }
            },
            "tools/call" => {
                match request.params {
                    Some(params) => {
                        match serde_json::from_value::<CallToolParams>(params) {
                            Ok(call_params) => {
                                match self.handle_tool_call(call_params).await {
                                    Ok(result) => McpResponse {
                                        jsonrpc: "2.0".to_string(),
                                        id: response_id,
                                        result: Some(serde_json::to_value(result).unwrap_or_default()),
                                        error: None,
                                    },
                                    Err(e) => McpResponse {
                                        jsonrpc: "2.0".to_string(),
                                        id: response_id,
                                        result: None,
                                        error: Some(e.to_mcp_error()),
                                    },
                                }
                            },
                            Err(e) => McpResponse {
                                jsonrpc: "2.0".to_string(),
                                id: response_id,
                                result: None,
                                error: Some(McpError {
                                    code: -32602,
                                    message: format!("Invalid tool call parameters: {}", e),
                                    data: None,
                                }),
                            },
                        }
                    },
                    None => McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: response_id,
                        result: None,
                        error: Some(McpError {
                            code: -32602,
                            message: "Missing tool call parameters".to_string(),
                            data: None,
                        }),
                    },
                }
            },
            _ => {
                error!("Unknown MCP method: {}", request.method);
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: response_id,
                    result: None,
                    error: Some(McpError {
                        code: -32601,
                        message: format!("Method not found: {}", request.method),
                        data: None,
                    }),
                }
            }
        }
    }}
