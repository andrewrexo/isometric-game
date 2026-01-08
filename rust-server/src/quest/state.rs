//! Quest State Tracking
//!
//! Tracks player quest progress, status, and persistence.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Status of a quest for a player
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestStatus {
    /// Quest is available but not started
    Available,
    /// Quest is active and in progress
    Active,
    /// All objectives complete, ready to turn in
    ReadyToComplete,
    /// Quest has been completed
    Completed,
    /// Quest was failed
    Failed,
    /// Quest was abandoned by player
    Abandoned,
}

impl QuestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            QuestStatus::Available => "available",
            QuestStatus::Active => "active",
            QuestStatus::ReadyToComplete => "ready_to_complete",
            QuestStatus::Completed => "completed",
            QuestStatus::Failed => "failed",
            QuestStatus::Abandoned => "abandoned",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "available" => Some(QuestStatus::Available),
            "active" => Some(QuestStatus::Active),
            "ready_to_complete" => Some(QuestStatus::ReadyToComplete),
            "completed" => Some(QuestStatus::Completed),
            "failed" => Some(QuestStatus::Failed),
            "abandoned" => Some(QuestStatus::Abandoned),
            _ => None,
        }
    }
}

/// Progress on a single objective
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveProgress {
    pub objective_id: String,
    pub current: i32,
    pub target: i32,
    pub completed: bool,
}

impl ObjectiveProgress {
    pub fn new(objective_id: &str, target: i32) -> Self {
        Self {
            objective_id: objective_id.to_string(),
            current: 0,
            target,
            completed: false,
        }
    }

    /// Add progress and return true if newly completed
    pub fn add_progress(&mut self, amount: i32) -> bool {
        if self.completed {
            return false;
        }
        self.current = (self.current + amount).min(self.target);
        if self.current >= self.target {
            self.completed = true;
            true
        } else {
            false
        }
    }

    /// Set progress directly
    pub fn set_progress(&mut self, amount: i32) {
        self.current = amount.min(self.target);
        self.completed = self.current >= self.target;
    }

    /// Mark as complete regardless of count
    pub fn force_complete(&mut self) {
        self.current = self.target;
        self.completed = true;
    }

    pub fn progress_percent(&self) -> f32 {
        if self.target == 0 {
            return 1.0;
        }
        self.current as f32 / self.target as f32
    }
}

/// Complete quest progress for a player
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestProgress {
    pub quest_id: String,
    pub status: QuestStatus,
    /// Progress on each objective (keyed by objective_id)
    pub objectives: HashMap<String, ObjectiveProgress>,
    /// When the quest was started
    pub started_at: Option<DateTime<Utc>>,
    /// When the quest was completed
    pub completed_at: Option<DateTime<Utc>>,
}

impl QuestProgress {
    pub fn new(quest_id: &str, objective_targets: &[(String, i32)]) -> Self {
        let objectives = objective_targets
            .iter()
            .map(|(id, target)| (id.clone(), ObjectiveProgress::new(id, *target)))
            .collect();

        Self {
            quest_id: quest_id.to_string(),
            status: QuestStatus::Active,
            objectives,
            started_at: Some(Utc::now()),
            completed_at: None,
        }
    }

    /// Update objective progress
    pub fn update_objective(&mut self, objective_id: &str, amount: i32) -> bool {
        if let Some(obj) = self.objectives.get_mut(objective_id) {
            let newly_completed = obj.add_progress(amount);
            self.check_all_complete();
            newly_completed
        } else {
            false
        }
    }

    /// Check if all objectives are complete and update status
    fn check_all_complete(&mut self) {
        if self.status != QuestStatus::Active {
            return;
        }
        if self.objectives.values().all(|o| o.completed) {
            self.status = QuestStatus::ReadyToComplete;
        }
    }

    /// Mark quest as completed
    pub fn complete(&mut self) {
        self.status = QuestStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    /// Mark quest as failed
    pub fn fail(&mut self) {
        self.status = QuestStatus::Failed;
        self.completed_at = Some(Utc::now());
    }

    /// Mark quest as abandoned
    pub fn abandon(&mut self) {
        self.status = QuestStatus::Abandoned;
        self.completed_at = Some(Utc::now());
    }

    /// Get duration in seconds (if started)
    pub fn duration_secs(&self) -> Option<i64> {
        self.started_at.map(|start| {
            let end = self.completed_at.unwrap_or_else(Utc::now);
            (end - start).num_seconds()
        })
    }

    /// Check if quest is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self.status, QuestStatus::Completed | QuestStatus::Failed | QuestStatus::Abandoned)
    }

    /// Serialize objectives to JSON for database storage
    pub fn objectives_to_json(&self) -> String {
        serde_json::to_string(&self.objectives).unwrap_or_else(|_| "{}".to_string())
    }

    /// Deserialize objectives from JSON
    pub fn objectives_from_json(json: &str) -> HashMap<String, ObjectiveProgress> {
        serde_json::from_str(json).unwrap_or_default()
    }
}

