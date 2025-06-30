pub fn format_version_string(version: &str, save_exact: bool) -> String {
    if save_exact {
        version.to_string()
    } else if version.starts_with('^') || version.starts_with('~') || version.contains('-') {
        version.to_string()
    } else {
        format!("^{}", version)
    }
}

pub fn is_exact_version(version: &str) -> bool {
    !version.starts_with('^') && !version.starts_with('~') && !version.contains('-')
}

pub fn extract_exact_version(version: &str) -> String {
    if version.starts_with('^') || version.starts_with('~') {
        version[1..].to_string()
    } else {
        version.to_string()
    }
}
