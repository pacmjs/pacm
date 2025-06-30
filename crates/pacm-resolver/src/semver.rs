use std::collections::HashMap;

use crate::comparators::{Comparator, Range};
use crate::version_utils::parse_partial_version;

/// Parse npm-style semver ranges with multiple comparators and OR logic
pub fn parse_npm_semver_ranges(range_str: &str) -> Result<Vec<Range>, String> {
    let range_str = range_str.trim();

    // Handle empty or wildcard ranges
    if range_str.is_empty() || range_str == "*" {
        return Ok(vec![Range::new(vec![Comparator::Wildcard])]);
    }

    // Split on "||" for OR logic
    let or_clauses: Vec<&str> = range_str.split("||").map(|s| s.trim()).collect();
    let mut ranges = Vec::new();

    for clause in or_clauses {
        let clause = clause.trim();
        if clause.is_empty() {
            continue;
        }

        // Parse individual clause (AND logic within each clause)
        let range = parse_range_clause(clause)?;
        ranges.push(range);
    }

    if ranges.is_empty() {
        return Ok(vec![Range::new(vec![Comparator::Wildcard])]);
    }

    Ok(ranges)
}

/// Parse a single range clause (e.g., ">=1.2.3 <2.0.0")
fn parse_range_clause(clause: &str) -> Result<Range, String> {
    let clause = clause.trim();

    // Handle special cases
    if clause == "*" || clause.is_empty() {
        return Ok(Range::new(vec![Comparator::Wildcard]));
    }

    let mut comparators = Vec::new();
    let mut remaining = clause;

    while !remaining.is_empty() {
        remaining = remaining.trim();
        if remaining.is_empty() {
            break;
        }

        // Try to match operators
        if let Some(rest) = remaining.strip_prefix(">=") {
            let (version_str, next) = extract_version_and_remaining(rest)?;
            let version = parse_partial_version(&version_str)?;
            comparators.push(Comparator::GreaterThanOrEqual(version));
            remaining = next;
        } else if let Some(rest) = remaining.strip_prefix("<=") {
            let (version_str, next) = extract_version_and_remaining(rest)?;
            let version = parse_partial_version(&version_str)?;
            comparators.push(Comparator::LessThanOrEqual(version));
            remaining = next;
        } else if let Some(rest) = remaining.strip_prefix(">") {
            let (version_str, next) = extract_version_and_remaining(rest)?;
            let version = parse_partial_version(&version_str)?;
            comparators.push(Comparator::GreaterThan(version));
            remaining = next;
        } else if let Some(rest) = remaining.strip_prefix("<") {
            let (version_str, next) = extract_version_and_remaining(rest)?;
            let version = parse_partial_version(&version_str)?;
            comparators.push(Comparator::LessThan(version));
            remaining = next;
        } else if let Some(rest) = remaining.strip_prefix("^") {
            let (version_str, next) = extract_version_and_remaining(rest)?;
            let version = parse_partial_version(&version_str)?;
            comparators.push(Comparator::Compatible(version));
            remaining = next;
        } else if let Some(rest) = remaining.strip_prefix("~") {
            let (version_str, next) = extract_version_and_remaining(rest)?;
            let version = parse_partial_version(&version_str)?;
            comparators.push(Comparator::Tilde(version));
            remaining = next;
        } else if let Some(rest) = remaining.strip_prefix("=") {
            let (version_str, next) = extract_version_and_remaining(rest)?;
            let version = parse_partial_version(&version_str)?;
            comparators.push(Comparator::Exact(version));
            remaining = next;
        } else {
            // Try to parse as a plain version (no operator prefix)
            let (version_str, next) = extract_version_and_remaining(remaining)?;
            let version = parse_partial_version(&version_str)?;
            comparators.push(Comparator::Exact(version));
            remaining = next;
        }
    }

    if comparators.is_empty() {
        return Ok(Range::new(vec![Comparator::Wildcard]));
    }

    Ok(Range::new(comparators))
}

/// Extract a version string and return the remaining input
fn extract_version_and_remaining(input: &str) -> Result<(String, &str), String> {
    let input = input.trim_start();

    if input.is_empty() {
        return Err("Expected version string but found end of input".to_string());
    }

    // Find the end of the version by looking for the next operator or end of string
    let mut end_pos = 0;
    let chars: Vec<char> = input.chars().collect();

    // Look for the next operator or end of string
    while end_pos < chars.len() {
        let current_char = chars[end_pos];
        if current_char.is_whitespace() {
            break;
        }
        // Check if we're at the start of a new operator
        if end_pos > 0 && ['>', '<', '=', '^', '~'].contains(&current_char) {
            break;
        }
        end_pos += 1;
    }

    let version_str = input[..end_pos].trim().to_string();
    let remaining = &input[end_pos..];

    if version_str.is_empty() {
        return Err("Empty version string found".to_string());
    }

    Ok((version_str, remaining))
}

/// Resolve version from available versions and range
pub fn resolve_version(
    available_versions: &serde_json::Value,
    range: &str,
    dist_tags: &HashMap<String, String>,
) -> Result<String, String> {
    use semver::Version;

    // If the range matches a dist-tag, return the version for that tag
    if let Some(tag_version) = dist_tags.get(range) {
        return Ok(tag_version.clone());
    }

    let ranges = parse_npm_semver_ranges(range)?;

    // Collect all versions as (Version, String) pairs
    let mut candidates: Vec<(Version, String)> = available_versions
        .as_object()
        .ok_or("Invalid versions object")?
        .keys()
        .filter_map(|v_str| {
            Version::parse(v_str).ok().map(|v| (v, v_str.clone()))
        })
        .collect();

    // Sort descending (highest version first)
    candidates.sort_by(|a, b| b.0.cmp(&a.0));

    // If the range does not allow pre-releases, filter them out unless explicitly matched
    let allows_prerelease = range.contains('-');
    let filtered: Vec<(Version, String)> = candidates
        .into_iter()
        .filter(|(v, _)| {
            if !allows_prerelease && !v.pre.is_empty() {
                false
            } else {
                ranges.iter().any(|range| range.matches(v))
            }
        })
        .collect();

    if let Some((_, v_str)) = filtered.first() {
        Ok(v_str.clone())
    } else {
        Err(format!("No matching version found for range '{}'", range))
    }
}
