# AutoHands Architecture Design Document

> Omnipotent Autonomous Agent Framework - Rust Implementation

## 1. Project Vision

AutoHands is a highly extensible omnipotent AI Agent framework with the following core design principles:

1. **Minimal Core** - Core only defines protocols and interfaces, not concrete implementations
2. **Everything is an Extension** - Tools, skills, channels, and providers are all extensions
3. **Protocol First** - Extension contracts defined via traits (not inheritance)
4. **Self-Describing** - Extensions carry metadata, supporting auto-discovery and documentation generation
5. **Hot-Pluggable** - Runtime loading/unloading of extensions (via dylib or WASM)
6. **Progressive Disclosure** - From zero configuration to full customization, as needed

## 2. System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Gateway Layer                                   │
│         HTTP API (OpenAI Compatible) │ WebSocket │ Channel Adapters         │
├─────────────────────────────────────────────────────────────────────────────┤
│                              Task Queue                                      │
│                    (Priority Queue & Delayed Tasks)                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                              Core Runtime                                    │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│  │   Kernel    │ │  Extension  │ │   Session   │ │   Config    │           │
│  │ (Microkernel)│ │  Registry   │ │   Manager   │ │   Manager   │           │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘           │
├─────────────────────────────────────────────────────────────────────────────┤
│                              Protocol Layer (Traits)                         │
│  ┌─────────┐ ┌──────────┐ ┌─────────┐ ┌────────┐ ┌───────┐ ┌───────────┐   │
│  │  Tool   │ │ Provider │ │ Channel │ │ Memory │ │ Agent │ │   Skill   │   │
│  └─────────┘ └──────────┘ └─────────┘ └────────┘ └───────┘ └───────────┘   │
├─────────────────────────────────────────────────────────────────────────────┤
│                              Extension Layer                                 │
│  ┌─────────────────────┐ ┌─────────────────────┐ ┌─────────────────────┐   │
│  │   Builtin (Static)  │ │   Dynamic (dylib)   │ │    WASM (Sandbox)   │   │
│  │ tools-*, provider-* │ │   User Extensions   │ │  Cross-lang Plugins │   │
│  └─────────────────────┘ └─────────────────────┘ └─────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 3. Directory Structure

```
autohands/
├── Cargo.toml                    # Workspace configuration
├── Cargo.lock
├── README.md
├── LICENSE
│
├── docs/                         # Documentation
│   ├── ARCHITECTURE.md           # Architecture design (this file)
│   ├── ROADMAP.md               # Development roadmap
│   ├── EXTENSION_GUIDE.md       # Extension development guide
│   └── API.md                   # API documentation
│
├── crates/                       # Core crates
│   ├── autohands-core/          # Microkernel, task queue, context
│   ├── autohands-protocols/     # Protocol definitions (traits)
│   ├── autohands-runtime/       # Agent runtime
│   ├── autohands-api/           # API server
│   ├── autohands-runloop/       # Event loop, task scheduling
│   ├── autohands-daemon/        # Daemon management
│   ├── autohands-config/        # Configuration management
│   ├── autohands-macros/        # Procedural macros
│   │
│   └── extensions/              # Built-in extensions
│       ├── tools-filesystem/    # Filesystem tools
│       ├── tools-shell/         # Shell execution tools
│       ├── tools-browser/       # Browser automation tools
│       ├── tools-desktop/       # Desktop control tools
│       ├── tools-code/          # Code editing tools
│       ├── tools-search/        # Search tools (grep, glob)
│       ├── tools-web/           # Web tools (fetch, search)
│       ├── provider-anthropic/  # Anthropic Provider
│       ├── provider-openai/     # OpenAI Provider
│       ├── provider-gemini/     # Gemini Provider
│       ├── provider-ark/        # Volcengine Ark Provider
│       ├── memory-sqlite/       # SQLite memory backend
│       ├── memory-vector/       # Vector memory backend
│       ├── channel-webhook/     # Webhook channel
│       ├── agent-general/       # General Agent
│       ├── skills-bundled/      # Built-in skills
│       ├── skills-dynamic/      # Dynamic skill loader
│       └── mcp-bridge/          # MCP protocol bridge
│
├── src/                         # Main program entry
│   └── main.rs
│
├── config/                      # Default configuration
│   └── default.toml
│
├── skills/                      # Skill definitions (Markdown)
│   ├── bundled/                # Built-in skills
│   └── examples/               # Example skills
│
└── tests/                       # Integration tests
    └── integration/
```

