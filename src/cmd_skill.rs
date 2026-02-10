//! Skill subcommand handlers for AutoHands.

use std::path::PathBuf;

use tracing::{info, warn};

use autohands_skills_dynamic::{DynamicSkillLoader, SkillPackager, SkillSource};

use crate::cli::SkillAction;

/// Handle skill subcommands.
pub(crate) async fn handle_skill_command(action: SkillAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        SkillAction::List { tag, category, format } => {
            skill_list(tag, category, &format).await
        }
        SkillAction::Info { skill_id } => {
            skill_info(&skill_id).await
        }
        SkillAction::Reload => {
            skill_reload().await
        }
        SkillAction::Pack { skill_dir, output } => {
            skill_pack(&skill_dir, output.as_deref()).await
        }
        SkillAction::Install { skill_file, dir } => {
            skill_install(&skill_file, dir.as_deref()).await
        }
        SkillAction::New { skill_id, name, output } => {
            skill_new(&skill_id, name.as_deref(), output.as_deref()).await
        }
    }
}

/// List all available skills.
async fn skill_list(
    tag: Option<String>,
    category: Option<String>,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let loader = create_skill_loader().await;
    loader.load_all().await?;

    use autohands_protocols::skill::SkillLoader;
    let skills = loader.list().await?;

    // Filter by tag or category
    let filtered: Vec<_> = skills
        .into_iter()
        .filter(|s| {
            if let Some(ref t) = tag {
                if !s.tags.contains(t) {
                    return false;
                }
            }
            if let Some(ref c) = category {
                if s.category.as_ref() != Some(c) {
                    return false;
                }
            }
            true
        })
        .collect();

    if filtered.is_empty() {
        println!("No skills found.");
        return Ok(());
    }

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&filtered)?;
            println!("{}", json);
        }
        _ => {
            // Table format
            println!("{:<20} {:<30} {:<15} {}", "ID", "NAME", "CATEGORY", "TAGS");
            println!("{}", "-".repeat(80));
            for skill in filtered {
                let category = skill.category.as_deref().unwrap_or("-");
                let tags = skill.tags.join(", ");
                println!("{:<20} {:<30} {:<15} {}", skill.id, skill.name, category, tags);
            }
        }
    }

    Ok(())
}

/// Show detailed info about a skill.
async fn skill_info(skill_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let loader = create_skill_loader().await;
    loader.load_all().await?;

    use autohands_protocols::skill::SkillLoader;
    let skill = loader.load(skill_id).await?;

    println!("Skill: {}", skill.definition.name);
    println!("{}", "=".repeat(50));
    println!("ID:          {}", skill.definition.id);
    println!("Description: {}", skill.definition.description);
    if let Some(cat) = &skill.definition.category {
        println!("Category:    {}", cat);
    }
    if !skill.definition.tags.is_empty() {
        println!("Tags:        {}", skill.definition.tags.join(", "));
    }
    println!("Priority:    {}", skill.definition.priority);
    println!("Enabled:     {}", skill.definition.enabled);

    if !skill.definition.required_tools.is_empty() {
        println!("Required Tools: {}", skill.definition.required_tools.join(", "));
    }

    if !skill.definition.variables.is_empty() {
        println!("\nVariables:");
        for var in &skill.definition.variables {
            let required = if var.required { " (required)" } else { "" };
            let default = var.default.as_ref().map(|d| format!(" [default: {}]", d)).unwrap_or_default();
            println!("  - {}: {}{}{}", var.name, var.description, required, default);
        }
    }

    println!("\nContent Preview:");
    println!("{}", "-".repeat(50));
    // Show first 500 chars
    let preview: String = skill.content.chars().take(500).collect();
    println!("{}", preview);
    if skill.content.len() > 500 {
        println!("... ({} more characters)", skill.content.len() - 500);
    }

    Ok(())
}

/// Reload all skills.
async fn skill_reload() -> Result<(), Box<dyn std::error::Error>> {
    let loader = create_skill_loader().await;

    use autohands_protocols::skill::SkillLoader;
    loader.reload().await?;

    let skills = loader.list().await?;
    println!("Reloaded {} skills", skills.len());

    Ok(())
}

/// Pack a skill directory.
async fn skill_pack(
    skill_dir: &PathBuf,
    output: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = output
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    let package_path = SkillPackager::pack(skill_dir, &output_dir)?;
    println!("Created skill package: {}", package_path.display());

    Ok(())
}

/// Install a skill package.
async fn skill_install(
    skill_file: &PathBuf,
    dir: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let skills_dir = dir
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join(".autohands").join("skills"))
                .unwrap_or_else(|| PathBuf::from("./skills"))
        });

    // Ensure directory exists
    std::fs::create_dir_all(&skills_dir)?;

    let installed_path = SkillPackager::install(skill_file, &skills_dir)?;
    println!("Installed skill to: {}", installed_path.display());

    Ok(())
}

/// Create a new skill from template.
async fn skill_new(
    skill_id: &str,
    name: Option<&str>,
    output: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let skills_dir = output
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join(".autohands").join("skills"))
                .unwrap_or_else(|| PathBuf::from("./skills"))
        });

    // Ensure directory exists
    std::fs::create_dir_all(&skills_dir)?;

    let skill_name = name.unwrap_or(skill_id);
    let skill_path = skills_dir.join(format!("{}.markdown", skill_id));

    if skill_path.exists() {
        return Err(format!("Skill already exists: {}", skill_path.display()).into());
    }

    let template = format!(
        r#"---
id: {}
name: {}
version: 1.0.0
description: Description of your skill

requires:
  tools: []
  bins: []

tags: []
category: general
priority: 10

variables: []
---

# {}

Your skill prompt content here.

## Instructions

1. Step one
2. Step two
3. Step three

## Guidelines

- Guideline one
- Guideline two
"#,
        skill_id, skill_name, skill_name
    );

    std::fs::write(&skill_path, template)?;
    println!("Created new skill: {}", skill_path.display());
    println!("\nEdit the file to customize your skill.");

    Ok(())
}

/// Create a skill loader with default configuration (for CLI commands).
async fn create_skill_loader() -> DynamicSkillLoader {
    let mut loader = DynamicSkillLoader::new();

    // Add workspace directory if exists
    if let Ok(cwd) = std::env::current_dir() {
        let workspace = cwd.join("skills");
        if workspace.exists() {
            loader = loader.with_source(SkillSource::Workspace(workspace));
        }
    }

    loader
}

/// Create a skill loader for the server with all skills loaded.
pub(crate) async fn create_skill_loader_for_server(work_dir: &PathBuf) -> DynamicSkillLoader {
    let mut loader = DynamicSkillLoader::new();

    // Add workspace directory if exists
    let workspace = work_dir.join("skills");
    if workspace.exists() {
        loader = loader.with_source(SkillSource::Workspace(workspace));
    }

    // Load all skills
    if let Err(e) = loader.load_all().await {
        warn!("Failed to load skills: {}", e);
    } else {
        use autohands_protocols::skill::SkillLoader;
        match loader.list().await {
            Ok(skills) => {
                info!("Loaded {} skills for Agent use", skills.len());
                for skill in &skills {
                    info!("  - {}: {}", skill.id, skill.name);
                }
            }
            Err(e) => warn!("Failed to list skills: {}", e),
        }
    }

    loader
}
