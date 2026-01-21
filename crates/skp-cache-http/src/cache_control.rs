use std::time::Duration;

/// Parsed Cache-Control header directives
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheControl {
    /// Max age in seconds
    pub max_age: Option<Duration>,
    /// S-Maxage (shared cache max age)
    pub s_maxage: Option<Duration>,
    /// No-cache directive
    pub no_cache: bool,
    /// No-store directive
    pub no_store: bool,
    /// Private directive
    pub private: bool,
    /// Public directive
    pub public: bool,
    /// Must-revalidate directive
    pub must_revalidate: bool,
    /// Stale-while-revalidate window
    pub stale_while_revalidate: Option<Duration>,
}

impl CacheControl {
    /// Parse from header string value
    pub fn parse(header: &str) -> Self {
        let mut cc = Self::default();
        for directive in header.split(',') {
             let directive = directive.trim();
             if directive.eq_ignore_ascii_case("no-cache") { cc.no_cache = true; }
             else if directive.eq_ignore_ascii_case("no-store") { cc.no_store = true; }
             else if directive.eq_ignore_ascii_case("private") { cc.private = true; }
             else if directive.eq_ignore_ascii_case("public") { cc.public = true; }
             else if directive.eq_ignore_ascii_case("must-revalidate") { cc.must_revalidate = true; }
             else if let Some(val) = directive.strip_prefix("max-age=") {
                 if let Ok(secs) = val.parse::<u64>() { cc.max_age = Some(Duration::from_secs(secs)); }
             }
             else if let Some(val) = directive.strip_prefix("s-maxage=") {
                 if let Ok(secs) = val.parse::<u64>() { cc.s_maxage = Some(Duration::from_secs(secs)); }
             }
             else if let Some(val) = directive.strip_prefix("stale-while-revalidate=") {
                 if let Ok(secs) = val.parse::<u64>() { cc.stale_while_revalidate = Some(Duration::from_secs(secs)); }
             }
        }
        cc
    }
}
