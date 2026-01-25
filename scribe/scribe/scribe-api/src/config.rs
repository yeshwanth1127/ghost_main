use std::env;

#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub api_access_key: String,
    pub openrouter_api_key: String,
    pub openrouter_base_url: String,
    pub openai_api_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Config {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            database_url: env::var("DATABASE_URL")
                .ok()
                .ok_or("DATABASE_URL not set")?,
            redis_url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            api_access_key: env::var("API_ACCESS_KEY")
                .ok()
                .ok_or("API_ACCESS_KEY not set")?,
            openrouter_api_key: env::var("OPENROUTER_API_KEY")
                .ok()
                .ok_or("OPENROUTER_API_KEY not set")?,
            openrouter_base_url: env::var("OPENROUTER_BASE_URL")
                .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string()),
            openai_api_key: env::var("OPENAI_API_KEY")
                .ok()
                .ok_or("OPENAI_API_KEY not set")?,
        })
    }
}
