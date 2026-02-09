# AutoHands 架构设计文档

> 全能自主工作智能体框架 - Rust 实现

## 一、项目愿景

AutoHands 是一个高度可扩展的全能 AI Agent 框架，核心设计理念：

1. **核心极简** - 核心只定义协议和接口，不实现具体能力
2. **一切皆扩展** - 工具、技能、渠道、Provider 都是扩展
3. **协议优先** - 通过 trait（而非继承）定义扩展契约
4. **自描述** - 扩展自带元数据，支持自动发现和文档生成
5. **热插拔** - 运行时加载/卸载扩展（通过 dylib 或 WASM）
6. **渐进式披露** - 从零配置到完全定制，按需深入

## 二、系统架构

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Gateway Layer                                   │
│         HTTP API (OpenAI Compatible) │ WebSocket │ Channel Adapters         │
├─────────────────────────────────────────────────────────────────────────────┤
│                              Event Bus                                       │
│                    (Async Message Passing & Pub/Sub)                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                              Core Runtime                                    │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│  │   Kernel    │ │  Extension  │ │   Session   │ │   Config    │           │
│  │  (微内核)   │ │  Registry   │ │   Manager   │ │   Manager   │           │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘           │
├─────────────────────────────────────────────────────────────────────────────┤
│                              Protocol Layer (Traits)                         │
│  ┌─────────┐ ┌──────────┐ ┌─────────┐ ┌────────┐ ┌───────┐ ┌───────────┐   │
│  │  Tool   │ │ Provider │ │ Channel │ │ Memory │ │ Agent │ │   Skill   │   │
│  └─────────┘ └──────────┘ └─────────┘ └────────┘ └───────┘ └───────────┘   │
├─────────────────────────────────────────────────────────────────────────────┤
│                              Extension Layer                                 │
│  ┌─────────────────────┐ ┌─────────────────────┐ ┌─────────────────────┐   │
│  │   Builtin (静态)    │ │   Dynamic (dylib)   │ │    WASM (沙箱)      │   │
│  │ tools-*, provider-* │ │   用户自定义扩展     │ │   跨语言扩展        │   │
│  └─────────────────────┘ └─────────────────────┘ └─────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 三、目录结构

```
autohands/
├── Cargo.toml                    # Workspace 配置
├── Cargo.lock
├── README.md
├── LICENSE
│
├── docs/                         # 文档
│   ├── ARCHITECTURE.md           # 架构设计 (本文件)
│   ├── ROADMAP.md               # 开发路线图
│   ├── EXTENSION_GUIDE.md       # 扩展开发指南
│   └── API.md                   # API 文档
│
├── crates/                       # 核心 crates
│   ├── autohands-core/          # 微内核、事件总线、上下文
│   ├── autohands-protocols/     # 协议定义 (traits)
│   ├── autohands-runtime/       # Agent 运行时
│   ├── autohands-gateway/       # Gateway 服务器
│   ├── autohands-config/        # 配置管理
│   ├── autohands-macros/        # 过程宏 (简化扩展开发)
│   │
│   └── extensions/              # 内置扩展
│       ├── tools-filesystem/    # 文件系统工具
│       ├── tools-shell/         # Shell 执行工具
│       ├── tools-code/          # 代码编辑工具
│       ├── tools-search/        # 搜索工具 (grep, glob)
│       ├── tools-web/           # Web 工具 (fetch, search)
│       ├── provider-anthropic/  # Anthropic Provider
│       ├── provider-openai/     # OpenAI Provider
│       ├── provider-gemini/     # Gemini Provider
│       ├── memory-sqlite/       # SQLite 记忆后端
│       ├── memory-vector/       # 向量记忆后端
│       ├── channel-webhook/     # Webhook 渠道
│       ├── channel-telegram/    # Telegram 渠道
│       ├── agent-general/       # 通用 Agent
│       ├── agent-coder/         # 编码 Agent
│       ├── skills-bundled/      # 内置技能
│       └── mcp-bridge/          # MCP 协议桥接
│
├── src/                         # 主程序入口
│   └── main.rs
│
├── config/                      # 默认配置
│   └── default.toml
│
├── skills/                      # 技能定义 (Markdown)
│   ├── bundled/                # 内置技能
│   └── examples/               # 示例技能
│
└── tests/                       # 集成测试
    └── integration/
```

## 四、核心模块设计

### 4.1 autohands-protocols (协议层)

定义所有扩展必须实现的 trait，**只有接口，没有实现**。

```rust
// 核心 trait 列表
pub trait Extension        // 扩展基础接口
pub trait Tool             // 工具接口
pub trait LLMProvider      // LLM 提供者接口
pub trait Channel          // 消息渠道接口
pub trait MemoryBackend    // 记忆后端接口
pub trait Agent            // Agent 接口
pub trait SkillLoader      // 技能加载器接口
```

### 4.2 autohands-core (核心层)

微内核实现，负责扩展生命周期管理和组件通信。

```rust
pub struct Kernel          // 微内核
pub struct EventBus        // 事件总线
pub struct ExecutionContext // 执行上下文
pub struct ExtensionRegistry // 扩展注册表
pub struct ToolRegistry    // 工具注册表
pub struct ProviderRegistry // Provider 注册表
pub struct ChannelRegistry // 渠道注册表
```

