//! Extension and provider registration for AutoHands.

use std::path::PathBuf;
use std::sync::Arc;

use tracing::{error, info, warn};

use autohands_config::{Config, ConfigLoader};
use autohands_core::registry::{ProviderRegistry, ToolRegistry};
use autohands_provider_anthropic::AnthropicProvider;
use autohands_provider_gemini::GeminiProvider;
use autohands_provider_openai::OpenAIProvider;
use autohands_runtime::AgentRuntime;

// Memory extensions
use autohands_memory_sqlite::SqliteMemoryExtension;
use autohands_memory_markdown::MarkdownMemoryExtension;
use autohands_memory_vector::VectorMemoryExtension;
use autohands_memory_hybrid::HybridMemoryExtension;
use autohands_tools_memory::MemoryToolsExtension;

// Tool extensions
use autohands_tools_browser::BrowserToolsExtension;
use autohands_tools_code::{AnalyzeCodeTool, FindSymbolTool};
use autohands_tools_cron::CronToolsExtension;
use autohands_tools_desktop::DesktopToolsExtension;
use autohands_tools_filesystem::FilesystemExtension;
use autohands_tools_image::ImageToolsExtension;
use autohands_tools_notify::NotifyToolsExtension;
use autohands_tools_search::SearchExtension;
use autohands_tools_shell::ShellExtension;
use autohands_tools_skill::SkillToolsExtension;
use autohands_tools_web::WebToolsExtension;

// Agent extensions
use autohands_agent_general::GeneralAgent;
use autohands_tools_agent::AgentToolsExtension;

// Protocols for extension context
use autohands_protocols::agent::AgentConfig;
use autohands_protocols::extension::Extension;

// Skills progressive disclosure
use autohands_skills_dynamic::SkillMetadataInjector;

use crate::adapters::autohands_dir;
use crate::cmd_skill::create_skill_loader_for_server;

