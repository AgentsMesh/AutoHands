# AutoHands Development Roadmap

## Overview

The project is divided into 4 main phases, with an estimated total development cycle of 12-16 weeks.

```
Phase 1: Core Framework (4-5 weeks)
    ↓
Phase 2: Basic Extensions (3-4 weeks)
    ↓
Phase 3: Agent Runtime (3-4 weeks)
    ↓
Phase 4: Ecosystem Enhancement (2-3 weeks)
```

---

## Phase 1: Core Framework (4-5 weeks)

### Goal
Build an extensible core framework, define all protocols, and implement the microkernel.

### Task List

#### 1.1 Project Initialization
- [x] Create project directory structure
- [x] Create architecture documentation
- [x] Initialize Cargo workspace
- [ ] Configure CI/CD (GitHub Actions)
- [x] Configure code quality tools (clippy, rustfmt)

#### 1.2 autohands-protocols (Protocol Layer)
- [x] Extension trait definition
- [x] Tool trait definition
- [x] LLMProvider trait definition
- [x] Channel trait definition
- [x] MemoryBackend trait definition
- [x] Agent trait definition
- [x] Skill-related type definitions
- [x] Common type definitions (Message, ToolResult, etc.)
- [x] Error type definitions

#### 1.3 autohands-core (Core Layer)
- [x] TaskQueue implementation
  - [x] Task subscribe/publish
  - [ ] Middleware support
  - [ ] Request-response pattern
- [x] ExecutionContext implementation
  - [x] Context data storage
  - [x] Abort signal
  - [x] Sub-context creation
- [x] ExtensionRegistry implementation
  - [x] Extension registration/unregistration
  - [x] Dependency resolution
- [x] ToolRegistry implementation
- [x] ProviderRegistry implementation
- [x] Kernel implementation
  - [x] Extension lifecycle management
  - [x] Extension context creation
  - [ ] Startup/shutdown process refinement

#### 1.4 autohands-config (Configuration Layer)
- [x] Configuration schema definition
- [x] TOML configuration parsing
- [x] Environment variable substitution
- [ ] Configuration validation
- [ ] Configuration hot-reload (optional, Phase 4)

#### 1.5 Unit Tests
- [x] protocols tests
- [x] core tests
- [x] config tests

### Deliverables
- [x] Compilable core framework
- [x] Complete protocol definitions
- [x] Unit test coverage

---

## Phase 2: Basic Extensions (3-4 weeks)

### Goal
Implement basic tools and providers to validate the framework design.

### Task List

#### 2.1 autohands-macros (Procedural Macros)
- [x] `#[extension]` macro
- [ ] `#[tool]` macro
- [x] Macro tests

#### 2.2 tools-filesystem (Filesystem Tools)
- [x] read_file tool
- [x] write_file tool
- [x] edit_file tool (SEARCH/REPLACE)
- [x] list_directory tool
- [x] create_directory tool
- [x] delete_file tool
- [x] move_file tool
- [x] Tests

#### 2.3 tools-shell (Shell Tools)
- [x] exec tool (command execution)
- [x] Persistent shell session
- [x] Background process management
- [x] Timeout control
- [x] Tests

#### 2.4 tools-search (Search Tools)
- [x] glob tool (file pattern matching)
- [x] grep tool (content search)
- [x] Integrated ripgrep
- [x] Tests

#### 2.5 provider-anthropic (Anthropic Provider)
- [x] API client implementation
- [x] Streaming completion
- [x] Function calling support
- [x] Error handling and retry
- [x] Tests

#### 2.6 provider-openai (OpenAI Provider)
- [x] API client implementation
- [x] Streaming completion
- [x] Function calling support
- [x] Tests

#### 2.7 provider-ark (Volcengine Ark Provider)
- [x] API client implementation
- [x] Streaming completion
- [x] Function calling support
- [x] Tests

#### 2.8 memory-sqlite (SQLite Memory Backend)
- [x] Database schema
- [x] CRUD operations
- [x] Basic search
- [x] Tests

### Deliverables
- [x] 7+ available tools (filesystem, shell, search)
- [x] 3 LLM Providers (Anthropic, OpenAI, Ark)
- [x] 1 memory backend
- [x] Extension development examples

---

## Phase 3: Agent Runtime (3-4 weeks)

### Goal
Implement complete Agent runtime and API gateway.

### Task List

#### 3.1 autohands-runtime (Runtime)
- [x] SessionManager implementation
  - [x] Session creation/retrieval
  - [x] Session persistence
  - [ ] Session cleanup
- [x] HistoryManager implementation
  - [x] Message history management
  - [x] History compression (summarization)
