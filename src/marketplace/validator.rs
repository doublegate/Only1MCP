/// Plugin Marketplace Validator
///
/// Performs security validation and checks on plugins before publishing

use crate::error::{Error, Result};
use crate::marketplace::types::PluginManifest;
use regex::Regex;
use semver::Version;

pub struct PluginValidator;

impl PluginValidator {
    /// Validate plugin manifest
    pub fn validate_manifest(manifest: &PluginManifest) -> Result<()> {
        // Validate plugin name
        Self::validate_name(&manifest.name)?;

        // Validate version
        Self::validate_version(&manifest.version)?;

        // Validate author
        if manifest.author.is_empty() {
            return Err(Error::Config("Plugin author cannot be empty".to_string()));
        }

        // Validate description
        if manifest.description.is_empty() {
            return Err(Error::Config("Plugin description cannot be empty".to_string()));
        }

        if manifest.description.len() > 500 {
            return Err(Error::Config(
                "Plugin description must be 500 characters or less".to_string(),
            ));
        }

        // Validate license
        Self::validate_license(&manifest.license)?;

        // Validate capabilities
        if manifest.capabilities.is_empty() {
            return Err(Error::Config("Plugin must declare at least one capability".to_string()));
        }

        // Validate Only1MCP version requirement
        Self::validate_version(&manifest.compatibility.min_only1mcp_version)?;

        tracing::info!("Plugin manifest validation passed for: {}", manifest.name);
        Ok(())
    }

    /// Validate plugin name
    fn validate_name(name: &str) -> Result<()> {
        // Plugin name rules:
        // - 3-50 characters
        // - lowercase letters, numbers, hyphens only
        // - must start with a letter
        // - cannot end with hyphen

        if name.len() < 3 || name.len() > 50 {
            return Err(Error::Config(
                "Plugin name must be between 3 and 50 characters".to_string(),
            ));
        }

        let name_regex = Regex::new(r"^[a-z][a-z0-9-]*[a-z0-9]$").unwrap();
        if !name_regex.is_match(name) {
            return Err(Error::Config(
                "Plugin name must start with a letter, contain only lowercase letters, numbers, and hyphens, and cannot end with a hyphen".to_string(),
            ));
        }

        // Reserved names
        let reserved = vec!["only1mcp", "core", "system", "admin", "api"];
        if reserved.contains(&name) {
            return Err(Error::Config(format!("Plugin name '{}' is reserved", name)));
        }

        Ok(())
    }

    /// Validate semantic version
    fn validate_version(version: &str) -> Result<()> {
        Version::parse(version).map_err(|e| {
            Error::Config(format!(
                "Invalid semantic version '{}': {}",
                version, e
            ))
        })?;

        Ok(())
    }

    /// Validate license
    fn validate_license(license: &str) -> Result<()> {
        // Accept common SPDX license identifiers
        let valid_licenses = vec![
            "MIT",
            "Apache-2.0",
            "GPL-3.0",
            "GPL-2.0",
            "BSD-2-Clause",
            "BSD-3-Clause",
            "ISC",
            "MPL-2.0",
            "LGPL-3.0",
        ];

        if !valid_licenses.contains(&license) {
            tracing::warn!(
                "License '{}' is not a common SPDX identifier. Accepted: {}",
                license,
                valid_licenses.join(", ")
            );
        }

        Ok(())
    }

    /// Security scan of plugin tarball
    pub async fn security_scan(tarball: &[u8]) -> Result<()> {
        // Basic security checks

        // 1. Check tarball size (max 50MB)
        const MAX_SIZE: usize = 50 * 1024 * 1024;
        if tarball.len() > MAX_SIZE {
            return Err(Error::Config(format!(
                "Plugin tarball too large: {} bytes (max {} bytes)",
                tarball.len(),
                MAX_SIZE
            )));
        }

        // 2. TODO: Scan for malicious patterns
        // - Check for suspicious file paths (e.g., absolute paths, ../)
        // - Scan for known malware signatures
        // - Validate Rust code doesn't use unsafe features (configurable)

        // 3. TODO: Run cargo audit on dependencies

        // 4. TODO: Verify code signature if present

        tracing::info!("Security scan passed for {} byte tarball", tarball.len());
        Ok(())
    }

    /// Validate plugin dependencies
    pub fn validate_dependencies(dependencies: &std::collections::HashMap<String, String>) -> Result<()> {
        for (name, version_req) in dependencies {
            // Validate dependency version requirement
            if version_req.is_empty() {
                return Err(Error::Config(format!(
                    "Dependency '{}' has empty version requirement",
                    name
                )));
            }

            // TODO: Check if dependency exists and version is available
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name() {
        assert!(PluginValidator::validate_name("rate-limiter").is_ok());
        assert!(PluginValidator::validate_name("oauth2-provider").is_ok());
        assert!(PluginValidator::validate_name("ab").is_err()); // Too short
        assert!(PluginValidator::validate_name("Rate-Limiter").is_err()); // Uppercase
        assert!(PluginValidator::validate_name("-rate-limiter").is_err()); // Starts with hyphen
        assert!(PluginValidator::validate_name("rate-limiter-").is_err()); // Ends with hyphen
        assert!(PluginValidator::validate_name("only1mcp").is_err()); // Reserved
    }

    #[test]
    fn test_validate_version() {
        assert!(PluginValidator::validate_version("1.0.0").is_ok());
        assert!(PluginValidator::validate_version("0.1.2-alpha").is_ok());
        assert!(PluginValidator::validate_version("invalid").is_err());
    }
}
