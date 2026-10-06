use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: i32,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Frequency {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ray {
    pub id: i32,
    pub frequency_id: i32,
    pub text: String,
    pub user_id: Option<i32>,
    pub user_name: String,
    pub prism_count: i32,
    pub users_prismed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Prism {
    pub id: Option<i32>,
    pub user_id: i32,
    pub ray_id: i32,
}

pub fn normalize_username(raw: &str) -> String {
    raw.trim().to_string()
}

pub fn validate_username(value: &str) -> Result<String, String> {
    let username = normalize_username(value);
    if username.is_empty() {
        return Err("username must not be blank".to_string());
    }
    if username.len() > 40 {
        return Err("username is too long".to_string());
    }
    if username.starts_with("Anon")
        && username.len() > 4
        && username[4..].chars().all(|c| c.is_ascii_digit())
    {
        return Ok(username);
    }
    if username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Ok(username);
    }
    Err("username may only contain letters, numbers, underscores, and dashes".to_string())
}

pub fn validate_frequency_name(value: &str) -> Result<String, String> {
    let name = value.trim().to_string();
    if name.is_empty() {
        return Err("frequency name must not be blank".to_string());
    }
    if name.len() > 60 {
        return Err("frequency name is too long".to_string());
    }
    Ok(name)
}

pub fn validate_ray_text(value: &str) -> Result<String, String> {
    let text = value.trim().to_string();
    if text.is_empty() {
        return Err("ray text must not be blank".to_string());
    }
    if text.len() > 300 {
        return Err("ray text is too long".to_string());
    }
    Ok(text)
}

pub fn generate_anonymous_name(seed: i32) -> String {
    format!("Anon{seed}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usernames_are_normalized() {
        let parsed = validate_username("  demo_user  ").unwrap();
        assert_eq!(parsed, "demo_user");
    }

    #[test]
    fn anonymous_names_are_valid() {
        let generated = generate_anonymous_name(42);
        assert_eq!(generated, "Anon42");
        assert!(validate_username(&generated).is_ok());
    }

    #[test]
    fn ray_text_must_be_non_empty() {
        assert!(validate_ray_text("   ").is_err());
        assert!(validate_ray_text("hello world").is_ok());
    }

    #[test]
    fn frequency_names_are_trimmed() {
        let name = validate_frequency_name("  general  ").unwrap();
        assert_eq!(name, "general");
    }
}
