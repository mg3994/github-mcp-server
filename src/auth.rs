use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};
use crate::error::GitHubMcpError;
use crate::models::User;
use crate::{log_auth_event};

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub token: String,
    pub validated_at: u64,
    pub expires_at: Option<u64>,
    pub scopes: Vec<String>,
    pub token_type: String,
}

#[derive(Debug)]
pub struct AuthManager {
    token_info: Option<TokenInfo>,
    authenticated_user: Option<User>,
    validation_cache_duration: u64, // seconds
}

impl AuthManager {
    pub fn new() -> Self {
        Self {
            token_info: None,
            authenticated_user: None,
            validation_cache_duration: 3600, // 1 hour default
        }
    }
    
    pub fn with_cache_duration(mut self, duration_seconds: u64) -> Self {
        self.validation_cache_duration = duration_seconds;
        self
    }
    
    pub async fn set_token(&mut self, token: String) -> Result<(), GitHubMcpError> {
        // Validate token format
        self.validate_token_format(&token)?;
        
        // Store the token with current timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        self.token_info = Some(TokenInfo {
            token: token.clone(),
            validated_at: now,
            expires_at: None,
            scopes: Vec::new(),
            token_type: self.detect_token_type(&token),
        });
        
        // Clear cached user info when token changes
        self.authenticated_user = None;
        
        debug!("Token stored successfully");
        Ok(())
    }
    
    pub fn get_token(&self) -> Option<&str> {
        self.token_info.as_ref().map(|info| info.token.as_str())
    }
    
    pub fn get_token_info(&self) -> Option<&TokenInfo> {
        self.token_info.as_ref()
    }
    
    pub fn is_authenticated(&self) -> bool {
        self.token_info.is_some()
    }
    
    pub fn is_token_valid(&self) -> bool {
        if let Some(token_info) = &self.token_info {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            // Check if token has expired
            if let Some(expires_at) = token_info.expires_at {
                if now >= expires_at {
                    warn!("Token has expired");
                    return false;
                }
            }
            
            // Check if validation cache is still valid
            let cache_valid = (now - token_info.validated_at) < self.validation_cache_duration;
            if !cache_valid {
                debug!("Token validation cache has expired");
            }
            
            cache_valid
        } else {
            false
        }
    }
    
    pub fn needs_revalidation(&self) -> bool {
        !self.is_token_valid()
    }
    
    pub fn get_authenticated_user(&self) -> Option<&User> {
        self.authenticated_user.as_ref()
    }
    
    pub fn set_authenticated_user(&mut self, user: User) {
        if let Some(ref mut token_info) = self.token_info {
            // Update validation timestamp when user is set
            token_info.validated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }
        
        log_auth_event!("user_authenticated", &user.login);
        info!("User authenticated: {}", user.login);
        self.authenticated_user = Some(user);
    }
    
    pub fn update_token_scopes(&mut self, scopes: Vec<String>) {
        if let Some(ref mut token_info) = self.token_info {
            token_info.scopes = scopes;
            debug!("Updated token scopes: {:?}", token_info.scopes);
        }
    }
    
    pub fn set_token_expiry(&mut self, expires_at: u64) {
        if let Some(ref mut token_info) = self.token_info {
            token_info.expires_at = Some(expires_at);
            debug!("Set token expiry: {}", expires_at);
        }
    }
    
    pub fn get_token_scopes(&self) -> Vec<String> {
        self.token_info
            .as_ref()
            .map(|info| info.scopes.clone())
            .unwrap_or_default()
    }
    
    pub fn has_scope(&self, scope: &str) -> bool {
        self.token_info
            .as_ref()
            .map(|info| info.scopes.contains(&scope.to_string()))
            .unwrap_or(false)
    }
    
    pub fn clear_authentication(&mut self) {
        if let Some(user) = &self.authenticated_user {
            log_auth_event!("user_logged_out", &user.login);
            info!("User logged out: {}", user.login);
        }
        
        self.token_info = None;
        self.authenticated_user = None;
        debug!("Authentication cleared");
    }
    
    pub fn get_token_age(&self) -> Option<u64> {
        self.token_info.as_ref().map(|info| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            now.saturating_sub(info.validated_at)
        })
    }
    
    pub fn get_time_until_expiry(&self) -> Option<u64> {
        self.token_info.as_ref().and_then(|info| {
            info.expires_at.map(|expires_at| {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                expires_at.saturating_sub(now)
            })
        })
    }
    
    fn validate_token_format(&self, token: &str) -> Result<(), GitHubMcpError> {
        if token.is_empty() {
            return Err(GitHubMcpError::AuthenticationError("Token cannot be empty".to_string()));
        }
        
        if token.len() < 10 {
            return Err(GitHubMcpError::AuthenticationError("Token appears to be too short".to_string()));
        }
        
        // Check for common token prefixes
        let valid_prefixes = ["ghp_", "gho_", "ghu_", "ghs_", "ghr_"];
        let has_valid_prefix = valid_prefixes.iter().any(|prefix| token.starts_with(prefix));
        
        if !has_valid_prefix && !token.chars().all(|c| c.is_ascii_alphanumeric()) {
            warn!("Token format may be invalid - no recognized prefix and contains non-alphanumeric characters");
        }
        
        Ok(())
    }
    
    fn detect_token_type(&self, token: &str) -> String {
        if token.starts_with("ghp_") {
            "personal_access_token".to_string()
        } else if token.starts_with("gho_") {
            "oauth_token".to_string()
        } else if token.starts_with("ghu_") {
            "user_access_token".to_string()
        } else if token.starts_with("ghs_") {
            "server_to_server_token".to_string()
        } else if token.starts_with("ghr_") {
            "refresh_token".to_string()
        } else {
            "unknown".to_string()
        }
    }
}

    // Authentication error handling methods
    pub async fn validate_token_with_github(&mut self, github_client: &crate::github::GitHubClient) -> Result<User, GitHubMcpError> {
        let token = self.get_token()
            .ok_or_else(|| GitHubMcpError::AuthenticationError("No token available for validation".to_string()))?;
        
        match github_client.authenticate(token).await {
            Ok(user) => {
                self.set_authenticated_user(user.clone());
                info!("Token validation successful for user: {}", user.login);
                Ok(user)
            },
            Err(GitHubMcpError::AuthenticationError(msg)) => {
                warn!("Token validation failed: {}", msg);
                self.handle_authentication_failure("Token validation failed", &msg).await;
                Err(GitHubMcpError::AuthenticationError(msg))
            },
            Err(GitHubMcpError::RateLimitError { retry_after }) => {
                warn!("Rate limit hit during token validation, retry after {} seconds", retry_after);
                Err(GitHubMcpError::RateLimitError { retry_after })
            },
            Err(GitHubMcpError::NetworkError(msg)) => {
                warn!("Network error during token validation: {}", msg);
                Err(GitHubMcpError::NetworkError(msg))
            },
            Err(other) => {
                warn!("Unexpected error during token validation: {}", other);
                Err(other)
            }
        }
    }
    
    pub async fn ensure_valid_authentication(&mut self, github_client: &crate::github::GitHubClient) -> Result<&User, GitHubMcpError> {
        // Check if we have a token
        if !self.is_authenticated() {
            return Err(GitHubMcpError::AuthenticationError("No authentication token provided".to_string()));
        }
        
        // Check if we have cached user info and token is still valid
        if let Some(user) = self.get_authenticated_user() {
            if self.is_token_valid() {
                debug!("Using cached authentication for user: {}", user.login);
                return Ok(user);
            }
        }
        
        // Need to validate token with GitHub
        debug!("Token validation required, checking with GitHub API");
        self.validate_token_with_github(github_client).await?;
        
        // Return the authenticated user
        self.get_authenticated_user()
            .ok_or_else(|| GitHubMcpError::AuthenticationError("Authentication validation failed".to_string()))
    }
    
    pub fn check_scope_permission(&self, required_scope: &str) -> Result<(), GitHubMcpError> {
        if !self.is_authenticated() {
            return Err(GitHubMcpError::AuthenticationError("Not authenticated".to_string()));
        }
        
        // If no scopes are cached, assume we have permission (for backwards compatibility)
        let scopes = self.get_token_scopes();
        if scopes.is_empty() {
            debug!("No cached scopes, assuming permission for scope: {}", required_scope);
            return Ok(());
        }
        
        // Check for specific scope or broader permissions
        let has_permission = scopes.iter().any(|scope| {
            scope == required_scope || 
            scope == "repo" || // repo scope covers most operations
            scope.starts_with(&format!("{}:", required_scope)) // scope with additional permissions
        });
        
        if has_permission {
            Ok(())
        } else {
            Err(GitHubMcpError::PermissionError(
                format!("Insufficient permissions. Required scope: '{}', available scopes: {:?}", 
                        required_scope, scopes)
            ))
        }
    }
    
    pub async fn handle_authentication_failure(&mut self, context: &str, error_message: &str) {
        warn!("Authentication failure in {}: {}", context, error_message);
        
        // Log the failure event
        if let Some(user) = &self.authenticated_user {
            log_auth_event!("authentication_failed", &user.login);
        } else {
            log_auth_event!("authentication_failed", "unknown_user");
        }
        
        // Clear authentication on certain types of failures
        if error_message.contains("Bad credentials") || 
           error_message.contains("Invalid token") ||
           error_message.contains("token expired") {
            info!("Clearing authentication due to credential failure");
            self.clear_authentication();
        }
    }
    
    pub fn get_authentication_status(&self) -> AuthenticationStatus {
        if !self.is_authenticated() {
            return AuthenticationStatus::NotAuthenticated;
        }
        
        if !self.is_token_valid() {
            return AuthenticationStatus::TokenExpired;
        }
        
        if self.needs_revalidation() {
            return AuthenticationStatus::NeedsRevalidation;
        }
        
        if self.get_authenticated_user().is_some() {
            AuthenticationStatus::Authenticated
        } else {
            AuthenticationStatus::TokenValidationPending
        }
    }
    
    pub fn get_authentication_summary(&self) -> AuthenticationSummary {
        let status = self.get_authentication_status();
        let token_info = self.get_token_info().cloned();
        let user = self.get_authenticated_user().cloned();
        let token_age = self.get_token_age();
        let time_until_expiry = self.get_time_until_expiry();
        
        AuthenticationSummary {
            status,
            token_info,
            user,
            token_age,
            time_until_expiry,
        }
    }

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum AuthenticationStatus {
    NotAuthenticated,
    TokenValidationPending,
    Authenticated,
    TokenExpired,
    NeedsRevalidation,
}

#[derive(Debug, Clone)]
pub struct AuthenticationSummary {
    pub status: AuthenticationStatus,
    pub token_info: Option<TokenInfo>,
    pub user: Option<User>,
    pub token_age: Option<u64>,
    pub time_until_expiry: Option<u64>,
}
