# AutoHands 开发路线图

## 总览

项目分为 4 个主要阶段，预计总开发周期 12-16 周。

```
Phase 1: 核心框架 (4-5 周)
    ↓
Phase 2: 基础扩展 (3-4 周)
    ↓
Phase 3: Agent 运行时 (3-4 周)
    ↓
Phase 4: 生态完善 (2-3 周)
```

---

## Phase 1: 核心框架 (4-5 周)

### 目标
搭建可扩展的核心框架，定义所有协议，实现微内核。

### 任务清单

#### 1.1 项目初始化
- [x] 创建项目目录结构
- [x] 创建架构文档
- [x] 初始化 Cargo workspace
- [ ] 配置 CI/CD (GitHub Actions)
- [ ] 配置代码质量工具 (clippy, rustfmt)

#### 1.2 autohands-protocols (协议层)
- [x] Extension trait 定义
- [x] Tool trait 定义
- [x] LLMProvider trait 定义
- [x] Channel trait 定义
- [x] MemoryBackend trait 定义
- [x] Agent trait 定义
- [x] Skill 相关类型定义
- [x] 公共类型定义 (Message, ToolResult, etc.)
- [x] 错误类型定义

#### 1.3 autohands-core (核心层)
- [x] EventBus 实现
  - [x] 事件订阅/发布
  - [ ] 中间件支持
  - [ ] 请求-响应模式
- [x] ExecutionContext 实现
  - [x] 上下文数据存储
  - [x] 中止信号
  - [x] 子上下文创建
- [x] ExtensionRegistry 实现
  - [x] 扩展注册/注销
  - [x] 依赖解析
- [x] ToolRegistry 实现
- [x] ProviderRegistry 实现
- [x] Kernel 实现
  - [x] 扩展生命周期管理
  - [x] 扩展上下文创建
  - [ ] 启动/关闭流程完善

#### 1.4 autohands-config (配置层)
- [x] 配置 Schema 定义
- [x] TOML 配置解析
- [x] 环境变量替换
- [ ] 配置验证
- [ ] 配置热重载 (可选，Phase 4)

#### 1.5 单元测试
- [ ] protocols 测试
- [ ] core 测试
- [ ] config 测试

### 交付物
- [x] 可编译的核心框架
- [x] 完整的协议定义
- [ ] 单元测试覆盖

---

## Phase 2: 基础扩展 (3-4 周)

### 目标
实现基础工具和 Provider，验证框架设计。

### 任务清单

#### 2.1 autohands-macros (过程宏)
- [ ] `#[extension]` 宏
- [ ] `#[tool]` 宏
- [ ] 宏测试

#### 2.2 tools-filesystem (文件系统工具)
- [x] read_file 工具
- [x] write_file 工具
- [x] edit_file 工具 (SEARCH/REPLACE)
- [x] list_directory 工具
- [ ] create_directory 工具
- [ ] delete_file 工具
- [ ] move_file 工具
- [ ] 测试

#### 2.3 tools-shell (Shell 工具)
- [x] exec 工具 (命令执行)
- [ ] 持久化 Shell 会话
- [ ] 后台进程管理
- [x] 超时控制
- [ ] 测试

#### 2.4 tools-search (搜索工具)
- [ ] glob 工具 (文件模式匹配)
- [ ] grep 工具 (内容搜索)
- [ ] 集成 ripgrep (可选)
- [ ] 测试

#### 2.5 provider-anthropic (Anthropic Provider)
- [x] API 客户端实现
- [x] 流式补全
- [x] Function Calling 支持
- [ ] 错误处理和重试
- [ ] 测试

#### 2.6 provider-openai (OpenAI Provider)
- [ ] API 客户端实现
- [ ] 流式补全
- [ ] Function Calling 支持
- [ ] 测试

#### 2.7 memory-sqlite (SQLite 记忆后端)
- [ ] 数据库 Schema
- [ ] CRUD 操作
- [ ] 基础搜索
- [ ] 测试

### 交付物
- [x] 5+ 可用工具 (read_file, write_file, edit_file, list_directory, exec)
- [x] 1 个 LLM Provider (Anthropic)
- [ ] 1 个记忆后端
- [x] 扩展开发示例

---

## Phase 3: Agent 运行时 (3-4 周)

### 目标
实现完整的 Agent 运行时和 Gateway。

### 任务清单

