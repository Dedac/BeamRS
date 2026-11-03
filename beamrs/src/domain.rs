use serde::{Deserialize, Serialize};

pub const MAX_USERNAME_LEN: usize = 40;
pub const MAX_FREQUENCY_NAME_LEN: usize = 60;
pub const MAX_RAY_TEXT_LEN: usize = 300;
pub const DEFAULT_FEED_PAGE_SIZE: usize = 25;
pub const MAX_FEED_PAGE_SIZE: usize = 100;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedCursor {
    pub created_at_micros: i64,
    pub id: i32,
}

// The repository converts cursor timestamps through DOUBLE PRECISION, which is
// exact up to 2^53 microseconds (about years 1685–2255) and keeps PostgreSQL's
// interval and timestamp arithmetic in range.
const MAX_CURSOR_MICROS: u64 = 1 << 53;

impl FeedCursor {
    pub fn parse(value: &str) -> Result<Self, String> {
        let (created_at_micros, id) = value
            .split_once(':')
            .ok_or_else(|| "cursor must contain a timestamp and ray id".to_string())?;
        let created_at_micros: i64 = created_at_micros
            .parse()
            .map_err(|_| "cursor timestamp is invalid".to_string())?;
        if created_at_micros.unsigned_abs() > MAX_CURSOR_MICROS {
            return Err("cursor timestamp is out of range".to_string());
        }
        let id = id
            .parse()
            .map_err(|_| "cursor ray id is invalid".to_string())?;
        if id <= 0 {
            return Err("cursor ray id must be positive".to_string());
        }
        Ok(Self {
            created_at_micros,
            id,
        })
    }

    pub fn encode(self) -> String {
        format!("{}:{}", self.created_at_micros, self.id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedPage {
    pub rays: Vec<Ray>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Prism {
    pub id: Option<i32>,
    pub user_id: i32,
    pub ray_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsernameInput {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrequencyInput {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RayInput {
    pub frequency_id: i32,
    pub user_id: i32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrismInput {
    pub user_id: i32,
    pub ray_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrismResult {
    pub ray_id: i32,
    pub prism_count: i32,
    pub prismed: bool,
}

pub fn normalize_username(raw: &str) -> String {
    raw.trim().to_string()
}

pub fn validate_username(value: &str) -> Result<String, String> {
    let username = normalize_username(value);
    if username.is_empty() {
        return Err("username must not be blank".to_string());
    }
    if username.chars().count() > MAX_USERNAME_LEN {
        return Err(format!(
            "username must be at most {MAX_USERNAME_LEN} characters"
        ));
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
    if name.chars().count() > MAX_FREQUENCY_NAME_LEN {
        return Err(format!(
            "frequency name must be at most {MAX_FREQUENCY_NAME_LEN} characters"
        ));
    }
    Ok(name)
}

pub fn validate_ray_text(value: &str) -> Result<String, String> {
    let text = value.trim().to_string();
    if text.is_empty() {
        return Err("ray text must not be blank".to_string());
    }
    if text.chars().count() > MAX_RAY_TEXT_LEN {
        return Err(format!(
            "ray text must be at most {MAX_RAY_TEXT_LEN} characters"
        ));
    }
    Ok(text)
}

pub fn generate_anonymous_name(seed: u32) -> String {
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

    #[test]
    fn usernames_reject_markup_and_spaces() {
        assert!(validate_username("<script>").is_err());
        assert!(validate_username("two words").is_err());
        assert!(validate_username("beam-user_1").is_ok());
    }

    #[test]
    fn validation_uses_character_limits() {
        assert!(validate_username(&"a".repeat(MAX_USERNAME_LEN)).is_ok());
        assert!(validate_username(&"a".repeat(MAX_USERNAME_LEN + 1)).is_err());
        assert!(validate_frequency_name(&"光".repeat(MAX_FREQUENCY_NAME_LEN)).is_ok());
        assert!(validate_ray_text(&"光".repeat(MAX_RAY_TEXT_LEN)).is_ok());
    }

    #[test]
    fn feed_cursors_round_trip_and_reject_malformed_values() {
        let cursor = FeedCursor {
            created_at_micros: 1_767_225_600_123_456,
            id: 42,
        };
        assert_eq!(FeedCursor::parse(&cursor.encode()), Ok(cursor));
        assert!(FeedCursor::parse("not-a-cursor").is_err());
        assert!(FeedCursor::parse("123:0").is_err());
        assert!(FeedCursor::parse("123:42:extra").is_err());
    }

    #[test]
    fn feed_cursors_reject_timestamps_postgres_cannot_convert_exactly() {
        let limit = 1_i64 << 53;
        assert!(FeedCursor::parse(&format!("{limit}:1")).is_ok());
        assert!(FeedCursor::parse(&format!("{}:1", -limit)).is_ok());
        assert!(FeedCursor::parse(&format!("{}:1", limit + 1)).is_err());
        assert!(FeedCursor::parse(&format!("{}:1", -limit - 1)).is_err());
        assert!(FeedCursor::parse(&format!("{}:1", i64::MAX)).is_err());
        assert!(FeedCursor::parse(&format!("{}:1", i64::MIN)).is_err());
    }
}
