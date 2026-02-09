# AutoHands

<div align="center">

<img src="https://img.shields.io/badge/rust-1.85+-orange.svg" alt="Rust Version">
<img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg" alt="License">
<img src="https://img.shields.io/badge/status-alpha-red.svg" alt="Status">

**Omnipotent Autonomous Agent Framework**

[Documentation](docs/ARCHITECTURE.md) | [Roadmap](docs/ROADMAP.md)

</div>

---

## Overview

AutoHands is a highly extensible autonomous AI agent framework written in Rust. It enables AI agents to perform complex tasks autonomously through tool use, browser automation, desktop control, and more.

### Key Features

- **Modular Architecture** - Core defines protocols and interfaces only; all capabilities are extensions
- **Rich Tool Ecosystem** - File operations, shell commands, browser automation, desktop control, code analysis
- **Multi-Provider Support** - Anthropic Claude, OpenAI, Google Gemini, Volcengine Ark
- **24/7 Autonomous Operation** - RunLoop-based event-driven architecture with daemon support
- **Progressive Skill Disclosure** - Dynamic skill loading with metadata injection
- **MCP Protocol Bridge** - Connect to Model Context Protocol servers

### Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              External Inputs                                 │
│   HTTP API    │   WebSocket   │   Webhook   │   Cron/Timer   │   Signal     │
└───────────────┴───────────────┴─────────────┴────────────────┴──────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           autohands-api                                      │
│   RunLoopBridge: Convert all inputs to Tasks and submit to RunLoop          │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         autohands-runloop (Core)                             │
│   TaskQueue │ RunLoop │ Source0/Source1 │ Observer │ Timer │ AgentDriver    │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         autohands-runtime                                    │
│   AgentLoop   │   Context Builder   │   History   │   Streaming              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Rust 1.85+
- An LLM API key (Anthropic, OpenAI, or Ark)

### Installation

```bash
# Clone the repository
git clone https://github.com/AgentsMesh/AutoHands.git
cd AutoHands

# Build
cargo build --release
```

### Configuration

Set your API key as an environment variable:

```bash
# For Anthropic Claude
export ANTHROPIC_API_KEY=your-api-key

# For Volcengine Ark
export ARK_API_KEY=your-api-key

# For OpenAI
export OPENAI_API_KEY=your-api-key
```

### Running

```bash
# Start the server
./target/release/autohands run

# Or run as a daemon
./target/release/autohands daemon start
```

### Submit a Task

```bash
curl -X POST http://127.0.0.1:8080/tasks \
  -H "Content-Type: application/json" \
  -d '{"task": "Check the latest emails in Gmail", "agent_id": "general"}'
```

## Project Structure

```
autohands/
├── crates/
│   ├── autohands-protocols/     # Protocol definitions (traits)
│   ├── autohands-core/          # Kernel, registries
│   ├── autohands-runtime/       # Agent runtime, agentic loop
│   ├── autohands-runloop/       # Event loop, task queue
│   ├── autohands-api/           # HTTP/WebSocket server
│   ├── autohands-daemon/        # Daemon management
│   └── extensions/
│       ├── tools-filesystem/    # File operations
│       ├── tools-shell/         # Shell commands
│       ├── tools-browser/       # Browser automation (CDP)
│       ├── tools-desktop/       # Desktop control (mouse, keyboard, OCR)
│       ├── tools-search/        # Glob, grep
│       ├── tools-web/           # HTTP fetch, web search
│       ├── tools-code/          # Code analysis
│       ├── provider-anthropic/  # Claude provider
│       ├── provider-openai/     # OpenAI provider
│       ├── provider-ark/        # Ark provider
│       ├── memory-sqlite/       # SQLite memory backend
│       ├── memory-vector/       # Vector memory backend
│       └── mcp-bridge/          # MCP protocol bridge
├── docs/
│   ├── ARCHITECTURE.md          # Architecture design
│   └── ROADMAP.md               # Development roadmap
└── skills/                      # Skill definitions (Markdown)
```

## Available Tools (46+)

| Category | Tools |
|----------|-------|
| **Filesystem** | read_file, write_file, edit_file, list_directory, create_directory, delete_file, move_file |
| **Shell** | exec, shell_session, background |
| **Browser** | browser_open, browser_navigate, browser_click, browser_type, browser_screenshot, browser_get_content, browser_execute_js, browser_ai_click, browser_ai_fill, browser_ai_extract, ... |
| **Desktop** | desktop_screenshot, desktop_mouse_move, desktop_mouse_click, desktop_keyboard_type, desktop_keyboard_hotkey, desktop_clipboard_get, desktop_clipboard_set, ... |
| **Search** | glob, grep |
| **Web** | web_fetch, web_search |
| **Code** | analyze_code, find_symbol |
| **Skills** | skill_list, skill_load, skill_read |

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/tasks` | Submit a task |
| GET | `/tasks/{id}` | Query task status |
| POST | `/webhook/{id}` | Trigger webhook |
| GET | `/ws` | WebSocket connection |

## Daemon Commands

```bash
# Start daemon
autohands daemon start

# Stop daemon
autohands daemon stop

# Check status
autohands daemon status

# Install as system service (macOS LaunchAgent / Linux Systemd)
autohands daemon install

# View logs
autohands daemon logs
```

## Skill Management

```bash
# List all skills
autohands skill list

# Show skill details
autohands skill info <skill-id>

# Create a new skill
autohands skill new my-skill

# Pack a skill
autohands skill pack ./my-skill-dir

# Install a skill package
autohands skill install ./my-skill.skill
```

## Development

```bash
# Run all tests
cargo test --workspace

# Run unit tests only (excluding integration tests)
cargo test --workspace --lib

# Lint
cargo clippy --workspace

# Format
cargo fmt --all
```

## Contributing

Contributions are welcome! Please read [ARCHITECTURE.md](docs/ARCHITECTURE.md) to understand the project architecture.

## License

This project is dual-licensed under MIT and Apache-2.0.

---

<div align="center">
Made with ❤️ by the AutoHands Contributors
</div>
