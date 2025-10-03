// Test file for Validate derive macro
use dynamics_lib_macros::Validate;
use crate::tui::widgets::{TextInputField, SelectField};

#[derive(Validate)]
struct TestForm {
    #[validate(not_empty, message = "Name is required")]
    name: TextInputField,

    #[validate(required, message = "Source is required")]
    source: SelectField,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_name() {
        let form = TestForm {
            name: TextInputField { value: String::new(), state: Default::default() },
            source: SelectField { selected_option: Some("test".to_string()), state: Default::default() },
        };

        let result = form.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Name is required");
    }

    #[test]
    fn test_validate_missing_source() {
        let mut form = TestForm {
            name: TextInputField { value: "test".to_string(), state: Default::default() },
            source: SelectField { selected_option: None, state: Default::default() },
        };

        let result = form.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Source is required");
    }

    #[test]
    fn test_validate_success() {
        let form = TestForm {
            name: TextInputField { value: "test".to_string(), state: Default::default() },
            source: SelectField { selected_option: Some("source".to_string()), state: Default::default() },
        };

        let result = form.validate();
        assert!(result.is_ok());
    }
}
