# semcp

## What is semcp?

semcp (Secure Execution of MCP) is a CLI kit for running MCP servers in docker containers. It provides a **drop-in replacement** for

- `npx` -> `snpx`
- `uvx` -> `suvx`

specifically designed for Model Context Protocol (MCP) servers.

## Why use semcp?

`npx` and `uvx` allow you to execute packages, but they don't provide process isolation. `semcp` enhances this by running packages in isolated Docker containers.

## Limitations

`semcp` only works with stdio-based MCP servers. SSE / Streamable HTTP transports will be supported in the future.

## Installation

### Build from source

```bash
git clone <repository-url>
cd semcp
make install
```

### Install

```bash
make install
```

### Usage 

Check out the `snpx` and `suvx` READMEs for specific usage instructions.
