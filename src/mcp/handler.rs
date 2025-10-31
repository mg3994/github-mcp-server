use serde_json::json;
use tracing::{debug, error};

use crate::auth::AuthManager;
use crate::error::GitHubMcpError;
use crate::github::GitHubClient;
use crate::models::*;

pub struct McpHandler {
    github_client: GitHubClient,
    auth_manager: AuthManager,
}

impl McpHandler {
    pub fn new(github_client: GitHubClient) -> Self {
        Self {
            github_client,
            auth_manager: AuthManager::new(),
        }
    }
    
    pub async fn handle_initialize(&mut self) -> Result<serde_json::Value, GitHubMcpError> {
        debug!("Handling MCP initialize request");
        
        let response = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "github-mcp-server",
                "version": "0.1.0"
            }
        });
        
        Ok(response)
    }
    
    pub async fn list_tools(&self) -> Result<Vec<Tool>, GitHubMcpError> {
        let tools = vec![
            Tool {
                name: "github_auth".to_string(),
                description: "Authenticate with GitHub using a personal access token".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "token": {
                            "type": "string",
                            "description": "GitHub personal access token"
                        }
                    },
                    "required": ["token"]
                }),
            },
            // Additional tools will be added in subsequent tasks
        ];
        
        Ok(tools)
    }
    
    pub async fn handle_tool_call(&mut self, request: ToolCallRequest) -> Result<ToolCallResponse, GitHubMcpError> {
        debug!("Handling tool call: {}", request.name);
        
        match request.name.as_str() {
            "github_auth" => self.handle_auth_tool(request.arguments).await,
            _ => Err(GitHubMcpError::InvalidRequest(format!("Unknown tool: {}", request.name))),
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
}