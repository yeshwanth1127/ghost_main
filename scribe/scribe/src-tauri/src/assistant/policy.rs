use crate::assistant::types::*;
use jsonwebtoken::{encode, decode, Header, EncodingKey, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Secret key for signing tokens (in production, should be stored securely)
static TOKEN_SECRET: &[u8] = b"actionable-assistant-secret-key-change-in-production";

#[derive(Debug, Serialize, Deserialize)]
struct TokenClaims {
    nonce: String,
    scopes: Vec<String>,
    session_id: String,
    expires_at: i64,
    issued_at: i64,
}

/// Mint a new capability token
pub fn mint_capability_token(
    scopes: Vec<String>,
    ttl_seconds: i64,
    session_id: String,
) -> Result<CapabilityToken, String> {
    let now = chrono::Utc::now().timestamp();
    let expires_at = now + ttl_seconds;
    let nonce = Uuid::new_v4().to_string();

    let claims = TokenClaims {
        nonce: nonce.clone(),
        scopes: scopes.clone(),
        session_id: session_id.clone(),
        expires_at,
        issued_at: now,
    };

    let _token_string = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TOKEN_SECRET),
    )
    .map_err(|e| format!("Failed to encode token: {}", e))?;

    Ok(CapabilityToken {
        nonce,
        scopes,
        ttl_seconds,
        session_id,
        expires_at,
        issued_at: now,
    })
}

/// Validate and decode a capability token
pub fn validate_token(token_string: &str) -> Result<CapabilityToken, String> {
    let validation = Validation::default();
    
    let token_data = decode::<TokenClaims>(
        token_string,
        &DecodingKey::from_secret(TOKEN_SECRET),
        &validation,
    )
    .map_err(|e| format!("Failed to decode token: {}", e))?;

    let claims = token_data.claims;

    // Check expiration
    let now = chrono::Utc::now().timestamp();
    if claims.expires_at < now {
        return Err("Token has expired".to_string());
    }

    Ok(CapabilityToken {
        nonce: claims.nonce,
        scopes: claims.scopes,
        ttl_seconds: claims.expires_at - claims.issued_at,
        session_id: claims.session_id,
        expires_at: claims.expires_at,
        issued_at: claims.issued_at,
    })
}

/// Check if a token has permission for an action
pub fn check_permission(
    token: &CapabilityToken,
    action_type: &str,
    resource: &str,
) -> bool {
    for scope_str in &token.scopes {
        if let Ok(scope) = Scope::parse(scope_str) {
            if scope.matches(action_type, resource) {
                return true;
            }
        }
    }
    false
}

/// Revoke a token (mark nonce as revoked in database)
pub fn revoke_token_nonce(_nonce: &str) -> Result<(), String> {
    // This should update the database to mark the nonce as revoked
    // For now, we'll just return Ok (database update will be done via IPC)
    Ok(())
}

