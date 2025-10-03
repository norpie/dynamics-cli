//! Entity name pluralization utilities for Dynamics 365 Web API

/// Convert entity name to plural form using English grammar rules
pub fn pluralize_entity_name(entity_name: &str) -> String {
    if entity_name.is_empty() {
        return entity_name.to_string();
    }

    let lower = entity_name.to_lowercase();


    // Words ending in 's', 'ss', 'sh', 'ch', 'x', 'z' -> add 'es'
    // Special case for 'z': double it before adding 'es'
    if lower.ends_with("z") && !lower.ends_with("tz") && !lower.ends_with("tz") {
        return format!("{}zes", entity_name);
    }
    if lower.ends_with("s") || lower.ends_with("ss") || lower.ends_with("sh") ||
       lower.ends_with("ch") || lower.ends_with("x") {
        return format!("{}es", entity_name);
    }

    // Words ending in consonant + 'y' -> change 'y' to 'ies'
    if lower.ends_with("y") && lower.len() > 1 {
        let second_last = lower.chars().nth(lower.len() - 2).unwrap();
        if !"aeiou".contains(second_last) {
            return format!("{}ies", &entity_name[..entity_name.len() - 1]);
        }
    }

    // Words ending in 'f' or 'fe' -> change to 'ves'
    if lower.ends_with("fe") {
        return format!("{}ves", &entity_name[..entity_name.len() - 2]);
    }
    if lower.ends_with("f") {
        return format!("{}ves", &entity_name[..entity_name.len() - 1]);
    }

    // Words ending in consonant + 'o' -> add 'es'
    if lower.ends_with("o") && lower.len() > 1 {
        let second_last = lower.chars().nth(lower.len() - 2).unwrap();
        if !"aeiou".contains(second_last) {
            return format!("{}es", entity_name);
        }
    }

    // Default: add 's'
    format!("{}s", entity_name)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regular_plurals() {
        assert_eq!(pluralize_entity_name("contact"), "contacts");
        assert_eq!(pluralize_entity_name("account"), "accounts");
        assert_eq!(pluralize_entity_name("product"), "products");
    }

    #[test]
    fn test_s_sh_ch_x_z_endings() {
        assert_eq!(pluralize_entity_name("address"), "addresses");
        assert_eq!(pluralize_entity_name("branch"), "branches");
        assert_eq!(pluralize_entity_name("box"), "boxes");
        assert_eq!(pluralize_entity_name("quiz"), "quizzes");
    }

    #[test]
    fn test_consonant_y_endings() {
        assert_eq!(pluralize_entity_name("company"), "companies");
        assert_eq!(pluralize_entity_name("category"), "categories");
        assert_eq!(pluralize_entity_name("opportunity"), "opportunities");
    }

    #[test]
    fn test_vowel_y_endings() {
        assert_eq!(pluralize_entity_name("key"), "keys");
        assert_eq!(pluralize_entity_name("survey"), "surveys");
    }

    #[test]
    fn test_f_fe_endings() {
        assert_eq!(pluralize_entity_name("leaf"), "leaves");
        assert_eq!(pluralize_entity_name("knife"), "knives");
        assert_eq!(pluralize_entity_name("life"), "lives");
    }

    #[test]
    fn test_consonant_o_endings() {
        assert_eq!(pluralize_entity_name("hero"), "heroes");
        assert_eq!(pluralize_entity_name("potato"), "potatoes");
    }

    #[test]
    fn test_vowel_o_endings() {
        assert_eq!(pluralize_entity_name("video"), "videos");
        assert_eq!(pluralize_entity_name("radio"), "radios");
    }


    #[test]
    fn test_custom_entities() {
        assert_eq!(pluralize_entity_name("new_entity"), "new_entities");
        assert_eq!(pluralize_entity_name("cgk_contact"), "cgk_contacts");
        assert_eq!(pluralize_entity_name("prefix_item"), "prefix_items");
    }
}