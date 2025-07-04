use std::env;

pub fn is_platform_compatible(
    os_list: &Option<Vec<String>>,
    cpu_list: &Option<Vec<String>>,
) -> bool {
    // If no platform restrictions, assume compatible
    if os_list.is_none() && cpu_list.is_none() {
        return true;
    }

    let current_os = get_current_os();
    let current_cpu = get_current_cpu();

    if let Some(os_requirements) = os_list {
        if !os_requirements.is_empty() {
            if !is_platform_field_compatible(&current_os, os_requirements) {
                return false;
            }
        }
    }

    if let Some(cpu_requirements) = cpu_list {
        if !cpu_requirements.is_empty() {
            if !is_platform_field_compatible(&current_cpu, cpu_requirements) {
                return false;
            }
        }
    }

    true
}

fn is_platform_field_compatible(current_platform: &str, requirements: &[String]) -> bool {
    let mut has_allow_list = false;
    let mut allowed = false;
    let mut blocked = false;

    for requirement in requirements {
        if let Some(blocked_platform) = requirement.strip_prefix('!') {
            // This is a blocked platform
            if current_platform == blocked_platform {
                blocked = true;
            }
        } else {
            // This is an allowed platform
            has_allow_list = true;
            if current_platform == requirement {
                allowed = true;
            }
        }
    }

    if blocked {
        return false;
    }

    if has_allow_list && !allowed {
        return false;
    }

    true
}

pub fn get_current_os() -> String {
    match env::consts::OS {
        "windows" => "win32".to_string(),
        "macos" => "darwin".to_string(),
        "linux" => "linux".to_string(),
        "freebsd" => "freebsd".to_string(),
        "netbsd" => "netbsd".to_string(),
        "openbsd" => "openbsd".to_string(),
        "dragonfly" => "dragonfly".to_string(),
        "solaris" => "sunos".to_string(),
        other => other.to_string(),
    }
}

pub fn get_current_cpu() -> String {
    match env::consts::ARCH {
        "x86_64" => "x64".to_string(),
        "x86" => "ia32".to_string(),
        "aarch64" => "arm64".to_string(),
        "arm" => "arm".to_string(),
        "mips" => "mips".to_string(),
        "mips64" => "mips64".to_string(),
        "powerpc" => "ppc".to_string(),
        "powerpc64" => "ppc64".to_string(),
        "s390x" => "s390x".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_restrictions() {
        assert!(is_platform_compatible(&None, &None));
    }

    #[test]
    fn test_empty_restrictions() {
        assert!(is_platform_compatible(&Some(vec![]), &Some(vec![])));
    }

    #[test]
    fn test_current_platform_detection() {
        let current_os = get_current_os();
        let current_cpu = get_current_cpu();

        // Should be compatible with itself
        assert!(is_platform_compatible(
            &Some(vec![current_os]),
            &Some(vec![current_cpu])
        ));
    }

    #[test]
    fn test_incompatible_os_allow_list() {
        let fake_os = vec!["nonexistent-os".to_string()];
        assert!(!is_platform_compatible(&Some(fake_os), &None));
    }

    #[test]
    fn test_incompatible_cpu_allow_list() {
        let fake_cpu = vec!["nonexistent-cpu".to_string()];
        assert!(!is_platform_compatible(&None, &Some(fake_cpu)));
    }

    #[test]
    fn test_os_block_list_current_platform() {
        let current_os = get_current_os();
        let blocked_os = vec![format!("!{}", current_os)];

        assert!(!is_platform_compatible(&Some(blocked_os), &None));
    }

    #[test]
    fn test_os_block_list_other_platform() {
        let blocked_os = vec!["!nonexistent-os".to_string()];

        assert!(is_platform_compatible(&Some(blocked_os), &None));
    }

    #[test]
    fn test_cpu_block_list_current_platform() {
        let current_cpu = get_current_cpu();
        let blocked_cpu = vec![format!("!{}", current_cpu)];

        assert!(!is_platform_compatible(&None, &Some(blocked_cpu)));
    }

    #[test]
    fn test_cpu_block_list_other_platform() {
        let blocked_cpu = vec!["!nonexistent-cpu".to_string()];

        assert!(is_platform_compatible(&None, &Some(blocked_cpu)));
    }

    #[test]
    fn test_mixed_allow_and_block() {
        let current_os = get_current_os();

        let mixed_os = vec![current_os.clone(), "!nonexistent-os".to_string()];
        assert!(is_platform_compatible(&Some(mixed_os), &None));

        let blocked_current = vec![current_os.clone(), format!("!{}", current_os)];
        assert!(!is_platform_compatible(&Some(blocked_current), &None));
    }

    #[test]
    fn test_multiple_blocks() {
        let multiple_blocks = vec!["!win32".to_string(), "!darwin".to_string()];
        let current_os = get_current_os();

        if current_os == "win32" || current_os == "darwin" {
            assert!(!is_platform_compatible(&Some(multiple_blocks), &None));
        } else {
            assert!(is_platform_compatible(&Some(multiple_blocks), &None));
        }
    }

    #[test]
    fn test_multiple_allows() {
        let multiple_allows = vec!["darwin".to_string(), "linux".to_string()];
        let current_os = get_current_os();

        if current_os == "darwin" || current_os == "linux" {
            assert!(is_platform_compatible(&Some(multiple_allows), &None));
        } else {
            assert!(!is_platform_compatible(&Some(multiple_allows), &None));
        }
    }

    #[test]
    fn test_platform_field_compatibility() {
        assert!(is_platform_field_compatible(
            "darwin",
            &["darwin".to_string(), "linux".to_string()]
        ));
        assert!(!is_platform_field_compatible(
            "win32",
            &["darwin".to_string(), "linux".to_string()]
        ));

        assert!(!is_platform_field_compatible(
            "win32",
            &["!win32".to_string()]
        ));
        assert!(is_platform_field_compatible(
            "darwin",
            &["!win32".to_string()]
        ));

        assert!(!is_platform_field_compatible(
            "darwin",
            &["darwin".to_string(), "!darwin".to_string()]
        ));
        assert!(is_platform_field_compatible(
            "linux",
            &[
                "darwin".to_string(),
                "linux".to_string(),
                "!win32".to_string()
            ]
        ));
        assert!(!is_platform_field_compatible(
            "win32",
            &[
                "darwin".to_string(),
                "linux".to_string(),
                "!win32".to_string()
            ]
        ));
    }
}
