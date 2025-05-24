# snpx

snpx is a **drop-in replacement for `npx`** that runs npm packages in containerized environments, specifically designed for Model Context Protocol (MCP) servers.

## Why use snpx?

`npx` allows you to execute npm packages without installing them globally, but it doesn't provide process isolation. `snpx` enhances this by running packages in isolated Docker containers.

## Limitations

As the name implies, `snpx` can only work with MCP servers that are available on npm.

Also, it only works with stdio-based MCP servers. SSE / Streamable HTTP transports will be supported in the future.

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

# policy file for enviornment variables, file mounting, networking, or docker security flags, use a policy file
npx -y @modelcontextprotocol/server-filesystem path/to/use
↓
snpx --policy samples/filesystem/policy.yaml -y @modelcontextprotocol/server-filesystem path/to/use
```

## Experiments

`snpx` is tested against the following reference node.js MCP servers:

- [x] `@modelcontextprotocol/server-sequential-thinking`
- [x] `@modelcontextprotocol/server-everything`
- [x] `@modelcontextprotocol/server-filesystem` (requires fs mounting)
- [x] `@modelcontextprotocol/server-github` (requires networking and secrets)
- [ ] `@modelcontextprotocol/server-google-maps`
- [ ] `@modelcontextprotocol/server-memory`
- [ ] `@modelcontextprotocol/server-redis`


## Troubleshooting

### Docker not available

`snpx` falls back to regular npx if Docker is not available.

## Capability Policy

`snpx` supports configuration via capability policy files defined in YAML format. You can find examples in the `samples` directory.