/// Entity set name constants for Dynamics 365 questionnaire entities
///
/// These constants eliminate magic strings and provide compile-time guarantees
/// for entity set names used throughout the questionnaire copy process.

pub const QUESTIONNAIRES: &str = "nrq_questionnaires";
pub const PAGES: &str = "nrq_questionnairepages";
pub const PAGE_LINES: &str = "nrq_questionnairepagelines";
pub const GROUPS: &str = "nrq_questiongroups";
pub const GROUP_LINES: &str = "nrq_questiongrouplines";
pub const QUESTIONS: &str = "nrq_questions";
pub const TEMPLATE_LINES: &str = "nrq_questiontemplatelines";
pub const CONDITIONS: &str = "nrq_questionconditions";
pub const CONDITION_ACTIONS: &str = "nrq_questionconditionactions";

// Shared entities (referenced, not copied)
pub const TEMPLATES: &str = "nrq_questiontemplates";
pub const TAGS: &str = "nrq_questiontags";

// Classifications
pub const CATEGORIES: &str = "nrq_categories";
pub const DOMAINS: &str = "nrq_domains";
pub const FUNDS: &str = "nrq_funds";
pub const SUPPORTS: &str = "nrq_supports";
pub const TYPES: &str = "nrq_types";
pub const SUBCATEGORIES: &str = "nrq_subcategories";
pub const FLEMISH_SHARES: &str = "nrq_flemishshares";
