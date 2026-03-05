use std::env;

/// Hardcoded JWT signing secret for admin tokens
const ADMIN_SECRET: &str = "ghost-admin-jwt-secret-2025";

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub admin_secret: String,
    pub razorpay_key_id: String,
    pub razorpay_key_secret: String,
    pub razorpay_webhook_secret: String,
    pub razorpay_plan_starter: String,
    pub razorpay_plan_pro: String,
    pub razorpay_plan_power: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL not set. Use same DB as scribe-api.")?;
        let port = env::var("ADMIN_PORT")
            .unwrap_or_else(|_| "6660".to_string())
            .parse()
            .unwrap_or(6660);

        Ok(Config {
            database_url,
            port,
            admin_secret: env::var("ADMIN_SECRET").unwrap_or_else(|_| ADMIN_SECRET.to_string()),
            razorpay_key_id: env::var("RAZORPAY_KEY_ID").unwrap_or_else(|_| String::new()),
            razorpay_key_secret: env::var("RAZORPAY_KEY_SECRET").unwrap_or_else(|_| String::new()),
            razorpay_webhook_secret: env::var("RAZORPAY_WEBHOOK_SECRET").unwrap_or_else(|_| String::new()),
            razorpay_plan_starter: env::var("RAZORPAY_PLAN_STARTER").unwrap_or_else(|_| String::new()),
            razorpay_plan_pro: env::var("RAZORPAY_PLAN_PRO").unwrap_or_else(|_| String::new()),
            razorpay_plan_power: env::var("RAZORPAY_PLAN_POWER").unwrap_or_else(|_| String::new()),
        })
    }
}
