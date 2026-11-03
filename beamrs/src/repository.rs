use async_trait::async_trait;
use sqlx::{PgPool, Row};
use thiserror::Error;

use crate::domain::{
    validate_frequency_name, validate_ray_text, validate_username, FeedCursor, FeedPage, Frequency,
    PrismResult, Ray, User, MAX_FEED_PAGE_SIZE,
};

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("{0}")]
    Validation(String),
    #[error("{0} not found")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

pub type RepositoryResult<T> = Result<T, RepositoryError>;

#[async_trait]
pub trait BeamStore: Send + Sync {
    async fn list_frequencies(&self) -> RepositoryResult<Vec<Frequency>>;
    async fn create_frequency(&self, name: &str) -> RepositoryResult<Frequency>;
    async fn get_or_create_user(&self, username: &str) -> RepositoryResult<User>;
    async fn update_user_name(&self, user_id: i32, username: &str) -> RepositoryResult<User>;
    async fn list_rays_by_frequency(
        &self,
        frequency_id: i32,
        limit: usize,
        cursor: Option<FeedCursor>,
    ) -> RepositoryResult<FeedPage>;
    async fn list_rays_by_user(
        &self,
        username: &str,
        limit: usize,
        cursor: Option<FeedCursor>,
    ) -> RepositoryResult<FeedPage>;
    async fn list_rays_prismed_by_user(
        &self,
        username: &str,
        limit: usize,
        cursor: Option<FeedCursor>,
    ) -> RepositoryResult<FeedPage>;
    async fn create_ray(
        &self,
        frequency_id: i32,
        user_id: i32,
        text: &str,
    ) -> RepositoryResult<Ray>;
    async fn add_prism(&self, user_id: i32, ray_id: i32) -> RepositoryResult<PrismResult>;
    async fn remove_prism(&self, user_id: i32, ray_id: i32) -> RepositoryResult<PrismResult>;
}

#[derive(Clone)]
pub struct BeamRepository {
    pool: PgPool,
}

impl BeamRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn initialize(pool: &PgPool) -> RepositoryResult<()> {
        sqlx::migrate!("./migrations")
            .run(pool)
            .await
            .map_err(|error| RepositoryError::Database(error.into()))?;
        Ok(())
    }

    async fn ray_by_id(&self, ray_id: i32) -> RepositoryResult<Ray> {
        let row = sqlx::query(
            r#"
            SELECT r.id, r.text, r.frequency_id, r.user_id,
                   COALESCE(u.username, '') AS user_name,
                   COUNT(p.id)::INT AS prism_count,
                   COALESCE(
                       ARRAY_AGG(pu.username ORDER BY pu.username)
                           FILTER (WHERE pu.username IS NOT NULL),
                       ARRAY[]::TEXT[]
                   ) AS users_prismed
            FROM rays r
            LEFT JOIN users u ON u.id = r.user_id
            LEFT JOIN prisms p ON p.ray_id = r.id
            LEFT JOIN users pu ON pu.id = p.user_id
            WHERE r.id = $1
            GROUP BY r.id, u.username
            "#,
        )
        .bind(ray_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_ray)
            .transpose()?
            .ok_or_else(|| RepositoryError::NotFound(format!("ray {ray_id}")))
    }

    async fn list_rays_where(
        &self,
        predicate: &str,
        bind_text: Option<&str>,
        bind_id: Option<i32>,
        limit: usize,
        cursor: Option<FeedCursor>,
    ) -> RepositoryResult<FeedPage> {
        if !(1..=MAX_FEED_PAGE_SIZE).contains(&limit) {
            return Err(RepositoryError::Validation(format!(
                "feed limit must be between 1 and {MAX_FEED_PAGE_SIZE}"
            )));
        }
        // Select the keyset page before enriching it so query cost tracks the
        // page size rather than the size of the whole feed. Author and prism
        // lookups are correlated per page row so generic plans cannot fall back
        // to hashing whole users/prisms tables. The cursor clause is omitted
        // entirely on the first page so later pages can use it as an index
        // condition.
        let cursor_clause = if cursor.is_some() {
            r#"AND (r.created_at, r.id) <
                   (
                       TIMESTAMPTZ 'epoch'
                           + ($3::DOUBLE PRECISION * INTERVAL '1 microsecond'),
                       $4::INTEGER
                   )"#
        } else {
            ""
        };
        let sql = format!(
            r#"
            WITH page AS (
                SELECT r.id, r.text, r.frequency_id, r.user_id, r.created_at
                FROM rays r
                WHERE {predicate}
                  {cursor_clause}
                ORDER BY r.created_at DESC, r.id DESC
                LIMIT $2
            )
            SELECT page.id, page.text, page.frequency_id, page.user_id,
                   COALESCE(
                       (SELECT author.username FROM users author
                        WHERE author.id = page.user_id),
                       ''
                   ) AS user_name,
                   prism_stats.prism_count,
                   FLOOR(EXTRACT(EPOCH FROM page.created_at) * 1000000)::BIGINT
                       AS cursor_created_at_micros,
                   prism_stats.users_prismed
            FROM page
            CROSS JOIN LATERAL (
                SELECT COUNT(*)::INT AS prism_count,
                       COALESCE(
                           ARRAY_AGG(pu.username ORDER BY pu.username)
                               FILTER (WHERE pu.username IS NOT NULL),
                           ARRAY[]::TEXT[]
                       ) AS users_prismed
                FROM prisms p
                LEFT JOIN users pu ON pu.id = p.user_id
                WHERE p.ray_id = page.id
            ) prism_stats
            ORDER BY page.created_at DESC, page.id DESC
            "#
        );
        let limit_with_lookahead = (limit + 1) as i64;

        let query = match (bind_text, bind_id) {
            (Some(value), None) => sqlx::query(&sql).bind(value),
            (None, Some(value)) => sqlx::query(&sql).bind(value),
            _ => {
                return Err(RepositoryError::Database(sqlx::Error::Protocol(
                    "invalid ray query binding".into(),
                )))
            }
        };
        let mut query = query.bind(limit_with_lookahead);
        if let Some(cursor) = cursor {
            query = query.bind(cursor.created_at_micros).bind(cursor.id);
        }
        let rows = query.fetch_all(&self.pool).await?;

        let has_more = rows.len() > limit;
        let mut rows = rows;
        if has_more {
            rows.pop();
        }
        let next_cursor = if has_more {
            rows.last()
                .map(|row| -> Result<String, RepositoryError> {
                    Ok(FeedCursor {
                        created_at_micros: row.try_get("cursor_created_at_micros")?,
                        id: row.try_get("id")?,
                    }
                    .encode())
                })
                .transpose()?
        } else {
            None
        };
        let rays = rows
            .into_iter()
            .map(row_to_ray)
            .collect::<RepositoryResult<_>>()?;
        Ok(FeedPage { rays, next_cursor })
    }
}

