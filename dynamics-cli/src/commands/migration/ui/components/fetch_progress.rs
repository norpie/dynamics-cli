#[derive(Debug, Clone)]
pub struct FetchProgress {
    pub source_fields: FetchStatus,
    pub target_fields: FetchStatus,
    pub source_views: FetchStatus,
    pub target_views: FetchStatus,
    pub source_forms: FetchStatus,
    pub target_forms: FetchStatus,
    pub examples: FetchStatus,
}

#[derive(Debug, Clone)]
pub enum FetchStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

impl FetchProgress {
    pub fn new() -> Self {
        Self {
            source_fields: FetchStatus::Pending,
            target_fields: FetchStatus::Pending,
            source_views: FetchStatus::Pending,
            target_views: FetchStatus::Pending,
            source_forms: FetchStatus::Pending,
            target_forms: FetchStatus::Pending,
            examples: FetchStatus::Pending,
        }
    }

    pub fn has_any_failures(&self) -> bool {
        matches!(self.source_fields, FetchStatus::Failed(_))
            || matches!(self.target_fields, FetchStatus::Failed(_))
            || matches!(self.source_views, FetchStatus::Failed(_))
            || matches!(self.target_views, FetchStatus::Failed(_))
            || matches!(self.source_forms, FetchStatus::Failed(_))
            || matches!(self.target_forms, FetchStatus::Failed(_))
            || matches!(self.examples, FetchStatus::Failed(_))
    }

    pub fn all_completed(&self) -> bool {
        matches!(self.source_fields, FetchStatus::Completed)
            && matches!(self.target_fields, FetchStatus::Completed)
            && matches!(self.source_views, FetchStatus::Completed)
            && matches!(self.target_views, FetchStatus::Completed)
            && matches!(self.source_forms, FetchStatus::Completed)
            && matches!(self.target_forms, FetchStatus::Completed)
            && matches!(self.examples, FetchStatus::Completed)
    }

    pub fn get_error_messages(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if let FetchStatus::Failed(msg) = &self.source_fields {
            errors.push(format!("Source fields: {}", msg));
        }
        if let FetchStatus::Failed(msg) = &self.target_fields {
            errors.push(format!("Target fields: {}", msg));
        }
        if let FetchStatus::Failed(msg) = &self.source_views {
            errors.push(format!("Source views: {}", msg));
        }
        if let FetchStatus::Failed(msg) = &self.target_views {
            errors.push(format!("Target views: {}", msg));
        }
        if let FetchStatus::Failed(msg) = &self.source_forms {
            errors.push(format!("Source forms: {}", msg));
        }
        if let FetchStatus::Failed(msg) = &self.target_forms {
            errors.push(format!("Target forms: {}", msg));
        }
        if let FetchStatus::Failed(msg) = &self.examples {
            errors.push(format!("Examples: {}", msg));
        }

        errors
    }
}

impl Default for FetchProgress {
    fn default() -> Self {
        Self::new()
    }
}
