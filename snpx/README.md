# snpx

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
- [x] `@modelcontextprotocol/server-filesystem` (requires fs mounting)
- [x] `@modelcontextprotocol/server-github` (requires networking and secrets)
- [ ] `@modelcontextprotocol/server-google-maps`
- [ ] `@modelcontextprotocol/server-memory`
- [ ] `@modelcontextprotocol/server-redis`


## Troubleshooting

### Docker not available

`snpx` does not fall back to regular npx if Docker is not available.

## Capability Policy

`snpx` supports configuration via capability policy files defined in YAML format. You can find examples in the `samples` directory.