use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// License key format: ROM-XXXX-XXXX-XXXX-XXXX
/// Storage: ~/.romance/license.json
/// Validation: POST https://api.romance.dev/v1/license/validate

const LICENSE_SERVER_URL: &str = "https://api.romance.dev/v1/license/validate";
const DEACTIVATE_URL: &str = "https://api.romance.dev/v1/license/deactivate";

/// Revalidation interval: 7 days
const REVALIDATION_DAYS: i64 = 7;

/// Grace period when offline: 30 days from last successful validation
const GRACE_PERIOD_DAYS: i64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub key: String,
    pub email: String,
    pub plan: String,
    pub activated_at: String,
    pub expires_at: String,
    pub last_validated: String,
}

#[derive(Debug, Clone)]
pub struct LicenseStatus {
    pub valid: bool,
    pub expired: bool,
    pub days_remaining: i64,
    pub plan: String,
    pub needs_revalidation: bool,
}

#[derive(Debug, Deserialize)]
pub struct LicenseValidationResponse {
    pub valid: bool,
    pub email: Option<String>,
    pub plan: Option<String>,
    pub expires_at: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
struct ValidateRequest {
    key: String,
}

#[derive(Debug, Serialize)]
struct DeactivateRequest {
    key: String,
}

/// Returns the path to the license file: ~/.romance/license.json
pub fn license_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".romance").join("license.json"))
}

/// Load the license info from disk, if it exists.
pub fn load_license() -> Result<Option<LicenseInfo>> {
    let path = license_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read license file at {}", path.display()))?;
    let info: LicenseInfo = serde_json::from_str(&content)
        .with_context(|| "Failed to parse license file")?;
    Ok(Some(info))
}

/// Save the license info to disk.
pub fn save_license(info: &LicenseInfo) -> Result<()> {
    let path = license_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(info)
        .context("Failed to serialize license info")?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write license file at {}", path.display()))?;
    Ok(())
}

/// Validate a license key online against the license server.
/// Sends POST to https://api.romance.dev/v1/license/validate with JSON {"key": key}.
/// If the server is unreachable, returns an error (caller should handle fallback).
pub fn validate_online(key: &str) -> Result<LicenseValidationResponse> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("Failed to create HTTP client")?;

    let request_body = ValidateRequest {
        key: key.to_string(),
    };

    let response = client
        .post(LICENSE_SERVER_URL)
        .json(&request_body)
        .send()
        .context("Failed to connect to license server")?;

    let validation: LicenseValidationResponse = response
        .json()
        .context("Failed to parse license server response")?;

    Ok(validation)
}

/// Main license check function. Called before every CLI command.
///
/// 1. Load license from file
/// 2. If no license -> return error
/// 3. If license exists, check if expired locally
/// 4. If last_validated is older than 7 days -> try online revalidation
/// 5. If online revalidation fails (network error) -> allow 30-day grace period
/// 6. If grace period exceeded -> return error
/// 7. Return LicenseStatus
pub fn check_license() -> Result<LicenseStatus> {
    let info = load_license()?;

    let info = match info {
        Some(info) => info,
        None => {
            bail!("No license activated. Run `romance activate <key>` to activate.");
        }
    };

    let now = Utc::now();

    // Parse expires_at
    let expires_at = DateTime::parse_from_rfc3339(&info.expires_at)
        .with_context(|| format!("Invalid expires_at date: {}", info.expires_at))?
        .with_timezone(&Utc);

    let days_remaining = (expires_at - now).num_days();
    let expired = now > expires_at;

    if expired {
        bail!(
            "Your license has expired (expired on {}). Please renew at https://romance.dev/account",
            info.expires_at
        );
    }

    // Parse last_validated
    let last_validated = DateTime::parse_from_rfc3339(&info.last_validated)
        .with_context(|| format!("Invalid last_validated date: {}", info.last_validated))?
        .with_timezone(&Utc);

    let days_since_validation = (now - last_validated).num_days();
    let needs_revalidation = days_since_validation >= REVALIDATION_DAYS;

    if needs_revalidation {
        // Try online revalidation
        match validate_online(&info.key) {
            Ok(response) => {
                if response.valid {
                    // Update license with fresh data
                    let updated_info = LicenseInfo {
                        key: info.key.clone(),
                        email: response.email.unwrap_or(info.email.clone()),
                        plan: response.plan.clone().unwrap_or(info.plan.clone()),
                        activated_at: info.activated_at.clone(),
                        expires_at: response.expires_at.unwrap_or(info.expires_at.clone()),
                        last_validated: Utc::now().to_rfc3339(),
                    };
                    save_license(&updated_info)?;

                    let new_expires = DateTime::parse_from_rfc3339(&updated_info.expires_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or(expires_at);
                    let new_days_remaining = (new_expires - Utc::now()).num_days();

                    return Ok(LicenseStatus {
                        valid: true,
                        expired: false,
                        days_remaining: new_days_remaining,
                        plan: response.plan.unwrap_or(info.plan),
                        needs_revalidation: false,
                    });
                } else {
                    let error_msg = response
                        .error
                        .unwrap_or_else(|| "License is no longer valid.".to_string());
                    bail!("License validation failed: {}", error_msg);
                }
            }
            Err(_) => {
                // Network error — check grace period
                if days_since_validation >= GRACE_PERIOD_DAYS {
                    bail!(
                        "License revalidation required. Your license has not been validated \
                         in {} days (grace period is {} days). Please connect to the internet \
                         and try again.",
                        days_since_validation,
                        GRACE_PERIOD_DAYS
                    );
                }
                // Within grace period — allow usage but flag revalidation needed
                return Ok(LicenseStatus {
                    valid: true,
                    expired: false,
                    days_remaining,
                    plan: info.plan,
                    needs_revalidation: true,
                });
            }
        }
    }

    Ok(LicenseStatus {
        valid: true,
        expired: false,
        days_remaining,
        plan: info.plan,
        needs_revalidation: false,
    })
}

/// Deactivate the license: delete the local file and notify the server.
pub fn deactivate_license() -> Result<()> {
    let info = load_license()?;

    if let Some(info) = &info {
        // Try to notify the server (best-effort, don't fail if server is unreachable)
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build();

        if let Ok(client) = client {
            let request_body = DeactivateRequest {
                key: info.key.clone(),
            };
            // Ignore errors — deactivation should succeed locally even if server is down
            let _ = client.post(DEACTIVATE_URL).json(&request_body).send();
        }
    }

    // Delete the license file
    let path = license_path()?;
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("Failed to remove license file at {}", path.display()))?;
    }

    Ok(())
}

/// Mask a license key for display: ROM-XXXX-****-****-XXXX
/// Shows first and last segments, masks middle two.
pub fn mask_key(key: &str) -> String {
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() == 5 {
        format!("{}-{}-****-****-{}", parts[0], parts[1], parts[4])
    } else {
        // Fallback if format doesn't match
        "****".to_string()
    }
}
