use crate::models::leave::{LeaveApplicationPayload};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::{types::Json, PgPool};
use uuid::Uuid;

#[derive(Clone)]
pub struct LeaveService {
    pool: PgPool,
}

impl LeaveService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn save_application(
        &self,
        payload: LeaveApplicationPayload,
    ) -> Result<LeaveApplicationRecord> {
        let attachments_json = Json(
            serde_json::to_value(&payload.attachments)
                .context("Failed to serialize attachments")?,
        );

        let record = sqlx::query_as::<_, LeaveApplicationRecord>(
            r#"
                INSERT INTO leave_applications
                    (id, name, usn, department, reason, summary, attachments)
                VALUES
                    ($1, $2, $3, $4, $5, $6, $7)
                RETURNING
                    id,
                    name,
                    usn,
                    department,
                    reason,
                    summary,
                    attachments,
                    created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(payload.name)
        .bind(payload.usn)
        .bind(payload.department)
        .bind(payload.reason)
        .bind(payload.summary)
        .bind(attachments_json)
        .fetch_one(&self.pool)
        .await
        .context("Failed to save leave application")?;

        Ok(record)
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LeaveApplicationRecord {
    pub id: Uuid,
    pub name: String,
    pub usn: String,
    pub department: String,
    pub reason: String,
    pub summary: String,
    #[serde(skip_serializing)]
    pub attachments: Json<Value>,
    pub created_at: DateTime<Utc>,
}

