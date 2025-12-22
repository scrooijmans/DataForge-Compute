//! UDF Registry for managing providers and UDFs.
//!
//! The registry is the central location for all registered UDFs.
//! Providers register themselves at startup, and the registry
//! provides lookup functionality for the execution engine.

use crate::compute::error::UdfError;
use crate::compute::{Udf, UdfProvider};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for UDF providers and their UDFs.
///
/// The registry maintains a collection of providers and their UDFs,
/// providing efficient lookup by full ID (provider:udf_id).
pub struct UdfRegistry {
    /// Registered providers by ID
    providers: HashMap<String, Arc<dyn UdfProvider>>,
    /// All UDFs indexed by full ID (provider:udf_id)
    udfs: HashMap<String, Arc<dyn Udf>>,
    /// UDF to provider mapping
    udf_providers: HashMap<String, String>,
}

impl Default for UdfRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl UdfRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            udfs: HashMap::new(),
            udf_providers: HashMap::new(),
        }
    }

    /// Register a provider and all its UDFs.
    ///
    /// This loads all UDFs from the provider and registers them
    /// with their full ID (provider:udf_id).
    pub fn register_provider(&mut self, provider: Arc<dyn UdfProvider>) -> Result<(), UdfError> {
        let provider_id = provider.id().to_string();

        // Check availability
        provider.is_available()?;

        // Check for duplicate provider
        if self.providers.contains_key(&provider_id) {
            return Err(UdfError::ProviderNotAvailable(format!(
                "Provider '{}' is already registered",
                provider_id
            )));
        }

        // Load and register UDFs
        let udfs = provider.load_udfs();
        for udf in udfs {
            let udf_id = udf.id().to_string();
            let full_id = format!("{}:{}", provider_id, udf_id);

            if self.udfs.contains_key(&full_id) {
                return Err(UdfError::ProviderNotAvailable(format!(
                    "UDF '{}' is already registered",
                    full_id
                )));
            }

            self.udfs.insert(full_id.clone(), udf);
            self.udf_providers.insert(full_id, provider_id.clone());
        }

        self.providers.insert(provider_id, provider);
        Ok(())
    }

    /// Get a UDF by its full ID (provider:udf_id).
    pub fn get_udf(&self, full_id: &str) -> Option<Arc<dyn Udf>> {
        self.udfs.get(full_id).cloned()
    }

    /// Get a provider by ID.
    pub fn get_provider(&self, provider_id: &str) -> Option<Arc<dyn UdfProvider>> {
        self.providers.get(provider_id).cloned()
    }

    /// Get the provider for a UDF.
    pub fn get_udf_provider(&self, full_id: &str) -> Option<Arc<dyn UdfProvider>> {
        self.udf_providers
            .get(full_id)
            .and_then(|pid| self.providers.get(pid))
            .cloned()
    }

    /// List all registered providers.
    pub fn list_providers(&self) -> Vec<ProviderInfo> {
        self.providers
            .values()
            .map(|p| ProviderInfo {
                id: p.id().to_string(),
                name: p.name().to_string(),
                version: p.version().to_string(),
                description: p.description().to_string(),
                udf_count: self.count_provider_udfs(p.id()),
            })
            .collect()
    }

    /// List all registered UDFs.
    pub fn list_udfs(&self) -> Vec<UdfInfo> {
        self.udfs
            .iter()
            .map(|(full_id, udf)| {
                let metadata = udf.metadata();
                let provider_id = self
                    .udf_providers
                    .get(full_id)
                    .cloned()
                    .unwrap_or_default();
                UdfInfo {
                    full_id: full_id.clone(),
                    provider_id,
                    name: metadata.name,
                    category: metadata.category,
                    description: metadata.description,
                    version: metadata.version,
                    tags: metadata.tags,
                }
            })
            .collect()
    }

    /// List UDFs from a specific provider.
    pub fn list_provider_udfs(&self, provider_id: &str) -> Vec<UdfInfo> {
        self.list_udfs()
            .into_iter()
            .filter(|u| u.provider_id == provider_id)
            .collect()
    }

    /// List UDFs by category.
    pub fn list_udfs_by_category(&self, category: &str) -> Vec<UdfInfo> {
        self.list_udfs()
            .into_iter()
            .filter(|u| u.category.eq_ignore_ascii_case(category))
            .collect()
    }

    /// Search UDFs by name or tag.
    pub fn search_udfs(&self, query: &str) -> Vec<UdfInfo> {
        let query_lower = query.to_lowercase();
        self.list_udfs()
            .into_iter()
            .filter(|u| {
                u.name.to_lowercase().contains(&query_lower)
                    || u.description.to_lowercase().contains(&query_lower)
                    || u.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Count UDFs for a provider.
    fn count_provider_udfs(&self, provider_id: &str) -> usize {
        self.udf_providers
            .values()
            .filter(|p| *p == provider_id)
            .count()
    }

    /// Total number of registered UDFs.
    pub fn udf_count(&self) -> usize {
        self.udfs.len()
    }

    /// Total number of registered providers.
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }
}

/// Summary information about a provider.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub udf_count: usize,
}

/// Summary information about a UDF.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UdfInfo {
    pub full_id: String,
    pub provider_id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub version: String,
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test helper - minimal provider for testing
    struct TestProvider;

    impl UdfProvider for TestProvider {
        fn id(&self) -> &str {
            "test"
        }
        fn name(&self) -> &str {
            "Test Provider"
        }
        fn version(&self) -> &str {
            "0.1.0"
        }
        fn description(&self) -> &str {
            "Test provider for unit tests"
        }
        fn load_udfs(&self) -> Vec<Arc<dyn Udf>> {
            Vec::new()
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = UdfRegistry::new();
        assert_eq!(registry.provider_count(), 0);
        assert_eq!(registry.udf_count(), 0);
    }

    #[test]
    fn test_register_provider() {
        let mut registry = UdfRegistry::new();
        let provider = Arc::new(TestProvider);

        registry.register_provider(provider).unwrap();

        assert_eq!(registry.provider_count(), 1);
        assert!(registry.get_provider("test").is_some());
    }

    #[test]
    fn test_duplicate_provider_error() {
        let mut registry = UdfRegistry::new();
        let provider1 = Arc::new(TestProvider);
        let provider2 = Arc::new(TestProvider);

        registry.register_provider(provider1).unwrap();
        let result = registry.register_provider(provider2);

        assert!(result.is_err());
    }
}
