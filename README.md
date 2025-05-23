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

## Security (Future)

`snpx` supports configuration via `snpx.yaml` policy file.

### Open Policy Agent (OPA) Integration

`snpx` can use Open Policy Agent (OPA) for runtime policy enforcement:

```bash
# Enable OPA policy enforcement
snpx --opa -y @modelcontextprotocol/server-sequential-thinking

# Specify a custom policy file
snpx --opa --policy-file=my-policy.yaml -y cowsay hello
```

This integration allows for sophisticated runtime security monitoring and enforcement based on the policies defined in `snpx.yaml`. OPA uses the Rego policy language to enforce rules related to filesystem access, network activity, and container capabilities.