## 4. Core Module Design

### 4.1 autohands-protocols (Protocol Layer)

Defines all traits that extensions must implement. **Interfaces only, no implementations**.

```rust
// Core trait list
pub trait Extension        // Extension base interface
pub trait Tool             // Tool interface
pub trait LLMProvider      // LLM provider interface
pub trait Channel          // Message channel interface
pub trait MemoryBackend    // Memory backend interface
pub trait Agent            // Agent interface
pub trait SkillLoader      // Skill loader interface
```

### 4.2 autohands-core (Core Layer)

Microkernel implementation, responsible for extension lifecycle management and component communication.

```rust
pub struct Kernel          // Microkernel
pub struct ExecutionContext // Execution context
pub struct ExtensionRegistry // Extension registry
pub struct ToolRegistry    // Tool registry
pub struct ProviderRegistry // Provider registry
pub struct ChannelRegistry // Channel registry
```

### 4.3 autohands-runtime (Runtime Layer)

Agent execution runtime, implements the Agentic Loop.

```rust
pub struct AgentRuntime    // Agent runtime
pub struct AgentLoop       // Agentic loop
pub struct SessionManager  // Session management
pub struct HistoryManager  // History management
pub struct ContextBuilder  // Context builder
```

### 4.4 autohands-runloop (RunLoop Layer)

Event-driven task scheduling and execution.

```rust
pub struct RunLoop         // Event loop
pub struct TaskQueue       // Task queue
pub struct Task            // Task definition
pub struct AgentDriver     // Agent execution driver
pub trait Source0          // Polling event source
pub trait Source1          // Async event source
pub trait Observer         // Event observer
```

### 4.5 autohands-api (API Layer)

HTTP/WebSocket server, exposes external APIs.

```rust
pub struct InterfaceServer // API server
pub struct HttpHandler     // HTTP handler
pub struct WsHandler       // WebSocket handler
pub struct RunLoopBridge   // RunLoop bridge
```

### 4.6 autohands-config (Configuration Layer)

Configuration loading, validation, hot-reloading.

```rust
pub struct ConfigManager   // Configuration manager
pub struct ConfigSchema    // Configuration schema
pub struct ConfigWatcher   // Configuration watcher (hot-reload)
```

### 4.7 autohands-macros (Macros)

Procedural macros to simplify extension development.

```rust
#[extension]               // Extension definition macro
#[tool]                    // Tool definition macro
#[provider]                // Provider definition macro
```

## 5. Key Design Decisions

### 5.1 Extension Loading Strategy

| Type | Loading Method | Use Case |
|------|---------------|----------|
| **Builtin** | Static compilation | Core extensions, best performance |
| **Dynamic** | dylib dynamic loading | User-defined Rust extensions |
| **WASM** | wasmtime runtime | Cross-language extensions, sandboxed |

Initially only Builtin is implemented; Dynamic and WASM support added as needed.

### 5.2 Error Handling Strategy

Use `thiserror` for error type definitions, `anyhow` for application-level error propagation.

```rust
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Extension not found: {0}")]
    ExtensionNotFound(String),

    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),

    // ...
}
```

### 5.3 Async Runtime

Uses `tokio` as the async runtime; all I/O operations are async.

### 5.4 Serialization

Uses `serde` for serialization; configuration uses `toml`, API uses `json`.

### 5.5 Logging and Tracing

Uses `tracing` for structured logging and distributed tracing.

## 6. Data Flow

### 6.1 User Request Processing Flow

```
User Request
    │
    ▼
┌─────────────┐
│   Gateway   │  ← HTTP/WebSocket/Channel
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ RunLoop     │  ← Convert to Task, enqueue
│ Bridge      │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  TaskQueue  │  ← Priority scheduling
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Agent     │  ← Select appropriate Agent
│   Driver    │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Agentic   │  ← Execute loop
│    Loop     │
└──────┬──────┘
       │
       ├──────────────┐
       │              │
       ▼              ▼
┌─────────────┐ ┌─────────────┐
│     LLM     │ │    Tool     │
│   Provider  │ │  Executor   │
└─────────────┘ └─────────────┘
```

### 6.2 Agentic Loop

```
while !finished {
    1. Build context (history + tools + skills)
    2. Call LLM
    3. Parse response
       ├── Text output → Stream return
       ├── Tool call → Execute tool → Add result to history
       └── Completion signal → Exit loop
    4. Check termination conditions (max_turns, timeout, abort)
    5. Optional: History compression
}
```

## 7. Extension Protocol Details

