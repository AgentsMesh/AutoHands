//! Dynamic skills loading extension for AutoHands.
//!
//! Provides runtime loading of skills from the filesystem with hot-reload support.
//!
//! # Features
//!
//! - **Multi-level loading**: Skills are loaded from multiple sources with priority ordering
//!   (bundled < managed < workspace)
//! - **SKILL.markdown format**: Simple markdown-based skill definition with YAML frontmatter
//! - **Hot-reload**: File system watching for automatic skill refresh
//! - **Dependency detection**: Automatic binary and tool availability checking
//! - **Package format**: `.skill` single-file distribution format
//! - **Progressive disclosure**: Claude Code-style 3-level skill disclosure

mod extension;
mod loader;
mod package;
mod progressive;
mod registry;

pub use extension::DynamicSkillsExtension;
pub use loader::{DynamicSkillLoader, SkillSource};
pub use package::{SkillPackage, SkillPackager};
pub use progressive::SkillMetadataInjector;
pub use registry::SkillRegistry;

/// Re-export common types from protocols.
pub use autohands_protocols::skill::{Skill, SkillDefinition, SkillVariable};
