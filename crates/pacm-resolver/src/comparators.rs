use semver::Version;

#[derive(Debug, Clone)]
pub enum Comparator {
    Exact(Version),
    GreaterThan(Version),
    GreaterThanOrEqual(Version),
    LessThan(Version),
    LessThanOrEqual(Version),
    Compatible(Version), // ^
    Tilde(Version),      // ~
    Wildcard,            // *
}

impl Comparator {
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            Comparator::Exact(v) => version == v,
            Comparator::GreaterThan(v) => version > v,
            Comparator::GreaterThanOrEqual(v) => version >= v,
            Comparator::LessThan(v) => version < v,
            Comparator::LessThanOrEqual(v) => version <= v,
            Comparator::Wildcard => true,
            Comparator::Compatible(v) => {
                // ^1.2.3 := >=1.2.3 <2.0.0 (Same major version)
                // ^0.2.3 := >=0.2.3 <0.3.0 (Same minor version if major is 0)
                // ^0.0.3 := >=0.0.3 <0.0.4 (Same patch version if major and minor are 0)
                if version < v {
                    return false;
                }
                if v.major > 0 {
                    version.major == v.major
                } else if v.minor > 0 {
                    version.major == 0 && version.minor == v.minor
                } else {
                    version.major == 0 && version.minor == 0 && version.patch == v.patch
                }
            }
            Comparator::Tilde(v) => {
                // ~1.2.3 := >=1.2.3 <1.3.0 (Same major and minor version)
                // ~1.2 := >=1.2.0 <1.3.0
                // ~1 := >=1.0.0 <2.0.0
                if version < v {
                    return false;
                }
                version.major == v.major && version.minor == v.minor
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Range {
    pub comparators: Vec<Comparator>,
}

impl Range {
    pub fn new(comparators: Vec<Comparator>) -> Self {
        Self { comparators }
    }

    pub fn matches(&self, version: &Version) -> bool {
        if self.comparators.is_empty() {
            return true;
        }
        self.comparators.iter().all(|comp| comp.matches(version))
    }
}
