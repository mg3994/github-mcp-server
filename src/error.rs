use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum GitHubMcpError {
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),
    
    #[error("GitHub API error: {status} - {message}")]
    GitHubApiError { status: u16, message: String },
    
    #[error("Rate limit exceeded. Retry after: {retry_after}")]
    RateLimitError { retry_after: u64 },
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Permission denied: {0}")]
    PermissionError(String),
    
    #[error("Invalid configuration: {0}")]
    ConfigError(String),
    
    #[error("MCP protocol error: {0}")]
    McpError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl From<reqwest::Error> for GitHubMcpError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            GitHubMcpError::NetworkError("Request timeout".to_string())
        } else if err.is_connect() {
            GitHubMcpError::NetworkError("Connection failed".to_string())
        } else {
            GitHubMcpError::NetworkError(err.to_string())
        }
    }
}

impl From<serde_json::Error> for GitHubMcpError {
    fn from(err: serde_json::Error) -> Self {
        GitHubMcpError::SerializationError(err.to_string())
    }
}

impl From<url::ParseError> for GitHubMcpError {
    fn from(err: url::ParseError) -> Self {
        GitHubMcpError::ConfigError(format!("Invalid URL: {}", err))
    }
}

impl From<std::env::VarError> for GitHubMcpError {
    fn from(err: std::env::VarError) -> Self {
        GitHubMcpError::ConfigError(format!("Environment variable error: {}", err))
    }
}

impl GitHubMcpError {
    pub fn to_error_response(&self) -> ErrorResponse {
        let (code, message) = match self {
            GitHubMcpError::AuthenticationError(msg) => (401, msg.clone()),
            GitHubMcpError::GitHubApiError { status, message } => (*status as i32, message.clone()),
            GitHubMcpError::RateLimitError { retry_after } => (429, format!("Rate limit exceeded. Retry after {} seconds", retry_after)),
            GitHubMcpError::NetworkError(msg) => (503, msg.clone()),
            GitHubMcpError::PermissionError(msg) => (403, msg.clone()),
            GitHubMcpError::ConfigError(msg) => (500, msg.clone()),
            GitHubMcpError::McpError(msg) => (400, msg.clone()),
            GitHubMcpError::SerializationError(msg) => (500, msg.clone()),
            GitHubMcpError::InvalidRequest(msg) => (400, msg.clone()),
        };
        
        ErrorResponse {
            code,
            message,
            data: None,
        }
    }
    
    pub fn to_mcp_error(&self) -> crate::models::McpError {
        let error_response = self.to_error_response();
        crate::models::McpError {
            code: error_response.code,
            message: error_response.message,
            data: error_response.data,
        }
    }
    
    pub fn is_retryable(&self) -> bool {
        match self {
            GitHubMcpError::NetworkError(_) => true,
            GitHubMcpError::RateLimitError { .. } => true,
            GitHubMcpError::GitHubApiError { status, .. } => *status >= 500,
            _ => false,
        }
    }
    
    pub fn retry_after(&self) -> Option<u64> {
        match self {
            GitHubMcpError::RateLimitError { retry_after } => Some(*retry_after),
            _ => None,
        }
    }
}