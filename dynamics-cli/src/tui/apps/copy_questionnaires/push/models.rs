use std::time::Instant;
use std::collections::HashMap;
use super::super::copy::domain::Questionnaire;

#[derive(Clone)]
pub struct State {
    pub questionnaire_id: String,
    pub copy_name: String,
    pub copy_code: String,
    pub questionnaire: Questionnaire,  // Already loaded from copy screen
    pub push_state: PushState,
}

impl Default for State {
    fn default() -> Self {
        Self {
            questionnaire_id: String::new(),
            copy_name: String::new(),
            copy_code: String::new(),
            questionnaire: Questionnaire {
                id: String::new(),
                name: String::new(),
                raw: serde_json::Value::Null,
                pages: vec![],
                page_lines: vec![],
                group_lines: vec![],
                template_lines: vec![],
                conditions: vec![],
                classifications: super::super::copy::domain::Classifications {
                    categories: vec![],
                    domains: vec![],
                    funds: vec![],
                    supports: vec![],
                    types: vec![],
                    subcategories: vec![],
                    flemish_shares: vec![],
                },
            },
            push_state: PushState::Confirming,
        }
    }
}

/// State machine for the push/copy process
#[derive(Clone)]
pub enum PushState {
    /// Screen 1: Confirmation - show summary and wait for user to start
    Confirming,

    /// Screen 2: Copy in progress - show real-time progress
    Copying(CopyProgress),

    /// Screen 3a: Success - show results
    Success(CopyResult),

    /// Screen 3b: Failure - show error and partial progress
    Failed(CopyError),
}

/// Progress tracking for the copy operation
#[derive(Clone)]
pub struct CopyProgress {
    pub phase: CopyPhase,
    pub step: usize,  // 1-10 (10 steps total)

    // Per-entity counts (done, total)
    pub questionnaire: (usize, usize),
    pub pages: (usize, usize),
    pub page_lines: (usize, usize),
    pub groups: (usize, usize),
    pub group_lines: (usize, usize),
    pub questions: (usize, usize),
    pub template_lines: (usize, usize),
    pub conditions: (usize, usize),
    pub condition_actions: (usize, usize),
    pub classifications: (usize, usize),

    // Overall progress
    pub total_created: usize,
    pub total_entities: usize,
    pub started_at: Instant,
}

impl CopyProgress {
    pub fn new(questionnaire: &Questionnaire) -> Self {
        let pages_count = questionnaire.pages.len();
        let groups_count: usize = questionnaire.pages.iter().map(|p| p.groups.len()).sum();
        let questions_count: usize = questionnaire.pages.iter()
            .flat_map(|p| &p.groups)
            .map(|g| g.questions.len())
            .sum();
        let page_lines_count = questionnaire.page_lines.len();
        let group_lines_count = questionnaire.group_lines.len();
        let template_lines_count = questionnaire.template_lines.len();
        let conditions_count = questionnaire.conditions.len();
        let condition_actions_count: usize = questionnaire.conditions.iter()
            .map(|c| c.actions.len())
            .sum();
        let classifications_count =
            questionnaire.classifications.categories.len() +
            questionnaire.classifications.domains.len() +
            questionnaire.classifications.funds.len() +
            questionnaire.classifications.supports.len() +
            questionnaire.classifications.types.len() +
            questionnaire.classifications.subcategories.len() +
            questionnaire.classifications.flemish_shares.len();

        let total_entities = questionnaire.total_entities();

        Self {
            phase: CopyPhase::CreatingQuestionnaire,
            step: 1,
            questionnaire: (0, 1),
            pages: (0, pages_count),
            page_lines: (0, page_lines_count),
            groups: (0, groups_count),
            group_lines: (0, group_lines_count),
            questions: (0, questions_count),
            template_lines: (0, template_lines_count),
            conditions: (0, conditions_count),
            condition_actions: (0, condition_actions_count),
            classifications: (0, classifications_count),
            total_created: 0,
            total_entities,
            started_at: Instant::now(),
        }
    }

