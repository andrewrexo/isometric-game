//! Quest System Module
//!
//! Hybrid quest system using TOML for metadata and Lua for complex logic.
//! Features hot-reloadable scripts, quest chains, and branching dialogue.

pub mod definition;
pub mod registry;
pub mod state;
pub mod events;
pub mod runner;
pub mod api;

pub use definition::{Quest, Objective, ObjectiveType, Reward, QuestChain, QuestDialogue};
pub use registry::{QuestRegistry, HotReloadEvent};
pub use state::{PlayerQuestState, QuestProgress, QuestStatus};
pub use events::QuestEvent;
pub use runner::{QuestRunner, ScriptResult, DialogueResult, DialogueChoice, BonusReward};
pub use api::QuestContext;
