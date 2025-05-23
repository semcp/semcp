# snpx

snpx is a **drop-in replacement for `npx`** that runs npm packages in containerized environments, specifically designed for Model Context Protocol (MCP) servers.

## Tools Available

- **snpx**: A containerized replacement for `npx` (Node.js)
- **suv**: A containerized replacement for `uv` (Python)
- **suvx**: A containerized replacement for `uvx` (Python)

## Why use snpx and suv?

- `npx` and `uv` allow you to execute packages without installing them globally, but they don't provide process isolation.
- `snpx` and `suv` enhance this by running packages in isolated Docker containers.
- Ideal for running Model Context Protocol (MCP) servers in a containerized environment.

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

## Usage

### Node.js (Drop-in npx replacement)

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

### Python (Drop-in uv replacement)

```bash
# Replace uv with suv - it's that simple!
uv run mcp-server-time
↓
suv run mcp-server-time

# Replace uvx with suvx
uvx mcp-server-time
↓
suvx mcp-server-time
```

## Experiments

### Node.js MCP Servers

`snpx` is tested against the following reference node.js MCP servers:

- [x] `@modelcontextprotocol/server-sequential-thinking`
- [x] `@modelcontextprotocol/server-everything`
- [ ] `@modelcontextprotocol/server-filesystem`
- [ ] `@modelcontextprotocol/server-github`
- [ ] `@modelcontextprotocol/server-gdrive`
- [ ] `@modelcontextprotocol/server-google-maps`
- [ ] `@modelcontextprotocol/server-memory`
- [ ] `@modelcontextprotocol/server-redis`

### Python MCP Servers

`suv` is tested against the following reference Python MCP servers:

- [ ] `mcp-server-time`

## Troubleshooting

### Docker not available

`snpx` and `suv` fall back to regular `npx` and `uv` respectively if Docker is not available.

## Security (Future)

`snpx` and `suv` support configuration via policy files.