#### 3.1 autohands-runtime (运行时)
- [x] SessionManager 实现
  - [x] 会话创建/获取
  - [ ] 会话持久化
  - [ ] 会话清理
- [x] HistoryManager 实现
  - [x] 消息历史管理
  - [ ] 历史压缩 (摘要)
- [ ] ContextBuilder 实现
  - [ ] 系统提示构建
  - [ ] 工具注入
  - [ ] 技能注入
- [x] AgentLoop 实现
  - [x] 主循环逻辑
  - [x] 工具执行
  - [ ] 流式响应
  - [ ] 错误处理和重试
  - [x] 终止条件检查
- [ ] AgentRuntime 实现
  - [ ] Agent 调度
  - [ ] 并发控制

#### 3.2 autohands-gateway (网关)
- [x] HTTP 服务器 (axum)
  - [ ] OpenAI 兼容 API
  - [x] 健康检查端点
  - [ ] 管理端点
- [ ] WebSocket 服务器
  - [ ] 连接管理
  - [ ] 消息协议
  - [ ] 心跳机制
- [ ] Gateway 主逻辑
  - [ ] 请求路由
  - [ ] 会话关联

#### 3.3 skills-bundled (内置技能)
- [ ] 技能加载器实现
- [ ] Markdown 解析器
- [ ] 内置技能
  - [ ] coding.md
  - [ ] research.md
  - [ ] writing.md
- [ ] 技能注入逻辑

#### 3.4 agent-general (通用 Agent)
- [ ] 通用 Agent 实现
- [ ] 工具选择逻辑
- [ ] 测试

#### 3.5 集成测试
- [ ] 端到端测试
- [ ] Gateway API 测试
- [ ] Agent 执行测试

### 交付物
- [ ] 完整可运行的 Agent 系统
- [ ] HTTP/WebSocket API
- [ ] 通用 Agent
- [ ] 内置技能

---

## Phase 4: 生态完善 (2-3 周)

### 目标
完善生态，提高可用性和扩展性。

### 任务清单

#### 4.1 更多 Provider
- [ ] provider-gemini (Google Gemini)
- [ ] provider-local (本地模型/Ollama)

#### 4.2 更多工具
- [ ] tools-web
  - [ ] web_fetch 工具
  - [ ] web_search 工具 (可选)
- [ ] tools-code
  - [ ] 代码分析工具
  - [ ] LSP 集成 (可选)

#### 4.3 渠道支持
- [ ] channel-webhook (Webhook 渠道)
- [ ] channel-telegram (Telegram 渠道，可选)

#### 4.4 MCP 支持
- [ ] mcp-bridge
  - [ ] MCP 协议实现 (JSON-RPC)
  - [ ] stdio 传输
  - [ ] HTTP/SSE 传输
  - [ ] 工具发现和注册

#### 4.5 向量记忆
- [ ] memory-vector
  - [ ] 嵌入生成
  - [ ] 向量存储 (sqlite-vec 或 qdrant)
  - [ ] 混合搜索

#### 4.6 文档和示例
- [ ] API 文档完善
- [ ] 扩展开发指南
- [ ] 示例项目

#### 4.7 性能优化
- [ ] 基准测试
- [ ] 性能分析和优化
- [ ] 内存优化

### 交付物
- [ ] 4+ Provider
- [ ] 10+ 工具
- [ ] MCP 支持
- [ ] 完整文档

---

## 里程碑

| 里程碑 | 目标日期 | 内容 |
|-------|---------|------|
| M1: 核心框架 | Week 4-5 | 协议定义、微内核、配置 |
| M2: 基础可用 | Week 7-9 | 基础工具、Provider、Agent |
| M3: 功能完整 | Week 10-12 | Gateway、技能、渠道 |
| M4: 生产就绪 | Week 12-16 | MCP、优化、文档 |

---

## 风险和缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| Rust 编译时间长 | 开发效率 | 使用增量编译、mold 链接器 |
| MCP SDK 缺失 | 开发周期 | 自行实现协议 |
| 扩展系统复杂 | 延期 | 初期只实现静态扩展 |
| LLM API 变化 | 兼容性 | 抽象层隔离 |

---

## 开发原则

1. **测试驱动** - 先写测试，再写实现
2. **文档同步** - 代码和文档同时更新
3. **小步迭代** - 每个 PR 聚焦单一功能
4. **代码审查** - 所有代码需要 review
5. **性能意识** - 关注性能，避免过早优化

---

*最后更新: 2026-02-07*
