//! Command modules for Tauri IPC

// Module declarations - these are kept private since we re-export their contents
mod ai;
mod anchor;
mod doc;
mod export;
mod graph;
mod plugin;
mod repo;
mod scan;
mod search;
mod settings;

// Re-export all items from each module (including Tauri-generated __cmd__ items)
pub use ai::*;
pub use anchor::*;
pub use doc::*;
pub use export::*;
pub use graph::*;
pub use plugin::*;
pub use repo::*;
pub use scan::*;
pub use search::*;
pub use settings::*;
