use std::env;

#[derive(Debug, Clone)]
pub struct Settings {
    pub database_url: String,
    pub port: u16,
    pub host: String,
}

impl Settings {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL")
            .or_else(|_| env::var("DATABASE_URL_DEV"))
            .unwrap_or_else(|_| "postgresql://beamrs:beamrs@localhost:5432/beamrs".to_string());

        let port = env::var("PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(8080);

        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        Ok(Self {
            database_url,
            port,
            host,
        })
    }
}
