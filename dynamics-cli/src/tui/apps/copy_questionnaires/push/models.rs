#[derive(Clone)]
pub struct State {
    pub questionnaire_id: String,
    pub copy_name: String,
    pub copy_code: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            questionnaire_id: String::new(),
            copy_name: String::new(),
            copy_code: String::new(),
        }
    }
}

#[derive(Clone)]
pub enum Msg {
    Back,
}

pub struct PushQuestionnaireParams {
    pub questionnaire_id: String,
    pub copy_name: String,
    pub copy_code: String,
}

impl Default for PushQuestionnaireParams {
    fn default() -> Self {
        Self {
            questionnaire_id: String::new(),
            copy_name: String::new(),
            copy_code: String::new(),
        }
    }
}
