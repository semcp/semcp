# snpx

snpx is a **drop-in replacement for `npx`** that runs npm packages in containerized environments, specifically designed for Model Context Protocol (MCP) servers.

## Why use snpx?

`npx` allows you to execute npm packages without installing them globally, but it doesn't provide process isolation. `snpx` enhances this by running packages in isolated Docker containers.

## Installation

### Build from source

```bash
git clone <repository-url>
cd snpx
make install
```

### Install

```bash
make install
```

## Usage (Drop-in npx replacement)

```bash
# Replace npx with snpx - it's that simple!
npx -y @modelcontextprotocol/server-sequential-thinking
↓
snpx -y @modelcontextprotocol/server-sequential-thinking

# All npx flags work the same way
npx -y cowsay hello
↓  
snpx -y cowsay hello
```

## Experiments

`snpx` is tested against the following reference node.js MCP servers:

- [x] `@modelcontextprotocol/server-sequential-thinking`
- [x] `@modelcontextprotocol/server-everything`
- [ ] `@modelcontextprotocol/server-filesystem`
- [ ] `@modelcontextprotocol/server-github`
- [ ] `@modelcontextprotocol/server-gdrive`
- [ ] `@modelcontextprotocol/server-google-maps`
- [ ] `@modelcontextprotocol/server-memory`
- [ ] `@modelcontextprotocol/server-redis`


## Troubleshooting

### Docker not available

`snpx` falls back to regular npx if Docker is not available.

## Security and Configuration

`snpx` supports configuration via `snpx.yaml` policy file. This file allows you to:

1. Define security policies for containerized execution
2. Configure package-specific transport protocols

### Transport Protocol Configuration

`snpx` automatically detects which transport protocol (HTTP, SSE, or stdio) is used by each package:

```yaml
# Package transport configuration in snpx.yaml
package_transports:
  # MCP servers with HTTP transport
  "@modelcontextprotocol/server-http": "http"
  "@modelcontextprotocol/server-web": "http"
  
  # MCP servers with SSE transport
  "@modelcontextprotocol/server-events": "sse"
  
  # Override automatic detection for custom packages
  "some-custom-package": "stdio"
```

If no configuration is provided, `snpx` uses name-based heuristics to detect the transport protocol:
- HTTP: Packages containing terms like "http", "web", "express", etc.
- SSE: Packages containing terms like "sse", "event-stream", "event-source", etc.
- Stdio: Default for most packages, including MCP servers
