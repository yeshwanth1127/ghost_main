use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LeaveAttachment {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub base64: String,
    pub size: i64,
}

#[derive(Debug, Deserialize)]
pub struct LeaveApplicationRequest {
    pub name: String,
    pub usn: String,
    pub department: String,
    pub reason: String,
    pub summary: String,
    pub attachments: Vec<LeaveAttachment>,
}

#[derive(Debug, Serialize)]
pub struct LeaveApplicationResponse {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LeaveApplicationPayload {
    pub name: String,
    pub usn: String,
    pub department: String,
    pub reason: String,
    pub summary: String,
    pub attachments: Vec<LeaveAttachment>,
}

impl From<LeaveApplicationRequest> for LeaveApplicationPayload {
    fn from(value: LeaveApplicationRequest) -> Self {
        Self {
            name: value.name,
            usn: value.usn,
            department: value.department,
            reason: value.reason,
            summary: value.summary,
            attachments: value.attachments,
        }
    }
}





