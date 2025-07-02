#[must_use]
pub fn parse_pkg_spec(spec: &str) -> (String, String) {
    if spec.starts_with('@') {
        if let Some(scope_end) = spec[1..].find('/') {
            let scope_and_name_end = scope_end + 2;
            if let Some(version_start) = spec[scope_and_name_end..].find('@') {
                let name = spec[..scope_and_name_end + version_start].to_string();
                let version = spec[scope_and_name_end + version_start + 1..].to_string();
                (name, version)
            } else {
                (spec.to_string(), "latest".to_string())
            }
        } else {
            match spec.split_once('@') {
                Some((n, v)) if !n.is_empty() => (n.to_string(), v.to_string()),
                _ => (spec.to_string(), "latest".to_string()),
            }
        }
    } else {
        match spec.split_once('@') {
            Some((n, v)) if !n.is_empty() => (n.to_string(), v.to_string()),
            _ => (spec.to_string(), "latest".to_string()),
        }
    }
}
