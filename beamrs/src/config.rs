use std::env;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Settings {
    pub database_url: String,
    pub port: u16,
    pub host: String,
}

impl Settings {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let database_url =
            env::var("DATABASE_URL").context("DATABASE_URL must be set to a PostgreSQL URL")?;
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .context("PORT must be a valid number between 0 and 65535")?;
        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        Ok(Self {
            database_url,
            port,
            host,
        })
    }
}
