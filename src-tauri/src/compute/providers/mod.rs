//! Built-in UDF providers.
//!
//! This module contains the default providers shipped with DataForge Compute.

pub mod core;
pub mod petrophysics;

pub use core::CoreProvider;
pub use petrophysics::PetrophysicsProvider;

use crate::compute::registry::UdfRegistry;
use std::sync::Arc;

/// Register all built-in providers with the registry.
pub fn register_builtin_providers(registry: &mut UdfRegistry) -> Result<(), crate::compute::UdfError> {
    // Register the core processing provider
    registry.register_provider(Arc::new(CoreProvider::new()))?;

    // Register the petrophysics provider
    registry.register_provider(Arc::new(PetrophysicsProvider::new()))?;

    Ok(())
}