/// Register available tools and return (skill registry, optional memory backend, agent tools extension).
pub(crate) async fn register_tools_with_skill_registry(
    tool_registry: Arc<ToolRegistry>,
    provider_registry: Arc<ProviderRegistry>,
    work_dir: &PathBuf,
    config: &Config,
) -> (
    Arc<autohands_skills_dynamic::SkillRegistry>,
    Option<Arc<dyn autohands_protocols::memory::MemoryBackend>>,
    Option<AgentToolsExtension>,
) {
    use autohands_core::registry::MemoryRegistry;
    use autohands_protocols::extension::ExtensionContext;

    // Create extension context for initializing extensions
    // Note: task_submitter is None since we're not running within a RunLoop here
    let memory_registry = Arc::new(MemoryRegistry::new());

    let ctx = ExtensionContext::new(
        serde_json::Value::Null,
        None, // task_submitter - not needed for tool registration
        tool_registry.clone() as Arc<dyn autohands_protocols::extension::ToolRegistryAccess>,
        provider_registry.clone() as Arc<dyn autohands_protocols::extension::ProviderRegistryAccess>,
        memory_registry.clone() as Arc<dyn autohands_protocols::extension::MemoryRegistryAccess>,
        work_dir.clone(),
    );

    // Register Filesystem tools
    let mut fs_ext = FilesystemExtension::new();
    match fs_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = fs_ext.manifest().provides.tools.clone();
            info!("Registered filesystem tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize filesystem extension: {}", e);
        }
    }

    // Register Shell tools
    let mut shell_ext = ShellExtension::new();
    match shell_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = shell_ext.manifest().provides.tools.clone();
            info!("Registered shell tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize shell extension: {}", e);
        }
    }

    // Register Browser tools - Chrome will be auto-launched on first use
    // Profile persisted at ~/.autohands/browser-profile for login state
    let mut browser_ext = BrowserToolsExtension::new();
    match browser_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = browser_ext.manifest().provides.tools.clone();
            info!("Registered browser tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize browser extension: {}", e);
        }
    }

    // Register Desktop tools
    let mut desktop_ext = DesktopToolsExtension::new();
    match desktop_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = desktop_ext.manifest().provides.tools.clone();
            info!("Registered desktop tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize desktop extension: {}", e);
        }
    }

    // Register Search tools (glob, grep)
    let mut search_ext = SearchExtension::new();
    match search_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = search_ext.manifest().provides.tools.clone();
            info!("Registered search tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize search extension: {}", e);
        }
    }

    // Register Web tools (web_fetch, web_search)
    let mut web_ext = WebToolsExtension::new();
    match web_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = web_ext.manifest().provides.tools.clone();
            info!("Registered web tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize web extension: {}", e);
        }
    }

    // Register Code tools (analyze_code, find_symbol) - no Extension, register directly
    if let Err(e) = tool_registry.register(Arc::new(AnalyzeCodeTool::new())) {
        warn!("Failed to register analyze_code tool: {}", e);
    } else {
        info!("Registered analyze_code tool");
    }
    if let Err(e) = tool_registry.register(Arc::new(FindSymbolTool::new())) {
        warn!("Failed to register find_symbol tool: {}", e);
    } else {
        info!("Registered find_symbol tool");
    }

    // Register Memory backend based on config
    match config.memory.backend.as_str() {
        "sqlite" => {
            let path = config.memory.path.clone()
                .map(|p| {
                    let expanded = ConfigLoader::expand_path(&p.to_string_lossy());
                    PathBuf::from(expanded)
                })
                .unwrap_or_else(|| autohands_dir().join("memory.db"));
            let mut sqlite_ext = SqliteMemoryExtension::new().with_path(&path);
            match sqlite_ext.initialize(ctx.clone()).await {
                Ok(()) => {
                    info!("Registered SQLite memory backend (path={})", path.display());
                }
                Err(e) => {
                    warn!("Failed to initialize SQLite memory backend: {}", e);
                }
            }
        }
        "markdown" => {
            let path = config.memory.path.clone()
                .map(|p| {
                    let expanded = ConfigLoader::expand_path(&p.to_string_lossy());
                    PathBuf::from(expanded)
                })
                .unwrap_or_else(|| autohands_dir().join("memory"));
            let mut md_ext = MarkdownMemoryExtension::new().with_path(&path);
            match md_ext.initialize(ctx.clone()).await {
                Ok(()) => {
                    info!("Registered markdown memory backend (path={})", path.display());
                }
                Err(e) => {
                    warn!("Failed to initialize markdown memory backend: {}", e);
                }
            }
        }
        "vector" => {
            let mut vector_ext = VectorMemoryExtension::new();
            match vector_ext.initialize(ctx.clone()).await {
                Ok(()) => {
                    info!("Registered vector memory backend");
                }
                Err(e) => {
                    warn!("Failed to initialize vector memory backend: {}", e);
                }
            }
        }
        "hybrid" => {
            let mut hybrid_ext = HybridMemoryExtension::new();
            match hybrid_ext.initialize(ctx.clone()).await {
                Ok(()) => {
                    info!("Registered hybrid memory backend");
                }
                Err(e) => {
                    warn!("Failed to initialize hybrid memory backend: {}", e);
                }
            }
        }
        other => {
            info!("Using '{}' memory backend (no additional registration needed)", other);
        }
    }

    // Register Memory tools if a memory backend is available
    let memory_backend: Option<Arc<dyn autohands_protocols::memory::MemoryBackend>> = {
        let ids = memory_registry.list_ids();
        if let Some(first_id) = ids.first() {
            memory_registry.get(first_id)
        } else {
            None
        }
    };
    if let Some(ref backend) = memory_backend {
        let mut memory_tools_ext = MemoryToolsExtension::new(backend.clone());
        match memory_tools_ext.initialize(ctx.clone()).await {
            Ok(()) => {
                let tools = memory_tools_ext.manifest().provides.tools.clone();
                info!("Registered memory tools: {:?}", tools);
            }
            Err(e) => {
                warn!("Failed to initialize memory tools extension: {}", e);
            }
        }
    }

    // Register Cron tools
    let mut cron_ext = CronToolsExtension::new();
    match cron_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = cron_ext.manifest().provides.tools.clone();
            info!("Registered cron tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize cron tools extension: {}", e);
        }
    }

    // Register Notify tools
    let mut notify_ext = NotifyToolsExtension::new();
    match notify_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = notify_ext.manifest().provides.tools.clone();
            info!("Registered notify tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize notify tools extension: {}", e);
        }
    }

    // Register Image tools
    let mut image_ext = ImageToolsExtension::new();
    match image_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = image_ext.manifest().provides.tools.clone();
            info!("Registered image tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize image tools extension: {}", e);
        }
    }

    // Register Agent tools (agent_spawn, agent_status, agent_message, etc.)
    let agent_tools_ext = {
        let mut ext = AgentToolsExtension::new();
        match ext.initialize(ctx.clone()).await {
            Ok(()) => {
                info!("Registered agent tools: {:?}", ext.manifest().provides.tools);
                Some(ext)
            }
            Err(e) => {
                warn!("Failed to initialize agent tools extension: {}", e);
                None
            }
        }
    };

    // Create skill registry and loader
    let skill_registry = Arc::new(autohands_skills_dynamic::SkillRegistry::new());
    let skill_loader = create_skill_loader_for_server(work_dir).await;

    // Load skills into registry
    {
        use autohands_protocols::skill::SkillLoader;
        if let Ok(skill_defs) = skill_loader.list().await {
            for def in &skill_defs {
                if let Ok(skill) = skill_loader.load(&def.id).await {
                    skill_registry.register(skill).await;
                }
            }
            info!("Loaded {} skills into registry for progressive disclosure", skill_defs.len());
        }
    }

    // Load bundled skills into registry
    {
        use autohands_skills_bundled::BundledSkillLoader;
        use autohands_protocols::skill::SkillLoader as _;

        let bundled_loader = BundledSkillLoader::new();
        if let Ok(defs) = bundled_loader.list().await {
            for def in &defs {
                if let Ok(skill) = bundled_loader.load(&def.id).await {
                    skill_registry.register(skill).await;
                }
            }
            info!("Loaded {} bundled skills into registry", defs.len());
        }
    }

    // Register Skill tools with the loader
    let skill_loader: Arc<tokio::sync::RwLock<dyn autohands_protocols::skill::SkillLoader>> =
        Arc::new(tokio::sync::RwLock::new(skill_loader));

    let mut skill_ext = SkillToolsExtension::new(skill_loader);
    match skill_ext.initialize(ctx.clone()).await {
        Ok(()) => {
            let tools = skill_ext.manifest().provides.tools.clone();
            info!("Registered skill tools: {:?}", tools);
        }
        Err(e) => {
            warn!("Failed to initialize skill tools extension: {}", e);
        }
    }

    // Log total registered tools
    let total_tools = tool_registry.list().len();
    info!("Total registered tools: {}", total_tools);

    (skill_registry, memory_backend, agent_tools_ext)
}

