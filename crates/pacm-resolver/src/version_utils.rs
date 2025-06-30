use semver::Version;

pub fn parse_partial_version(version_str: &str) -> Result<Version, String> {
    let cleaned = version_str.trim();

    if cleaned == "*" || cleaned == "" {
        return Ok(Version::new(0, 0, 0));
    }

    if let Ok(version) = Version::parse(cleaned) {
        return Ok(version);
    }

    let parts: Vec<&str> = cleaned.split('.').collect();
    match parts.len() {
        1 => {
            let major = parts[0]
                .parse::<u64>()
                .map_err(|_| format!("Invalid major version: {}", parts[0]))?;
            Ok(Version::new(major, 0, 0))
        }
        2 => {
            let major = parts[0]
                .parse::<u64>()
                .map_err(|_| format!("Invalid major version: {}", parts[0]))?;
            let minor = parts[1]
                .parse::<u64>()
                .map_err(|_| format!("Invalid minor version: {}", parts[1]))?;
            Ok(Version::new(major, minor, 0))
        }
        _ => {
            let extended = if !cleaned.contains('.') {
                format!("{}.0.0", cleaned)
            } else if cleaned.matches('.').count() == 1 {
                format!("{}.0", cleaned)
            } else {
                cleaned.to_string()
            };
            Version::parse(&extended).map_err(|e| format!("Invalid version '{}': {}", cleaned, e))
        }
    }
}
