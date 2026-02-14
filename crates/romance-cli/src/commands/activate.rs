use anyhow::{Result, bail};
use colored::*;
use romance_core::license;

/// Activate a license key by validating it online and saving it locally.
pub fn run_activate(key: &str) -> Result<()> {
    // Basic format validation
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() != 5 || parts[0] != "ROM" {
        bail!(
            "Invalid license key format. Expected: ROM-XXXX-XXXX-XXXX-XXXX\nGot: {}",
            key
        );
    }

    println!("{}", "Validating license key...".dimmed());

    let response = license::validate_online(key)?;

    if !response.valid {
        let error_msg = response
            .error
            .unwrap_or_else(|| "Unknown error".to_string());
        bail!("License activation failed: {}", error_msg);
    }

    let email = response
        .email
        .unwrap_or_else(|| "unknown".to_string());
    let plan = response
        .plan
        .unwrap_or_else(|| "unknown".to_string());
    let expires_at = response
        .expires_at
        .unwrap_or_else(|| "unknown".to_string());

    let now = chrono::Utc::now().to_rfc3339();

    let info = license::LicenseInfo {
        key: key.to_string(),
        email: email.clone(),
        plan: plan.clone(),
        activated_at: now.clone(),
        expires_at: expires_at.clone(),
        last_validated: now,
    };

    license::save_license(&info)?;

    println!();
    println!("{}", "License activated successfully!".green().bold());
    println!();
    println!("  {}  {}", "Email:".bold(), email);
    println!("  {}   {}", "Plan:".bold(), format_plan(&plan));
    println!("  {} {}", "Expires:".bold(), expires_at);
    println!();

    Ok(())
}

/// Show the current license status.
pub fn run_status() -> Result<()> {
    let info = license::load_license()?;

    match info {
        None => {
            println!(
                "{}",
                "No license activated. Run `romance activate <key>` to activate.".yellow()
            );
        }
        Some(info) => {
            let status = license::check_license();

            println!();
            println!("{}", "Romance License Status".bold().underline());
            println!();
            println!("  {}    {}", "Key:".bold(), license::mask_key(&info.key));
            println!("  {}  {}", "Email:".bold(), info.email);
            println!("  {}   {}", "Plan:".bold(), format_plan(&info.plan));
            println!("  {} {}", "Expires:".bold(), info.expires_at);

            match status {
                Ok(s) => {
                    let status_str = if s.expired {
                        "EXPIRED".red().bold().to_string()
                    } else if s.days_remaining <= 7 {
                        format!("{} ({} days remaining)", "EXPIRING SOON".yellow().bold(), s.days_remaining)
                    } else {
                        format!("{} ({} days remaining)", "ACTIVE".green().bold(), s.days_remaining)
                    };
                    println!("  {} {}", "Status:".bold(), status_str);

                    if s.needs_revalidation {
                        println!(
                            "  {}",
                            "Warning: License needs online revalidation. Connect to the internet."
                                .yellow()
                        );
                    }
                }
                Err(e) => {
                    println!("  {} {}", "Status:".bold(), format!("{}", e).red());
                }
            }

            println!();
        }
    }

    Ok(())
}

/// Deactivate and remove the license.
pub fn run_deactivate() -> Result<()> {
    let info = license::load_license()?;

    if info.is_none() {
        println!("{}", "No license is currently activated.".yellow());
        return Ok(());
    }

    println!("{}", "Deactivating license...".dimmed());

    license::deactivate_license()?;

    println!("{}", "License deactivated and removed.".green());

    Ok(())
}

/// Format the plan name for display with color.
fn format_plan(plan: &str) -> String {
    match plan {
        "starter" => "Starter".to_string(),
        "pro" => "Pro".cyan().bold().to_string(),
        "enterprise" => "Enterprise".magenta().bold().to_string(),
        other => other.to_string(),
    }
}
