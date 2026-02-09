//! Bot configuration
//!
//! Loads configuration from environment variables

use anyhow::Result;
use std::ops::Deref;
use televent_core::config::CoreConfig;

/// Bot configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    /// Core configuration (database, bot token)
    pub core: CoreConfig,
}

impl Config {
    /// Load configuration from environment variables
    #[allow(dead_code)]
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            core: CoreConfig::from_env()?,
        })
    }
}

impl Deref for Config {
    type Target = CoreConfig;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_structure() {
        // Verify Config implements required traits
        fn assert_clone<T: Clone>() {}
        fn assert_debug<T: std::fmt::Debug>() {}

        assert_clone::<Config>();
        assert_debug::<Config>();
    }
}