    /// Calculate overall percentage
    pub fn percentage(&self) -> usize {
        if self.total_entities == 0 {
            0
        } else {
            ((self.total_created as f64 / self.total_entities as f64) * 100.0) as usize
        }
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }
}

/// Phases of the copy operation
#[derive(Clone, Debug, PartialEq)]
pub enum CopyPhase {
    CreatingQuestionnaire,    // Step 1
    CreatingPages,            // Step 2
    CreatingPageLines,        // Step 3
    CreatingGroups,           // Step 4
    CreatingGroupLines,       // Step 5
    CreatingQuestions,        // Step 6
    CreatingTemplateLines,    // Step 7
    CreatingConditions,       // Step 8
    CreatingConditionActions, // Step 9
    CreatingClassifications,  // Step 10
}

impl CopyPhase {
    pub fn step_number(&self) -> usize {
        match self {
            CopyPhase::CreatingQuestionnaire => 1,
            CopyPhase::CreatingPages => 2,
            CopyPhase::CreatingPageLines => 3,
            CopyPhase::CreatingGroups => 4,
            CopyPhase::CreatingGroupLines => 5,
            CopyPhase::CreatingQuestions => 6,
            CopyPhase::CreatingTemplateLines => 7,
            CopyPhase::CreatingConditions => 8,
            CopyPhase::CreatingConditionActions => 9,
            CopyPhase::CreatingClassifications => 10,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            CopyPhase::CreatingQuestionnaire => "Creating Questionnaire",
            CopyPhase::CreatingPages => "Creating Pages",
            CopyPhase::CreatingPageLines => "Creating Page Lines",
            CopyPhase::CreatingGroups => "Creating Groups",
            CopyPhase::CreatingGroupLines => "Creating Group Lines",
            CopyPhase::CreatingQuestions => "Creating Questions",
            CopyPhase::CreatingTemplateLines => "Creating Template Lines",
            CopyPhase::CreatingConditions => "Creating Conditions",
            CopyPhase::CreatingConditionActions => "Creating Condition Actions",
            CopyPhase::CreatingClassifications => "Creating Classifications",
        }
    }
}

/// Result of a successful copy
#[derive(Clone)]
pub struct CopyResult {
    pub new_questionnaire_id: String,
    pub new_questionnaire_name: String,
    pub entities_created: HashMap<String, usize>,  // entity_type -> count
    pub total_entities: usize,
    pub duration: std::time::Duration,
}

/// Error during copy operation
#[derive(Clone)]
pub struct CopyError {
    pub phase: CopyPhase,
    pub step: usize,
    pub error_message: String,
    pub partial_counts: HashMap<String, usize>,
    pub rollback_complete: bool,
}

#[derive(Clone)]
pub enum Msg {
    // Screen 1: Confirmation
    StartCopy,
    Cancel,

    // Screen 2: Progress (from async task)
    CopyProgressUpdate(CopyProgress),

    // Screen 3: Results
    CopySuccess(CopyResult),
    CopyFailed(CopyError),

    // Actions
    ViewCopy,
    CopyAnother,
    Retry,
    ViewLogs,
    Done,
    Back,
}

pub struct PushQuestionnaireParams {
    pub questionnaire_id: String,
    pub copy_name: String,
    pub copy_code: String,
    pub questionnaire: Questionnaire,  // Pass the already-loaded questionnaire
}

impl Default for PushQuestionnaireParams {
    fn default() -> Self {
        Self {
            questionnaire_id: String::new(),
            copy_name: String::new(),
            copy_code: String::new(),
            questionnaire: Questionnaire {
                id: String::new(),
                name: String::new(),
                raw: serde_json::Value::Null,
                pages: vec![],
                page_lines: vec![],
                group_lines: vec![],
                template_lines: vec![],
                conditions: vec![],
                classifications: super::super::copy::domain::Classifications {
                    categories: vec![],
                    domains: vec![],
                    funds: vec![],
                    supports: vec![],
                    types: vec![],
                    subcategories: vec![],
                    flemish_shares: vec![],
                },
            },
        }
    }
}