/// Register available agents with skill metadata injected into system prompt.
pub(crate) async fn register_agents(
    agent_runtime: &AgentRuntime,
    provider_registry: Arc<ProviderRegistry>,
    tool_registry: Arc<ToolRegistry>,
    skill_registry: Arc<autohands_skills_dynamic::SkillRegistry>,
) {
    // Get first available provider for the default agent
    let provider_ids = provider_registry.list_ids();
    if provider_ids.is_empty() {
        warn!("No providers available, cannot create agents");
        return;
    }

    let default_provider_id = &provider_ids[0];
    let provider = match provider_registry.get(default_provider_id) {
        Some(p) => p,
        None => {
            error!("Default provider '{}' not found in registry", default_provider_id);
            return;
        }
    };

    // Use doubao-seed-1-8-251228 as the default model
    // Note: For Ark platform, you may need to use your endpoint ID instead
    let default_model = "doubao-seed-1-8-251228".to_string();

    // Collect all registered tools
    let tool_defs = tool_registry.list();
    let tools: Vec<Arc<dyn autohands_protocols::tool::Tool>> = tool_defs
        .iter()
        .filter_map(|def| tool_registry.get(&def.id))
        .collect();

    // Generate skill metadata section for system prompt (Progressive Disclosure L1)
    let skill_injector = SkillMetadataInjector::new(skill_registry.clone());
    let skill_section = skill_injector.generate_system_prompt_section().await;

    // Create general agent config with skill metadata in system prompt
    let mut agent_config = AgentConfig::new("general", "General Agent", &default_model);

    // Build system prompt with skill metadata
    let base_prompt = r#"You are AutoHands, an omnipotent autonomous agent capable of executing any task.

You have access to various tools for:
- File operations (read, write, edit, glob, grep)
- Shell commands
- Browser automation
- Desktop control (mouse, keyboard, screenshots, OCR)
- Web fetching and searching
- Code analysis
- Long-term memory (memory_search, memory_get, memory_store)

## Memory
You have long-term memory capabilities. When answering questions about past conversations,
user preferences, or historical decisions, use memory_search first to find relevant memories.
When you learn important information (user preferences, key decisions, facts, action items),
use memory_store to persist them for future reference.

Execute tasks efficiently and thoroughly."#;

    agent_config.system_prompt = if skill_section.is_empty() {
        Some(base_prompt.to_string())
    } else {
        Some(format!("{}\n{}", base_prompt, skill_section))
    };

    // Log skill injection status
    let skill_count = skill_registry.len().await;
    if skill_count > 0 {
        info!(
            "Injected {} skill(s) metadata into agent system prompt (Progressive Disclosure L1)",
            skill_count
        );
    }

    // Create and register general agent
    let general_agent = GeneralAgent::new(agent_config, provider.clone(), tools);
    agent_runtime.register_agent(Arc::new(general_agent));

    info!("Registered general agent with model: {}", default_model);
    info!("Total registered agents: {}", agent_runtime.list_agents().len());
}