### 4.3 autohands-runtime (运行时层)

Agent 执行运行时，实现 Agentic Loop。

```rust
pub struct AgentRuntime    // Agent 运行时
pub struct AgentLoop       // Agentic 循环
pub struct SessionManager  // 会话管理
pub struct HistoryManager  // 历史管理
pub struct ContextBuilder  // 上下文构建器
```

### 4.4 autohands-gateway (网关层)

HTTP/WebSocket 服务器，对外暴露 API。

```rust
pub struct GatewayServer   // Gateway 服务器
pub struct HttpHandler     // HTTP 处理器
pub struct WsHandler       // WebSocket 处理器
pub struct ChannelManager  // 渠道管理器
```

### 4.5 autohands-config (配置层)

配置加载、验证、热重载。

```rust
pub struct ConfigManager   // 配置管理器
pub struct ConfigSchema    // 配置 Schema
pub struct ConfigWatcher   // 配置监听器 (热重载)
```

### 4.6 autohands-macros (宏)

过程宏，简化扩展开发。

```rust
#[extension]               // 扩展定义宏
#[tool]                    // 工具定义宏
#[provider]                // Provider 定义宏
```

## 五、关键设计决策

### 5.1 扩展加载策略

| 类型 | 加载方式 | 适用场景 |
|------|---------|---------|
| **Builtin** | 静态编译 | 核心扩展，性能最优 |
| **Dynamic** | dylib 动态加载 | 用户自定义 Rust 扩展 |
| **WASM** | wasmtime 运行 | 跨语言扩展，沙箱隔离 |

初期只实现 Builtin，后续根据需要添加 Dynamic 和 WASM 支持。

### 5.2 错误处理策略

使用 `thiserror` 定义错误类型，`anyhow` 用于应用层错误传播。

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

### 5.3 异步运行时

使用 `tokio` 作为异步运行时，所有 I/O 操作都是异步的。

### 5.4 序列化

使用 `serde` 进行序列化，配置使用 `toml`，API 使用 `json`。

### 5.5 日志和追踪

使用 `tracing` 进行结构化日志和分布式追踪。

## 六、数据流

### 6.1 用户请求处理流程

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
│  EventBus   │  ← 发布 "request:received" 事件
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Session   │  ← 获取/创建会话
│   Manager   │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│    Agent    │  ← 选择合适的 Agent
│   Runtime   │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Agentic   │  ← 执行循环
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
    1. 构建上下文 (历史 + 工具 + 技能)
    2. 调用 LLM
    3. 解析响应
       ├── 文本输出 → 流式返回
       ├── 工具调用 → 执行工具 → 结果加入历史
       └── 完成信号 → 退出循环
    4. 检查终止条件 (max_turns, timeout, abort)
    5. 可选: 历史压缩
}
```

## 七、扩展协议详细设计

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

## 八、配置设计

### 8.1 配置文件结构

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
# 静态扩展通过 feature flags 控制
# 动态扩展在这里配置
paths = ["~/.autohands/extensions"]

[channels.telegram]
enabled = true
token = "${TELEGRAM_BOT_TOKEN}"

[skills]
paths = ["~/.autohands/skills", "./skills"]
enabled = ["coding", "research", "writing"]
```

### 8.2 环境变量支持

配置值支持 `${VAR}` 语法引用环境变量。

## 九、安全设计

### 9.1 权限模型

```rust
pub enum Permission {
    FileSystem { paths: Vec<PathPattern>, read: bool, write: bool },
    Network { hosts: Vec<HostPattern> },
    Shell { commands: Vec<CommandPattern> },
    Environment { variables: Vec<String> },
}
```

### 9.2 执行审批

高风险操作需要用户确认：

```rust
pub enum RiskLevel {
    Low,      // 自动执行
    Medium,   // 可配置是否确认
    High,     // 必须确认
}
```

### 9.3 沙箱隔离

WASM 扩展运行在 wasmtime 沙箱中，资源访问受限。

## 十、性能考量

### 10.1 编译优化

```toml
# Cargo.toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### 10.2 异步优化

- 使用 `tokio` 多线程运行时
- 合理使用 `Arc<RwLock<T>>` 减少锁竞争
- 流式处理避免内存峰值

### 10.3 内存优化

- 使用 `bytes` crate 处理大数据
- 及时释放不需要的资源
- 考虑使用 `jemalloc` 替代系统分配器

## 十一、测试策略

### 11.1 单元测试

每个 crate 内部的模块测试。

### 11.2 集成测试

`tests/integration/` 目录下的端到端测试。

### 11.3 性能测试

使用 `criterion` 进行基准测试。

## 十二、文档和示例

### 12.1 API 文档

使用 `cargo doc` 生成 Rust 文档。

### 12.2 扩展开发指南

`docs/EXTENSION_GUIDE.md` 详细说明如何开发扩展。

### 12.3 示例扩展

`crates/extensions/` 下的每个扩展都是示例。

---

*文档版本: 0.1.0*
*最后更新: 2026-02-07*
