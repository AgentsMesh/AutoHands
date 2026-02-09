//! # AutoHands Core
//!
//! Microkernel implementation for the AutoHands framework.
//!
//! ## Components
//!
//! - [`Kernel`] - The microkernel managing extension lifecycle
//! - [`ExecutionContext`] - Context for tool/agent execution
//! - [`LifecycleManager`] - Lifecycle management for kernel components
//! - Registries for tools, providers, and extensions
//!
//! ## Task System
//!
//! Tasks are handled through the RunLoop task system. Extensions and tools
//! use the `TaskSubmitter` trait to submit tasks that flow through RunLoop.

pub mod context;
pub mod kernel;
pub mod lifecycle;
pub mod registry;

pub use context::ExecutionContext;
pub use kernel::Kernel;
pub use lifecycle::{
    KernelState, LifecycleHook, LifecycleManager, RunLoopControl, RunLoopLifecycleHook,
    ShutdownSignal,
};
pub use registry::{ChannelRegistry, ExtensionRegistry, ProviderRegistry, ToolRegistry};
