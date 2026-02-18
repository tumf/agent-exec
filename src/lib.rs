/// agent-exec v0.1 — core library
///
/// Provides JSON output types, job-directory management, and the
/// implementation of the five sub-commands: run, status, tail, wait, kill.
pub mod schema;
pub mod jobstore;
pub mod run;
pub mod status;
pub mod tail;
pub mod wait;
pub mod kill;

// Legacy commands kept for backward compatibility during transition.
pub mod commands {
    /// Build a greeting string.
    pub fn greet(name: Option<&str>) -> String {
        let name = name
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("world");
        format!("Hello, {name}!")
    }

    /// Echo the input back as-is.
    pub fn echo(message: &str) -> String {
        message.to_string()
    }

    /// Return the crate version.
    pub fn version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn greet_defaults_to_world() {
            assert_eq!(greet(None), "Hello, world!");
            assert_eq!(greet(Some("")), "Hello, world!");
            assert_eq!(greet(Some("   ")), "Hello, world!");
        }

        #[test]
        fn greet_uses_name() {
            assert_eq!(greet(Some("Alice")), "Hello, Alice!");
        }

        #[test]
        fn echo_roundtrips() {
            assert_eq!(echo("hi"), "hi");
        }
    }
}
