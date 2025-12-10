//! # Utilities Module
//!
//! This module contains helper functions and utilities used
//! across the backend service.

use chrono::{DateTime, Utc};

/// Format a balance value as human-readable USDT.
///
/// Converts from smallest units (6 decimals) to readable format.
///
/// ## Arguments
///
/// * `amount` - Balance in smallest units
///
/// ## Returns
///
/// Formatted string like "1,234.56 USDT"
///
/// ## Examples
///
/// ```rust
/// assert_eq!(format_usdt(1_000_000), "1.00 USDT");
/// assert_eq!(format_usdt(1_234_567_890), "1,234.57 USDT");
/// ```
#[allow(dead_code)]
pub fn format_usdt(amount: i64) -> String {
    let usdt = amount as f64 / 1_000_000.0;
    
    // Format with thousands separator
    let formatted = if usdt >= 1000.0 {
        let whole = usdt as u64;
        let frac = ((usdt - whole as f64) * 100.0).round() as u64;
        
        // Add commas
        let whole_str = whole.to_string();
        let mut result = String::new();
        for (i, c) in whole_str.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                result.push(',');
            }
            result.push(c);
        }
        let reversed: String = result.chars().rev().collect();
        format!("{}.{:02} USDT", reversed, frac)
    } else {
        format!("{:.2} USDT", usdt)
    };
    
    formatted
}

/// Parse a USDT amount string to smallest units.
///
/// ## Arguments
///
/// * `amount_str` - Amount string like "100.50"
///
/// ## Returns
///
/// Amount in smallest units (6 decimals).
///
/// ## Examples
///
/// ```rust
/// assert_eq!(parse_usdt("100.00")?, 100_000_000);
/// assert_eq!(parse_usdt("1.5")?, 1_500_000);
/// ```
#[allow(dead_code)]
pub fn parse_usdt(amount_str: &str) -> Result<u64, String> {
    let cleaned = amount_str.replace(",", "").replace(" USDT", "");
    
    let amount: f64 = cleaned
        .parse()
        .map_err(|_| format!("Invalid amount: {}", amount_str))?;
    
    if amount < 0.0 {
        return Err("Amount cannot be negative".to_string());
    }
    
    Ok((amount * 1_000_000.0) as u64)
}

/// Validate a Solana public key.
///
/// ## Arguments
///
/// * `pubkey` - Base58-encoded public key
///
/// ## Returns
///
/// * `Ok(())` - Valid public key
/// * `Err(String)` - Invalid format
#[allow(dead_code)]
pub fn validate_pubkey(pubkey: &str) -> Result<(), String> {
    // Base58 characters
    const BASE58_CHARS: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    
    // Check length (32-44 characters for Solana pubkeys)
    if pubkey.len() < 32 || pubkey.len() > 44 {
        return Err(format!(
            "Invalid pubkey length: {} (expected 32-44)",
            pubkey.len()
        ));
    }
    
    // Check all characters are valid base58
    for c in pubkey.chars() {
        if !BASE58_CHARS.contains(c) {
            return Err(format!("Invalid character in pubkey: {}", c));
        }
    }
    
    Ok(())
}

/// Calculate percentage change between two values.
///
/// ## Arguments
///
/// * `old_value` - Previous value
/// * `new_value` - Current value
///
/// ## Returns
///
/// Percentage change as a float (e.g., 10.5 for 10.5% increase).
#[allow(dead_code)]
pub fn calculate_percentage_change(old_value: i64, new_value: i64) -> f64 {
    if old_value == 0 {
        if new_value == 0 {
            return 0.0;
        }
        return 100.0; // Infinite increase, cap at 100%
    }
    
    ((new_value - old_value) as f64 / old_value as f64) * 100.0
}

/// Format a timestamp as ISO 8601.
#[allow(dead_code)]
pub fn format_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp.to_rfc3339()
}

/// Parse an ISO 8601 timestamp.
#[allow(dead_code)]
pub fn parse_timestamp(timestamp_str: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(timestamp_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| format!("Invalid timestamp: {}", e))
}

/// Truncate a string to a maximum length.
///
/// Useful for logging long public keys.
#[allow(dead_code)]
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let half = (max_len - 3) / 2;
        format!("{}...{}", &s[..half], &s[s.len() - half..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_usdt() {
        assert_eq!(format_usdt(1_000_000), "1.00 USDT");
        assert_eq!(format_usdt(0), "0.00 USDT");
        assert_eq!(format_usdt(500_000), "0.50 USDT");
    }

    #[test]
    fn test_parse_usdt() {
        assert_eq!(parse_usdt("100.00").unwrap(), 100_000_000);
        assert_eq!(parse_usdt("1.5").unwrap(), 1_500_000);
        assert_eq!(parse_usdt("0").unwrap(), 0);
    }

    #[test]
    fn test_validate_pubkey() {
        // Valid pubkey
        assert!(validate_pubkey("7xKt9Fj2abc123def456ghi789jkl012mno345").is_ok());
        
        // Too short
        assert!(validate_pubkey("7xKt9Fj2").is_err());
        
        // Invalid character (0)
        assert!(validate_pubkey("0xKt9Fj2abc123def456ghi789jkl012mno345").is_err());
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("abcdefghij", 10), "abcdefghij");
        assert_eq!(truncate_string("abcdefghijklmnop", 10), "abc...nop");
    }
}

