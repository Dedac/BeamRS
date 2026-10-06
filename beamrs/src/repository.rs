use anyhow::{Context, Result};
use sqlx::{PgPool, Row};

use crate::domain::{
    generate_anonymous_name, validate_frequency_name, validate_ray_text, validate_username,
    Frequency, Ray, User,
};

#[derive(Clone)]
pub struct BeamRepository {
    pool: PgPool,
}

impl BeamRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn initialize(pool: &PgPool) -> Result<()> {
        sqlx::migrate!("./migrations").run(pool).await?;
        Ok(())
    }

    pub async fn list_frequencies(&self) -> Result<Vec<Frequency>> {
        let rows = sqlx::query("SELECT id, name FROM frequencies ORDER BY name ASC")
            .fetch_all(&self.pool)
            .await?;

        let mut frequencies = Vec::with_capacity(rows.len());
        for row in rows {
            frequencies.push(Frequency {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
            });
        }

        Ok(frequencies)
    }

    pub async fn create_frequency(&self, raw_name: &str) -> Result<Frequency> {
        let name = validate_frequency_name(raw_name).map_err(|err| anyhow::anyhow!(err))?;
        let row = sqlx::query("INSERT INTO frequencies (name) VALUES ($1) ON CONFLICT (name) DO NOTHING RETURNING id, name")
            .bind(&name)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(existing) => Ok(Frequency {
                id: existing.try_get("id")?,
                name: existing.try_get("name")?,
            }),
            None => {
                let existing = sqlx::query("SELECT id, name FROM frequencies WHERE name = $1")
                    .bind(&name)
                    .fetch_one(&self.pool)
                    .await?;
                Ok(Frequency {
                    id: existing.try_get("id")?,
                    name: existing.try_get("name")?,
                })
            }
        }
    }

    pub async fn get_or_create_user(&self, raw_username: &str) -> Result<User> {
        let username = validate_username(raw_username).unwrap_or_else(|_| {
            let fallback = generate_anonymous_name(1 + rand::random::<i32>().rem_euclid(9999));
            validate_username(&fallback).expect("anonymous username is valid")
        });

        let row = sqlx::query("SELECT id, username FROM users WHERE username = $1")
            .bind(&username)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(existing) = row {
            return Ok(User {
                id: existing.try_get("id")?,
                username: existing.try_get("username")?,
            });
        }

        let row = sqlx::query("INSERT INTO users (username) VALUES ($1) RETURNING id, username")
            .bind(&username)
            .fetch_one(&self.pool)
            .await?;

        Ok(User {
            id: row.try_get("id")?,
            username: row.try_get("username")?,
        })
    }

    #[allow(dead_code)]
    pub async fn update_user_name(&self, user_id: i32, raw_username: &str) -> Result<User> {
        let username = validate_username(raw_username).map_err(|err| anyhow::anyhow!(err))?;
        let row =
            sqlx::query("UPDATE users SET username = $1 WHERE id = $2 RETURNING id, username")
                .bind(&username)
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .with_context(|| format!("failed to update username for user_id={user_id}"))?;

        Ok(User {
            id: row.try_get("id")?,
            username: row.try_get("username")?,
        })
    }

    pub async fn get_user_name(&self, user_id: i32) -> Result<String> {
        let row = sqlx::query("SELECT username FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(value) => Ok(value.try_get("username")?),
            None => Err(anyhow::anyhow!("user {user_id} not found")),
        }
    }

    pub async fn list_rays_by_frequency(&self, frequency_id: i32) -> Result<Vec<Ray>> {
        let row_list = sqlx::query(
            "SELECT id, text, frequency_id, user_id FROM rays WHERE frequency_id = $1 ORDER BY id DESC",
        )
        .bind(frequency_id)
        .fetch_all(&self.pool)
        .await?;

        let mut rays = Vec::with_capacity(row_list.len());
        for row in row_list {
            let id: i32 = row.try_get("id")?;
            let user_id: Option<i32> = row.try_get("user_id")?;
            let user_name = match user_id {
                Some(user_id) => self.get_user_name(user_id).await?,
                None => "".to_string(),
            };
            let prism_count = self.prism_count_for_ray(id).await?;
            let users_prismed = self.prism_users_for_ray(id).await?;
            rays.push(Ray {
                id,
                frequency_id: row.try_get("frequency_id")?,
                text: row.try_get("text")?,
                user_id,
                user_name,
                prism_count,
                users_prismed,
            });
        }
        Ok(rays)
    }

    pub async fn list_rays_by_user(&self, username: &str) -> Result<Vec<Ray>> {
        let user = self.get_or_create_user(username).await?;
        let rows = sqlx::query(
            "SELECT id, text, frequency_id, user_id FROM rays WHERE user_id = $1 ORDER BY id DESC",
        )
        .bind(user.id)
        .fetch_all(&self.pool)
        .await?;

        let mut rays = Vec::with_capacity(rows.len());
        for row in rows {
            let id: i32 = row.try_get("id")?;
            let user_id: Option<i32> = row.try_get("user_id")?;
            rays.push(Ray {
                id,
                frequency_id: row.try_get("frequency_id")?,
                text: row.try_get("text")?,
                user_id,
                user_name: user.username.clone(),
                prism_count: self.prism_count_for_ray(id).await?,
                users_prismed: self.prism_users_for_ray(id).await?,
            });
        }
        Ok(rays)
    }

    pub async fn list_rays_prismed_by_user(&self, username: &str) -> Result<Vec<Ray>> {
        let user = self.get_or_create_user(username).await?;
        let rows = sqlx::query(
            "SELECT r.id, r.text, r.frequency_id, r.user_id FROM rays r JOIN prisms p ON p.ray_id = r.id WHERE p.user_id = $1 ORDER BY r.id DESC",
        )
        .bind(user.id)
        .fetch_all(&self.pool)
        .await?;

        let mut rays = Vec::with_capacity(rows.len());
        for row in rows {
            let id: i32 = row.try_get("id")?;
            let user_id: Option<i32> = row.try_get("user_id")?;
            let ray_user = match user_id {
                Some(uid) => self.get_user_name(uid).await?,
                None => String::new(),
            };
            rays.push(Ray {
                id,
                frequency_id: row.try_get("frequency_id")?,
                text: row.try_get("text")?,
                user_id,
                user_name: ray_user,
                prism_count: self.prism_count_for_ray(id).await?,
                users_prismed: self.prism_users_for_ray(id).await?,
            });
        }
        Ok(rays)
    }

    pub async fn create_ray(
        &self,
        frequency_id: i32,
        user_id: i32,
        raw_text: &str,
    ) -> Result<Vec<Ray>> {
        let text = validate_ray_text(raw_text).map_err(|err| anyhow::anyhow!(err))?;
        sqlx::query("INSERT INTO rays (frequency_id, user_id, text) VALUES ($1, $2, $3)")
            .bind(frequency_id)
            .bind(user_id)
            .bind(&text)
            .execute(&self.pool)
            .await?;

        self.list_rays_by_frequency(frequency_id).await
    }

    pub async fn add_prism(&self, user_id: i32, ray_id: i32) -> Result<Vec<Ray>> {
        let ray = sqlx::query("SELECT frequency_id FROM rays WHERE id = $1")
            .bind(ray_id)
            .fetch_optional(&self.pool)
            .await?;

        let Some(ray) = ray else {
            return Err(anyhow::anyhow!("ray {ray_id} not found"));
        };

        let frequency_id: i32 = ray.try_get("frequency_id")?;

        sqlx::query("INSERT INTO prisms (user_id, ray_id) VALUES ($1, $2) ON CONFLICT (user_id, ray_id) DO NOTHING")
            .bind(user_id)
            .bind(ray_id)
            .execute(&self.pool)
            .await?;

        self.list_rays_by_frequency(frequency_id).await
    }

    pub async fn remove_prism(&self, user_id: i32, ray_id: i32) -> Result<Vec<Ray>> {
        let frequency_id = sqlx::query("SELECT frequency_id FROM rays WHERE id = $1")
            .bind(ray_id)
            .fetch_optional(&self.pool)
            .await?;

        let Some(frequency) = frequency_id else {
            return Err(anyhow::anyhow!("ray {ray_id} not found"));
        };
        let frequency_id: i32 = frequency.try_get("frequency_id")?;

        sqlx::query("DELETE FROM prisms WHERE user_id = $1 AND ray_id = $2")
            .bind(user_id)
            .bind(ray_id)
            .execute(&self.pool)
            .await?;

        self.list_rays_by_frequency(frequency_id).await
    }

    pub async fn prism_count_for_ray(&self, ray_id: i32) -> Result<i32> {
        let row = sqlx::query("SELECT COUNT(*) AS count FROM prisms WHERE ray_id = $1")
            .bind(ray_id)
            .fetch_one(&self.pool)
            .await?;
        let count: i64 = row.try_get("count")?;
        Ok(count as i32)
    }

    pub async fn prism_users_for_ray(&self, ray_id: i32) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT u.username FROM prisms p JOIN users u ON u.id = p.user_id WHERE p.ray_id = $1 ORDER BY u.username ASC",
        )
        .bind(ray_id)
        .fetch_all(&self.pool)
        .await?;

        let mut names = Vec::with_capacity(rows.len());
        for row in rows {
            names.push(row.try_get("username")?);
        }
        Ok(names)
    }

    #[allow(dead_code)]
    pub async fn has_user_prismed(&self, user_id: i32, ray_id: i32) -> Result<bool> {
        let row = sqlx::query("SELECT 1 FROM prisms WHERE user_id = $1 AND ray_id = $2 LIMIT 1")
            .bind(user_id)
            .bind(ray_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.is_some())
    }
}