### 7.1 Extension Manifest

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<Author>,
    pub dependencies: Dependencies,
    pub provides: Provides,
    pub config_schema: Option<JsonSchema>,
    pub permissions: Vec<Permission>,
}
```

### 7.2 Tool Protocol

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn definition(&self) -> &ToolDefinition;
    async fn execute(&self, params: Value, ctx: ToolContext) -> Result<ToolResult, ToolError>;
    fn validate(&self, params: &Value) -> Result<(), ValidationError> { Ok(()) }
}
```

### 7.3 Provider Protocol

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn id(&self) -> &str;
    fn models(&self) -> &[ModelDefinition];
    fn capabilities(&self) -> &ProviderCapabilities;
    async fn complete_stream(&self, request: CompletionRequest) -> Result<CompletionStream, ProviderError>;
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ProviderError>;
}
```

### 7.4 Channel Protocol

```rust
#[async_trait]
pub trait Channel: Send + Sync {
    fn id(&self) -> &str;
    fn capabilities(&self) -> &ChannelCapabilities;
    async fn connect(&mut self) -> Result<(), ChannelError>;
    async fn disconnect(&mut self) -> Result<(), ChannelError>;
    async fn send(&self, target: &MessageTarget, message: OutgoingMessage) -> Result<SentMessage, ChannelError>;
    fn on_message(&self) -> broadcast::Receiver<IncomingMessage>;
}
```

### 7.5 Memory Protocol

```rust
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    async fn store(&self, entry: MemoryEntry) -> Result<String, MemoryError>;
    async fn retrieve(&self, id: &str) -> Result<Option<MemoryEntry>, MemoryError>;
    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemorySearchResult>, MemoryError>;
    async fn delete(&self, id: &str) -> Result<(), MemoryError>;
}
```

## 8. Configuration Design

### 8.1 Configuration File Structure

```toml
# config.toml

[server]
host = "127.0.0.1"
port = 8080

[agent]
default = "general"
max_turns = 50
timeout_seconds = 300

[providers.anthropic]
api_key = "${ANTHROPIC_API_KEY}"
default_model = "claude-sonnet-4-20250514"

[providers.openai]
api_key = "${OPENAI_API_KEY}"
default_model = "gpt-4o"

[memory]
backend = "sqlite"
path = "~/.autohands/memory.db"

[extensions]
# Static extensions controlled via feature flags
# Dynamic extensions configured here
paths = ["~/.autohands/extensions"]

[channels.telegram]
enabled = true
token = "${TELEGRAM_BOT_TOKEN}"

[skills]
paths = ["~/.autohands/skills", "./skills"]
enabled = ["coding", "research", "writing"]
```

### 8.2 Environment Variable Support

Configuration values support `${VAR}` syntax to reference environment variables.

## 9. Security Design

### 9.1 Permission Model

```rust
pub enum Permission {
    FileSystem { paths: Vec<PathPattern>, read: bool, write: bool },
    Network { hosts: Vec<HostPattern> },
    Shell { commands: Vec<CommandPattern> },
    Environment { variables: Vec<String> },
}
```

### 9.2 Execution Approval

High-risk operations require user confirmation:

```rust
pub enum RiskLevel {
    Low,      // Auto-execute
    Medium,   // Configurable confirmation
    High,     // Must confirm
}
```

### 9.3 Sandbox Isolation

WASM extensions run in wasmtime sandbox with restricted resource access.

## 10. Performance Considerations

### 10.1 Compilation Optimization

```toml
# Cargo.toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### 10.2 Async Optimization

- Use `tokio` multi-threaded runtime
- Use `Arc<RwLock<T>>` appropriately to reduce lock contention
- Stream processing to avoid memory spikes

### 10.3 Memory Optimization

- Use `bytes` crate for large data handling
- Release unneeded resources promptly
- Consider using `jemalloc` instead of system allocator

## 11. Testing Strategy

### 11.1 Unit Tests

Module tests within each crate.

### 11.2 Integration Tests

End-to-end tests in `tests/integration/` directory.

### 11.3 Performance Tests

Benchmark tests using `criterion`.

## 12. Documentation and Examples

### 12.1 API Documentation

Generate Rust documentation using `cargo doc`.

### 12.2 Extension Development Guide

`docs/EXTENSION_GUIDE.md` explains how to develop extensions in detail.

### 12.3 Example Extensions

Each extension under `crates/extensions/` serves as an example.

---

*Document Version: 0.1.0*
*Last Updated: 2026-02-09*