/// Register available LLM providers based on config and environment variables.
pub(crate) async fn register_providers(registry: &ProviderRegistry, config: &Config) {
    // Iterate over configured providers, falling back to env vars for API keys
    for (name, provider_config) in &config.providers {
        let api_key = provider_config.api_key.clone()
            .or_else(|| std::env::var(format!("{}_API_KEY", name.to_uppercase())).ok());

        let Some(api_key) = api_key else {
            info!("Skipping provider '{}': no API key configured or in environment", name);
            continue;
        };

        match name.as_str() {
            "anthropic" => {
                let provider = AnthropicProvider::new(api_key);
                if let Err(e) = registry.register(Arc::new(provider)) {
                    warn!("Failed to register Anthropic provider: {}", e);
                } else {
                    info!("Registered Anthropic provider");
                }
            }
            "openai" => {
                let provider = if let Some(ref base_url) = provider_config.base_url {
                    OpenAIProvider::with_url(api_key, base_url.clone())
                } else {
                    OpenAIProvider::new(api_key)
                };
                if let Err(e) = registry.register(Arc::new(provider)) {
                    warn!("Failed to register OpenAI provider: {}", e);
                } else {
                    info!("Registered OpenAI provider");
                }
            }
            "gemini" => {
                let provider = GeminiProvider::new(api_key);
                if let Err(e) = registry.register(Arc::new(provider)) {
                    warn!("Failed to register Gemini provider: {}", e);
                } else {
                    info!("Registered Gemini provider");
                }
            }
            "ark" => {
                let provider = autohands_provider_ark::ArkProvider::new(api_key);
                if let Err(e) = registry.register(Arc::new(provider)) {
                    warn!("Failed to register Ark provider: {}", e);
                } else {
                    info!("Registered Ark provider (豆包)");
                }
            }
            other => {
                warn!("Unknown provider type: '{}', skipping", other);
            }
        }
    }

    // Fallback: register from env vars if no providers configured
    if config.providers.is_empty() {
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            let provider = AnthropicProvider::new(api_key);
            if let Err(e) = registry.register(Arc::new(provider)) {
                warn!("Failed to register Anthropic provider: {}", e);
            } else {
                info!("Registered Anthropic provider (from env)");
            }
        }
        if let Ok(api_key) = std::env::var("ARK_API_KEY") {
            let provider = autohands_provider_ark::ArkProvider::new(api_key);
            if let Err(e) = registry.register(Arc::new(provider)) {
                warn!("Failed to register Ark provider: {}", e);
            } else {
                info!("Registered Ark provider (from env)");
            }
        }
    }

    let provider_ids = registry.list_ids();
    if provider_ids.is_empty() {
        warn!("No LLM providers registered. Configure providers in config or set API key environment variables.");
    } else {
        info!("Registered providers: {:?}", provider_ids);
    }
}