fn row_to_ray(row: sqlx::postgres::PgRow) -> RepositoryResult<Ray> {
    Ok(Ray {
        id: row.try_get("id")?,
        frequency_id: row.try_get("frequency_id")?,
        text: row.try_get("text")?,
        user_id: row.try_get("user_id")?,
        user_name: row.try_get("user_name")?,
        prism_count: row.try_get("prism_count")?,
        users_prismed: row.try_get("users_prismed")?,
    })
}

fn map_unique_violation(error: sqlx::Error, message: &str) -> RepositoryError {
    if error
        .as_database_error()
        .is_some_and(|database_error| database_error.is_unique_violation())
    {
        RepositoryError::Conflict(message.to_string())
    } else {
        RepositoryError::Database(error)
    }
}

#[async_trait]
impl BeamStore for BeamRepository {
    async fn list_frequencies(&self) -> RepositoryResult<Vec<Frequency>> {
        let rows = sqlx::query("SELECT id, name FROM frequencies ORDER BY name ASC")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter()
            .map(|row| {
                Ok(Frequency {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                })
            })
            .collect()
    }

    async fn create_frequency(&self, raw_name: &str) -> RepositoryResult<Frequency> {
        let name = validate_frequency_name(raw_name).map_err(RepositoryError::Validation)?;
        let row = sqlx::query("INSERT INTO frequencies (name) VALUES ($1) RETURNING id, name")
            .bind(&name)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| map_unique_violation(error, "frequency already exists"))?;
        Ok(Frequency {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
        })
    }

    async fn get_or_create_user(&self, raw_username: &str) -> RepositoryResult<User> {
        let username = validate_username(raw_username).map_err(RepositoryError::Validation)?;
        let row = sqlx::query(
            r#"
            INSERT INTO users (username)
            VALUES ($1)
            ON CONFLICT (username) DO UPDATE SET username = EXCLUDED.username
            RETURNING id, username
            "#,
        )
        .bind(&username)
        .fetch_one(&self.pool)
        .await?;
        Ok(User {
            id: row.try_get("id")?,
            username: row.try_get("username")?,
        })
    }

    async fn update_user_name(&self, user_id: i32, raw_username: &str) -> RepositoryResult<User> {
        let username = validate_username(raw_username).map_err(RepositoryError::Validation)?;
        let row =
            sqlx::query("UPDATE users SET username = $1 WHERE id = $2 RETURNING id, username")
                .bind(&username)
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|error| map_unique_violation(error, "username already exists"))?;
        let row = row.ok_or_else(|| RepositoryError::NotFound(format!("user {user_id}")))?;
        Ok(User {
            id: row.try_get("id")?,
            username: row.try_get("username")?,
        })
    }

    async fn list_rays_by_frequency(
        &self,
        frequency_id: i32,
        limit: usize,
        cursor: Option<FeedCursor>,
    ) -> RepositoryResult<FeedPage> {
        self.list_rays_where(
            "r.frequency_id = $1",
            None,
            Some(frequency_id),
            limit,
            cursor,
        )
        .await
    }

    async fn list_rays_by_user(
        &self,
        username: &str,
        limit: usize,
        cursor: Option<FeedCursor>,
    ) -> RepositoryResult<FeedPage> {
        let username = validate_username(username).map_err(RepositoryError::Validation)?;
        self.list_rays_where(
            "r.user_id = (SELECT id FROM users WHERE username = $1)",
            Some(&username),
            None,
            limit,
            cursor,
        )
        .await
    }

    async fn list_rays_prismed_by_user(
        &self,
        username: &str,
        limit: usize,
        cursor: Option<FeedCursor>,
    ) -> RepositoryResult<FeedPage> {
        let username = validate_username(username).map_err(RepositoryError::Validation)?;
        self.list_rays_where(
            "EXISTS (
                SELECT 1
                FROM prisms selected_prism
                JOIN users selected_user ON selected_user.id = selected_prism.user_id
                WHERE selected_prism.ray_id = r.id AND selected_user.username = $1
            )",
            Some(&username),
            None,
            limit,
            cursor,
        )
        .await
    }

    async fn create_ray(
        &self,
        frequency_id: i32,
        user_id: i32,
        raw_text: &str,
    ) -> RepositoryResult<Ray> {
        let text = validate_ray_text(raw_text).map_err(RepositoryError::Validation)?;
        let row = sqlx::query(
            "INSERT INTO rays (frequency_id, user_id, text) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(frequency_id)
        .bind(user_id)
        .bind(&text)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| {
            if error
                .as_database_error()
                .is_some_and(|database_error| database_error.is_foreign_key_violation())
            {
                RepositoryError::NotFound("frequency or user".to_string())
            } else {
                RepositoryError::Database(error)
            }
        })?;
        self.ray_by_id(row.try_get("id")?).await
    }

    async fn add_prism(&self, user_id: i32, ray_id: i32) -> RepositoryResult<PrismResult> {
        let mut transaction = self.pool.begin().await?;
        let ray_exists = sqlx::query("SELECT id FROM rays WHERE id = $1 FOR UPDATE")
            .bind(ray_id)
            .fetch_optional(&mut *transaction)
            .await?
            .is_some();
        if !ray_exists {
            return Err(RepositoryError::NotFound(format!("ray {ray_id}")));
        }
        sqlx::query("INSERT INTO prisms (user_id, ray_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(user_id)
            .bind(ray_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| {
                if error
                    .as_database_error()
                    .is_some_and(|database_error| database_error.is_foreign_key_violation())
                {
                    RepositoryError::NotFound(format!("user {user_id}"))
                } else {
                    RepositoryError::Database(error)
                }
            })?;
        let prism_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM prisms WHERE ray_id = $1")
            .bind(ray_id)
            .fetch_one(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(PrismResult {
            ray_id,
            prism_count: prism_count as i32,
            prismed: true,
        })
    }

    async fn remove_prism(&self, user_id: i32, ray_id: i32) -> RepositoryResult<PrismResult> {
        let mut transaction = self.pool.begin().await?;
        let ray_exists = sqlx::query("SELECT id FROM rays WHERE id = $1 FOR UPDATE")
            .bind(ray_id)
            .fetch_optional(&mut *transaction)
            .await?
            .is_some();
        if !ray_exists {
            return Err(RepositoryError::NotFound(format!("ray {ray_id}")));
        }
        sqlx::query("DELETE FROM prisms WHERE user_id = $1 AND ray_id = $2")
            .bind(user_id)
            .bind(ray_id)
            .execute(&mut *transaction)
            .await?;
        let prism_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM prisms WHERE ray_id = $1")
            .bind(ray_id)
            .fetch_one(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(PrismResult {
            ray_id,
            prism_count: prism_count as i32,
            prismed: false,
        })
    }
}
