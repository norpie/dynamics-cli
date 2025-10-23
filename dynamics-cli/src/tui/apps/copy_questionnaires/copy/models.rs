use serde_json::Value;
use crate::tui::Resource;

#[derive(Clone)]
pub struct State {
    pub questionnaire_id: String,
    pub questionnaire_name: String,
    pub snapshot: Resource<QuestionnaireSnapshot>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            questionnaire_id: String::new(),
            questionnaire_name: String::new(),
            snapshot: Resource::NotAsked,
        }
    }
}

/// Complete snapshot of a questionnaire and all related entities
#[derive(Clone, Debug)]
pub struct QuestionnaireSnapshot {
    // Core entities
    pub questionnaire: Value,
    pub pages: Vec<Value>,
    pub page_lines: Vec<Value>,
    pub groups: Vec<Value>,
    pub group_lines: Vec<Value>,
    pub questions: Vec<Value>,
    pub template_lines: Vec<Value>,

    // Logic entities
    pub conditions: Vec<Value>,
    pub condition_actions: Vec<Value>,

    // N:N relationship IDs (store the related entity IDs, not junction records)
    pub categories: Vec<String>,
    pub domains: Vec<String>,
    pub funds: Vec<String>,
    pub supports: Vec<String>,
    pub types: Vec<String>,
    pub subcategories: Vec<String>,
    pub flemish_shares: Vec<String>,
}

impl QuestionnaireSnapshot {
    /// Count total entities in the snapshot
    pub fn total_entities(&self) -> usize {
        1 + // questionnaire
        self.pages.len() +
        self.page_lines.len() +
        self.groups.len() +
        self.group_lines.len() +
        self.questions.len() +
        self.template_lines.len() +
        self.conditions.len() +
        self.condition_actions.len() +
        self.categories.len() +
        self.domains.len() +
        self.funds.len() +
        self.supports.len() +
        self.types.len() +
        self.subcategories.len() +
        self.flemish_shares.len()
    }
}

#[derive(Clone)]
pub enum Msg {
    SnapshotLoaded(Result<QuestionnaireSnapshot, String>),
    Back,
    StartCopy, // Placeholder for future functionality
}

pub struct CopyQuestionnaireParams {
    pub questionnaire_id: String,
    pub questionnaire_name: String,
}

impl Default for CopyQuestionnaireParams {
    fn default() -> Self {
        Self {
            questionnaire_id: String::new(),
            questionnaire_name: String::new(),
        }
    }
}
