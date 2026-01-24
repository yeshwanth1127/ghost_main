use sqlx::PgPool;

#[derive(Clone)]
pub struct LicenseService {
    pool: PgPool,
}

impl LicenseService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn validate_license(&self, license_key: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM licenses WHERE license_key = $1 AND status = 'active')"
        )
        .bind(license_key)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }
}
