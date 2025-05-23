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

### Falco Security Integration

`snpx` can integrate with [Falco](https://falco.org) for enhanced runtime security monitoring and policy enforcement. Falco acts as a security monitor that can detect and alert on suspicious behavior in containers.

To use Falco with `snpx`:

1. Ensure Falco is installed on your system or available to your container environment
2. Enable Falco monitoring with the `--falco` flag:

```bash
snpx --falco -y @modelcontextprotocol/server-sequential-thinking
```

#### Custom Security Policies

Security policies can be defined in the `snpx.yaml` configuration file:

```yaml
falco:
  enabled: true  # Set to true to enable by default without the --falco flag
  rules:
    - name: "filesystem_access_control"
      description: "Block unauthorized file system access"
      enabled: true
      rules:
        - name: "write_sensitive_dirs"
          description: "Block writes to sensitive directories"
          condition: "open_write and fd.directory in (/etc, /root)"
          output: "Blocking write to sensitive directory (user=%user.name command=%proc.cmdline directory=%fd.directory)"
          priority: "WARNING"
          action: "terminate"
```

This configuration can be placed in:
- `./snpx.yaml` in the current directory
- `~/.snpx.yaml` in your home directory
- `~/.config/snpx/config.yaml` in your config directory
