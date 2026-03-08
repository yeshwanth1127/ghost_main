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
    pub resend_api_key: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from_email: String,
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
            resend_api_key: env::var("RESEND_API_KEY").unwrap_or_default(),
            smtp_host: env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.hostinger.com".to_string()),
            smtp_port: env::var("SMTP_PORT").unwrap_or_else(|_| "587".to_string()).parse().unwrap_or(587),
            smtp_username: env::var("SMTP_USERNAME").unwrap_or_default(),
            smtp_password: env::var("SMTP_PASSWORD").unwrap_or_default(),
            smtp_from_email: env::var("SMTP_FROM_EMAIL").unwrap_or_else(|_| "support@exora.solutions".to_string()),
        })
    }
}
