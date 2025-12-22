//! Built-in UDF providers.
//!
//! This module contains the default providers shipped with DataForge Compute.

pub mod petrophysics;

pub use petrophysics::PetrophysicsProvider;

use crate::compute::registry::UdfRegistry;
use crate::compute::UdfProvider;
use std::sync::Arc;

/// Register all built-in providers with the registry.
pub fn register_builtin_providers(registry: &mut UdfRegistry) -> Result<(), crate::compute::UdfError> {
    // Register the petrophysics provider
    registry.register_provider(Arc::new(PetrophysicsProvider::new()))?;

    // Future providers would be registered here:
    // registry.register_provider(Arc::new(QualityControlProvider::new()))?;
    // registry.register_provider(Arc::new(TransformsProvider::new()))?;

    Ok(())
}
