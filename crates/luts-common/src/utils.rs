//! Common utility functions used across LUTS components

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

/// Generate a unique ID for various entities
pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

/// Generate a short ID for display purposes (8 characters)
pub fn generate_short_id() -> String {
    Uuid::new_v4().to_string()[..8].to_string()
}

/// Generate a timestamped ID with optional prefix
pub fn generate_timestamped_id(prefix: Option<&str>) -> String {
    let timestamp = Utc::now().timestamp();
    let short_uuid = Uuid::new_v4().to_string()[..8].to_string();
    
    match prefix {
        Some(p) => format!("{}_{}_{}", p, timestamp, short_uuid),
        None => format!("{}_{}", timestamp, short_uuid),
    }
}

/// Get current UTC timestamp as milliseconds
pub fn current_timestamp_millis() -> i64 {
    Utc::now().timestamp_millis()
}

/// Get current UTC timestamp as seconds
pub fn current_timestamp_secs() -> i64 {
    Utc::now().timestamp()
}

/// Convert timestamp milliseconds to DateTime
pub fn timestamp_millis_to_datetime(millis: i64) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp_millis(millis)
}

/// Convert timestamp seconds to DateTime
pub fn timestamp_secs_to_datetime(secs: i64) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp(secs, 0)
}

/// Format bytes into human-readable string
pub fn format_bytes(bytes: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Format duration in seconds to human-readable string
pub fn format_duration_secs(seconds: f64) -> String {
    if seconds < 1.0 {
        format!("{:.0}ms", seconds * 1000.0)
    } else if seconds < 60.0 {
        format!("{:.1}s", seconds)
    } else if seconds < 3600.0 {
        let minutes = seconds / 60.0;
        format!("{:.1}m", minutes)
    } else {
        let hours = seconds / 3600.0;
        format!("{:.1}h", hours)
    }
}

/// Validate file extension against allowed list
pub fn validate_file_extension(filename: &str, allowed_extensions: &[&str]) -> bool {
    if let Some(extension) = std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
    {
        allowed_extensions.iter().any(|&allowed| allowed.eq_ignore_ascii_case(extension))
    } else {
        false
    }
}

/// Sanitize filename by removing/replacing invalid characters
pub fn sanitize_filename(filename: &str) -> String {
    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let mut sanitized = filename.to_string();
    
    for &invalid_char in &invalid_chars {
        sanitized = sanitized.replace(invalid_char, "_");
    }
    
    // Remove leading/trailing whitespace and dots
    sanitized = sanitized.trim().trim_matches('.').to_string();
    
    // Ensure filename is not empty
    if sanitized.is_empty() {
        sanitized = "unnamed".to_string();
    }
    
    sanitized
}

/// Merge two HashMaps, with values from the second map taking precedence
pub fn merge_metadata(base: HashMap<String, String>, overlay: HashMap<String, String>) -> HashMap<String, String> {
    let mut result = base;
    for (key, value) in overlay {
        result.insert(key, value);
    }
    result
}

/// Truncate string to specified length with ellipsis
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        "...".to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Calculate percentage with bounds checking
pub fn calculate_percentage(part: f64, total: f64) -> f64 {
    if total <= 0.0 {
        0.0
    } else {
        (part / total * 100.0).min(100.0).max(0.0)
    }
}

/// Create a progress bar string
pub fn format_progress_bar(percentage: f64, width: usize) -> String {
    let filled = ((percentage / 100.0) * width as f64) as usize;
    let empty = width.saturating_sub(filled);
    
    format!(
        "[{}{}] {:.1}%",
        "█".repeat(filled),
        "░".repeat(empty),
        percentage
    )
}

/// Generate a consistent hash for a string (useful for consistent coloring, etc.)
pub fn string_hash(s: &str) -> u32 {
    let mut hash = 0u32;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ids() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36); // UUID length
        
        let short_id = generate_short_id();
        assert_eq!(short_id.len(), 8);
        
        let timestamped = generate_timestamped_id(Some("test"));
        assert!(timestamped.starts_with("test_"));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration_secs(0.5), "500ms");
        assert_eq!(format_duration_secs(1.5), "1.5s");
        assert_eq!(format_duration_secs(90.0), "1.5m");
        assert_eq!(format_duration_secs(3660.0), "1.0h");
    }

    #[test]
    fn test_validate_file_extension() {
        assert!(validate_file_extension("test.json", &["json", "yaml"]));
        assert!(validate_file_extension("test.JSON", &["json", "yaml"]));
        assert!(!validate_file_extension("test.txt", &["json", "yaml"]));
        assert!(!validate_file_extension("test", &["json", "yaml"]));
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("valid_name.txt"), "valid_name.txt");
        assert_eq!(sanitize_filename("in<valid>name"), "in_valid_name");
        assert_eq!(sanitize_filename("  .hidden  "), "hidden");
        assert_eq!(sanitize_filename(""), "unnamed");
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 8), "hello...");
        assert_eq!(truncate_string("hi", 2), "hi");
        assert_eq!(truncate_string("hi", 1), "...");
    }

    #[test]
    fn test_calculate_percentage() {
        assert_eq!(calculate_percentage(50.0, 100.0), 50.0);
        assert_eq!(calculate_percentage(150.0, 100.0), 100.0);
        assert_eq!(calculate_percentage(50.0, 0.0), 0.0);
        assert_eq!(calculate_percentage(-10.0, 100.0), 0.0);
    }

    #[test]
    fn test_timestamp_functions() {
        let millis = current_timestamp_millis();
        let secs = current_timestamp_secs();
        
        assert!(millis > 0);
        assert!(secs > 0);
        assert!(millis > secs); // millis should be much larger
        
        let dt_from_millis = timestamp_millis_to_datetime(millis);
        assert!(dt_from_millis.is_some());
        
        let dt_from_secs = timestamp_secs_to_datetime(secs);
        assert!(dt_from_secs.is_some());
    }

    #[test]
    fn test_merge_metadata() {
        let mut base = HashMap::new();
        base.insert("key1".to_string(), "value1".to_string());
        base.insert("key2".to_string(), "value2".to_string());
        
        let mut overlay = HashMap::new();
        overlay.insert("key2".to_string(), "new_value2".to_string());
        overlay.insert("key3".to_string(), "value3".to_string());
        
        let result = merge_metadata(base, overlay);
        
        assert_eq!(result.get("key1"), Some(&"value1".to_string()));
        assert_eq!(result.get("key2"), Some(&"new_value2".to_string())); // overridden
        assert_eq!(result.get("key3"), Some(&"value3".to_string()));
    }

    #[test]
    fn test_string_hash() {
        let hash1 = string_hash("test");
        let hash2 = string_hash("test");
        let hash3 = string_hash("different");
        
        assert_eq!(hash1, hash2); // consistent
        assert_ne!(hash1, hash3); // different for different strings
    }
}