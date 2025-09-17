# Dynamics CLI

A Rust-based command-line interface for Microsoft Dynamics 365, providing an intuitive way to query and interact with your Dynamics instances from the terminal.

## Features

- **FQL Query Language**: Execute queries using FetchXML Query Language (FQL), a jq-inspired syntax that compiles to FetchXML
- **Multi-Environment Support**: Manage multiple Dynamics 365 environments with persistent configuration
- **Interactive Setup**: Guided authentication setup with Azure AD OAuth2
- **Flexible Output**: JSON, XML, and formatted table output options
- **Entity Management**: Built-in and custom entity name mappings for Web API compatibility

## Installation

Install directly from source:

```bash
cargo install --path .
```

Or run from source:

```bash
cargo run -- <command>
```

## Quick Start

```bash
# Setup authentication for your Dynamics instance
dynamics-cli auth setup

# Execute a simple query
dynamics-cli query run ".account | .name, .revenue | limit(10)"

# Check authentication status
dynamics-cli auth status
```

## Documentation

For complete usage instructions, command reference, and FQL language specification, see [USAGE.md](USAGE.md).

## Development

This project uses Nix flakes for reproducible development:

```bash
nix develop    # Enter development shell
cargo build    # Build the project
cargo test     # Run tests
```