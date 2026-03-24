use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// PIN — Security isolation types
// ---------------------------------------------------------------------------

/// Full PIN config row from the `pin_config` singleton table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinConfig {
    pub pin_hash: Option<String>,
    pub recovery_hash: Option<String>,
    pub failed_attempts: i32,
    pub lockout_until: Option<String>,
    pub updated_at: String,
}

impl PinConfig {
    /// Whether a PIN is set.
    pub fn has_pin(&self) -> bool {
        self.pin_hash.is_some()
    }

    /// Whether the account is currently locked out.
    pub fn is_locked(&self) -> bool {
        self.lockout_seconds_remaining() > 0
    }

    /// Remaining seconds until lockout expires.
    pub fn lockout_seconds_remaining(&self) -> i32 {
        if let Some(ref until) = self.lockout_until {
            if let Ok(until_dt) = chrono::NaiveDateTime::parse_from_str(until, "%Y-%m-%d %H:%M:%S")
            {
                let now = chrono::Utc::now().naive_utc();
                if until_dt > now {
                    return (until_dt - now).num_seconds() as i32;
                }
            }
        }
        0
    }
}

/// Frontend-facing PIN status (no hashes leaked).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PinStatus {
    pub has_pin: bool,
    pub is_locked: bool,
    pub attempts_remaining: i32,
    pub lockout_seconds_remaining: i32,
}