/// All quest state for a single player
#[derive(Debug, Clone, Default)]
pub struct PlayerQuestState {
    /// Active quests (quest_id -> progress)
    pub active_quests: HashMap<String, QuestProgress>,
    /// Completed quest IDs
    pub completed_quests: Vec<String>,
    /// Available quest IDs (unlocked but not started)
    pub available_quests: Vec<String>,
    /// Persistent flags for quest state (dialogue choices, world state)
    pub flags: HashMap<String, String>,
}

impl PlayerQuestState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new quest
    pub fn start_quest(&mut self, quest_id: &str, objective_targets: &[(String, i32)]) {
        // Remove from available
        self.available_quests.retain(|id| id != quest_id);

        // Add to active
        let progress = QuestProgress::new(quest_id, objective_targets);
        self.active_quests.insert(quest_id.to_string(), progress);
    }

    /// Get active quest progress
    pub fn get_quest(&self, quest_id: &str) -> Option<&QuestProgress> {
        self.active_quests.get(quest_id)
    }

    /// Get mutable active quest progress
    pub fn get_quest_mut(&mut self, quest_id: &str) -> Option<&mut QuestProgress> {
        self.active_quests.get_mut(quest_id)
    }

    /// Complete a quest
    pub fn complete_quest(&mut self, quest_id: &str) {
        if let Some(progress) = self.active_quests.get_mut(quest_id) {
            progress.complete();
        }
        // Move to completed
        if let Some(progress) = self.active_quests.remove(quest_id) {
            if progress.status == QuestStatus::Completed {
                self.completed_quests.push(quest_id.to_string());
            }
        }
    }

    /// Abandon a quest
    pub fn abandon_quest(&mut self, quest_id: &str) {
        if let Some(mut progress) = self.active_quests.remove(quest_id) {
            progress.abandon();
        }
    }

    /// Check if a quest is completed
    pub fn is_quest_completed(&self, quest_id: &str) -> bool {
        self.completed_quests.contains(&quest_id.to_string())
    }

    /// Check if a quest is available
    pub fn is_quest_available(&self, quest_id: &str) -> bool {
        self.available_quests.contains(&quest_id.to_string())
    }

    /// Check if a quest is active
    pub fn is_quest_active(&self, quest_id: &str) -> bool {
        self.active_quests.contains_key(quest_id)
    }

    /// Make a quest available
    pub fn unlock_quest(&mut self, quest_id: &str) {
        if !self.is_quest_completed(quest_id)
            && !self.is_quest_active(quest_id)
            && !self.is_quest_available(quest_id)
        {
            self.available_quests.push(quest_id.to_string());
        }
    }

    /// Get a persistent flag
    pub fn get_flag(&self, flag_name: &str) -> Option<&String> {
        self.flags.get(flag_name)
    }

    /// Set a persistent flag
    pub fn set_flag(&mut self, flag_name: &str, value: &str) {
        self.flags.insert(flag_name.to_string(), value.to_string());
    }

    /// Get all active quests that are waiting for a specific event type
    pub fn get_quests_for_target(&self, target: &str, objective_type: super::definition::ObjectiveType) -> Vec<(&str, &str)> {
        // This would need the Quest definitions to check, so return quest_id + objective_id pairs
        // The caller should filter by checking the Quest definitions
        self.active_quests
            .values()
            .flat_map(|progress| {
                progress.objectives
                    .iter()
                    .filter(|(_, obj)| !obj.completed)
                    .map(|(obj_id, _)| (progress.quest_id.as_str(), obj_id.as_str()))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_objective_progress() {
        let mut obj = ObjectiveProgress::new("kill_slimes", 5);
        assert!(!obj.completed);
        assert_eq!(obj.current, 0);

        obj.add_progress(3);
        assert!(!obj.completed);
        assert_eq!(obj.current, 3);

        let completed = obj.add_progress(2);
        assert!(completed);
        assert!(obj.completed);
        assert_eq!(obj.current, 5);

        // Can't add more after complete
        let completed = obj.add_progress(1);
        assert!(!completed);
        assert_eq!(obj.current, 5);
    }

    #[test]
    fn test_quest_progress() {
        let targets = vec![
            ("kill_slimes".to_string(), 5),
            ("collect_cores".to_string(), 3),
        ];
        let mut progress = QuestProgress::new("first_hunt", &targets);

        assert_eq!(progress.status, QuestStatus::Active);

        // Complete first objective
        progress.update_objective("kill_slimes", 5);
        assert_eq!(progress.status, QuestStatus::Active);

        // Complete second objective
        progress.update_objective("collect_cores", 3);
        assert_eq!(progress.status, QuestStatus::ReadyToComplete);
    }

    #[test]
    fn test_player_quest_state() {
        let mut state = PlayerQuestState::new();

        state.start_quest("quest1", &[("obj1".to_string(), 3)]);
        assert!(state.is_quest_active("quest1"));

        state.complete_quest("quest1");
        assert!(state.is_quest_completed("quest1"));
        assert!(!state.is_quest_active("quest1"));
    }
}
