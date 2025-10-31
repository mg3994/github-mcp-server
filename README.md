# GitHub MCP Server

A Model Context Protocol (MCP) server implementation in Rust that provides GitHub integration capabilities for AI assistants.

## Features

- GitHub API authentication with personal access tokens
- Repository operations (list, search, file access)
- Issue management (create, update, list)
- Pull request operations
- Comprehensive error handling and rate limiting
- Configurable through environment variables

## Installation

### Prerequisites

- Rust 1.70 or later
- A GitHub personal access token

### Building from Source

```bash
git clone https://github.com/yourusername/github-mcp-server.git
cd github-mcp-server
cargo build --release
```

## Configuration

The server can be configured using environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `GITHUB_API_URL` | `https://api.github.com` | GitHub API base URL |
| `REQUEST_TIMEOUT` | `30` | Request timeout in seconds |
| `LOG_LEVEL` | `info` | Logging level (trace, debug, info, warn, error) |
| `MAX_RETRIES` | `3` | Maximum retry attempts for failed requests |
| `RATE_LIMIT_BUFFER` | `10` | Rate limit buffer percentage |

## Usage

### Running the Server

```bash
# Using default configuration
./target/release/github-mcp-server

# With custom log level
./target/release/github-mcp-server --log-level debug
```

### MCP Tools

The server provides the following MCP tools:

#### `github_auth`
Authenticate with GitHub using a personal access token.

**Parameters:**
- `token` (string): GitHub personal access token

**Example:**
```json
{
  "name": "github_auth",
  "arguments": {
    "token": "ghp_your_token_here"
  }
}
```

More tools will be documented as they are implemented.

## Development

### Running Tests

```bash
cargo test
```

### Running with Debug Logging

```bash
RUST_LOG=debug cargo run
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Submit a pull request

## Security

- GitHub tokens are stored in memory only and never persisted to disk
- All GitHub API requests use HTTPS
- Input validation is performed on all tool parameters