- [x] ContextBuilder implementation
  - [x] System prompt construction
  - [x] Tool injection
  - [x] Skill injection
- [x] AgentLoop implementation
  - [x] Main loop logic
  - [x] Tool execution
  - [x] Streaming response
  - [x] Error handling and retry
  - [x] Termination condition checking
- [x] AgentRuntime implementation
  - [x] Agent scheduling
  - [x] Concurrency control

#### 3.2 autohands-api (API Server)
- [x] HTTP server (axum)
  - [ ] OpenAI compatible API
  - [x] Health check endpoint
  - [x] Admin endpoints
- [x] WebSocket server
  - [x] Connection management
  - [x] Message protocol
  - [ ] Heartbeat mechanism
- [x] RunLoopBridge
  - [x] Request routing
  - [x] Session association

#### 3.3 autohands-runloop (RunLoop)
- [x] TaskQueue implementation
- [x] RunLoop implementation
- [x] Source0/Source1 patterns
- [x] Observer pattern
- [x] Timer support
- [x] CronTimer support

#### 3.4 skills-dynamic (Dynamic Skills)
- [x] Skill loader implementation
- [x] Markdown parser
- [x] Progressive disclosure (L1/L2/L3)
- [x] Multiple format adapters (Claude Code, Microsoft, OpenClaw)
- [x] Skill injection logic

#### 3.5 agent-general (General Agent)
- [x] General agent implementation
- [x] Tool selection logic
- [x] Tests

#### 3.6 Integration Tests
- [x] End-to-end tests
- [x] API tests
- [x] Agent execution tests

### Deliverables
- [x] Complete runnable Agent system
- [x] HTTP/WebSocket API
- [x] General Agent
- [x] Dynamic skills

---

## Phase 4: Ecosystem Enhancement (2-3 weeks)

### Goal
Enhance ecosystem, improve usability and extensibility.

### Task List

#### 4.1 More Providers
- [x] provider-gemini (Google Gemini)
- [ ] provider-local (Local models/Ollama)

#### 4.2 More Tools
- [x] tools-web
  - [x] web_fetch tool
  - [x] web_search tool
- [x] tools-code
  - [x] Code analysis tools
  - [ ] LSP integration (optional)
- [x] tools-browser
  - [x] Browser automation (CDP)
  - [x] AI-powered click/fill/extract
- [x] tools-desktop
  - [x] Screenshot
  - [x] Mouse control
  - [x] Keyboard control
  - [x] OCR

#### 4.3 Channel Support
- [ ] channel-telegram (Telegram channel, optional)

#### 4.4 MCP Support
- [ ] MCP protocol bridge (planned)

#### 4.5 Vector Memory
- [x] memory-vector
  - [x] Embedding generation
  - [x] Vector storage
  - [ ] Hybrid search
- [x] memory-hybrid
  - [x] Full-text search
  - [x] Vector search
  - [x] Fusion ranking

#### 4.6 Daemon Support
- [x] autohands-daemon
  - [x] Process daemonization
  - [x] PID file management
  - [x] Signal handling
  - [x] macOS LaunchAgent support
  - [x] Linux Systemd support

#### 4.7 Documentation and Examples
- [x] API documentation
- [x] Architecture documentation
- [ ] Extension development guide
- [ ] Example projects

#### 4.8 Performance Optimization
- [ ] Benchmark tests
- [ ] Performance analysis and optimization
- [ ] Memory optimization

### Deliverables
- [x] 4 Providers
- [x] 46+ Tools
- [x] MCP support
- [x] Daemon support
- [ ] Complete documentation

---

## Milestones

| Milestone | Target Date | Content |
|-----------|-------------|---------|
| M1: Core Framework | Week 4-5 | Protocol definitions, microkernel, configuration |
| M2: Basic Usable | Week 7-9 | Basic tools, providers, agent |
| M3: Feature Complete | Week 10-12 | API server, skills, channels |
| M4: Production Ready | Week 12-16 | MCP, optimization, documentation |

---

## Risks and Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Long Rust compilation time | Development efficiency | Use incremental compilation, mold linker |
| Missing MCP SDK | Development cycle | Implement protocol ourselves |
| Complex extension system | Delay | Initially only implement static extensions |
| LLM API changes | Compatibility | Abstraction layer isolation |

---

## Development Principles

1. **Test-Driven** - Write tests first, then implementation
2. **Documentation in Sync** - Update code and documentation together
3. **Small Iterations** - Each PR focuses on a single feature
4. **Code Review** - All code requires review
5. **Performance Awareness** - Focus on performance, avoid premature optimization

---

*Last Updated: 2026-02-09*
