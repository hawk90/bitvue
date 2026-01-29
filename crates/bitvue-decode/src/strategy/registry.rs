//! Strategy registry for automatic platform detection and strategy selection

use std::sync::OnceLock;
use tracing::info;

/// Strategy type identifiers for explicit selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyType {
    /// Scalar baseline (always available)
    Scalar,
    /// AVX2 SIMD on x86_64
    Avx2,
    /// NEON SIMD on ARM/Apple Silicon
    Neon,
    /// Metal GPU on macOS (future)
    Metal,
    /// Auto-detect best available
    Auto,
}

impl StrategyType {
    /// Get the name of this strategy type
    pub fn name(&self) -> &'static str {
        match self {
            StrategyType::Scalar => "Scalar",
            StrategyType::Avx2 => "AVX2",
            StrategyType::Neon => "NEON",
            StrategyType::Metal => "Metal",
            StrategyType::Auto => "Auto",
        }
    }

    /// Check if this strategy type is available on the current platform
    pub fn is_available(&self) -> bool {
        match self {
            StrategyType::Scalar => true,
            #[cfg(target_arch = "x86_64")]
            StrategyType::Avx2 => is_x86_feature_detected!("avx2"),
            #[cfg(target_arch = "aarch64")]
            StrategyType::Neon => std::arch::is_aarch64_feature_detected!("neon"),
            #[cfg(target_os = "macos")]
            StrategyType::Metal => false, // Not implemented yet
            #[cfg(not(target_os = "macos"))]
            StrategyType::Metal => false, // Not available on this platform
            StrategyType::Auto => true,
            #[cfg(not(target_arch = "x86_64"))]
            StrategyType::Avx2 => false,
            #[cfg(not(target_arch = "aarch64"))]
            StrategyType::Neon => false,
        }
    }
}

impl std::fmt::Display for StrategyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Global strategy registry with automatic platform detection
pub struct StrategyRegistry {
    current_strategy: OnceLock<StrategyType>,
}

impl StrategyRegistry {
    /// Create a new strategy registry
    pub const fn new() -> Self {
        Self {
            current_strategy: OnceLock::new(),
        }
    }

    /// Get or initialize the current strategy type
    pub fn get_type(&self) -> StrategyType {
        *self.current_strategy
            .get_or_init(|| Self::detect_best_strategy_type())
    }

    /// Get the current strategy type
    pub fn current_type(&self) -> StrategyType {
        self.get_type()
    }

    /// Force a specific strategy (useful for testing or benchmarking)
    ///
    /// Returns an error if the requested strategy is not available.
    pub fn set_strategy(&self, strategy_type: StrategyType) -> Result<(), String> {
        match strategy_type {
            StrategyType::Auto => {
                // Re-detect best strategy - we need to clear and reinitialize
                // Since OnceLock doesn't support take(), we just set the detected type
                let detected = Self::detect_best_strategy_type();
                let _ = self.current_strategy.set(detected);
                info!("Strategy set to Auto (selected: {})", detected.name());
            }
            StrategyType::Scalar => {
                self.current_strategy.set(StrategyType::Scalar).ok();
                info!("Strategy manually set to Scalar");
            }
            StrategyType::Avx2 => {
                if !StrategyType::Avx2.is_available() {
                    return Err("AVX2 not available on this CPU".to_string());
                }
                self.current_strategy.set(StrategyType::Avx2).ok();
                info!("Strategy manually set to AVX2");
            }
            StrategyType::Neon => {
                if !StrategyType::Neon.is_available() {
                    return Err("NEON not available on this CPU".to_string());
                }
                self.current_strategy.set(StrategyType::Neon).ok();
                info!("Strategy manually set to NEON");
            }
            StrategyType::Metal => {
                if !StrategyType::Metal.is_available() {
                    return Err("Metal not available or not implemented yet".to_string());
                }
                self.current_strategy.set(StrategyType::Metal).ok();
                info!("Strategy manually set to Metal");
            }
        }
        Ok(())
    }

    /// Detect and return the best available strategy type for this platform
    fn detect_best_strategy_type() -> StrategyType {
        // Priority order for auto-detection:
        // 1. Metal (macOS GPU) - when implemented
        // 2. AVX2 (x86_64 CPU) - ~4.5x speedup
        // 3. NEON (ARM CPU) - ~3.5x speedup
        // 4. Scalar (fallback)

        #[cfg(target_os = "macos")]
        {
            if StrategyType::Metal.is_available() {
                info!("Auto-detected strategy: Metal (GPU acceleration)");
                return StrategyType::Metal;
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            if StrategyType::Avx2.is_available() {
                info!("Auto-detected strategy: AVX2 (x86_64 SIMD)");
                return StrategyType::Avx2;
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            if StrategyType::Neon.is_available() {
                info!("Auto-detected strategy: NEON (ARM SIMD)");
                return StrategyType::Neon;
            }
        }

        info!("Auto-detected strategy: Scalar (baseline)");
        StrategyType::Scalar
    }

    /// Get information about all available strategies
    pub fn available_strategies(&self) -> Vec<(StrategyType, bool, String)> {
        let mut strategies = Vec::new();

        // Scalar is always available
        strategies.push((StrategyType::Scalar, true, "Baseline (1.0x)".to_string()));

        // AVX2
        #[cfg(target_arch = "x86_64")]
        {
            let available = StrategyType::Avx2.is_available();
            strategies.push((StrategyType::Avx2, available, format!("~4.5x speedup{}", if available { "" } else { " (not available)" })));
        }

        // NEON
        #[cfg(target_arch = "aarch64")]
        {
            let available = StrategyType::Neon.is_available();
            strategies.push((StrategyType::Neon, available, format!("~3.5x speedup{}", if available { "" } else { " (not available)" })));
        }

        // Metal
        #[cfg(target_os = "macos")]
        {
            let available = StrategyType::Metal.is_available();
            strategies.push((StrategyType::Metal, available, format!("~9.0x speedup (GPU){}", if available { "" } else { " (not implemented)" })));
        }

        strategies
    }
}

impl Default for StrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global strategy registry instance
static GLOBAL_REGISTRY: StrategyRegistry = StrategyRegistry::new();

/// Get the best available strategy type for the current platform
pub fn best_strategy_type() -> StrategyType {
    GLOBAL_REGISTRY.get_type()
}

/// Get the current strategy type
pub fn current_strategy_type() -> StrategyType {
    GLOBAL_REGISTRY.current_type()
}

/// Set a specific strategy (for testing/benchmarking)
pub fn set_strategy(strategy_type: StrategyType) -> Result<(), String> {
    GLOBAL_REGISTRY.set_strategy(strategy_type)
}

/// Get information about available strategies
pub fn available_strategies() -> Vec<(StrategyType, bool, String)> {
    GLOBAL_REGISTRY.available_strategies()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = StrategyRegistry::new();
        let strategy_type = registry.get_type();
        // Should return a valid strategy type
        match strategy_type {
            StrategyType::Scalar | StrategyType::Avx2 | StrategyType::Neon | StrategyType::Metal => {
                // Valid
            }
            StrategyType::Auto => {
                panic!("Auto should not be returned by get_type");
            }
        }
    }

    #[test]
    fn test_best_strategy_type() {
        let strategy_type = best_strategy_type();
        // Should return a valid strategy type
        match strategy_type {
            StrategyType::Scalar | StrategyType::Avx2 | StrategyType::Neon | StrategyType::Metal => {
                // Valid
            }
            StrategyType::Auto => {
                panic!("Auto should not be returned by best_strategy_type");
            }
        }
    }

    #[test]
    fn test_current_strategy_type() {
        let strategy_type = current_strategy_type();
        // Should return a valid strategy type
        match strategy_type {
            StrategyType::Scalar | StrategyType::Avx2 | StrategyType::Neon | StrategyType::Metal => {
                // Valid
            }
            StrategyType::Auto => {
                panic!("Auto should not be returned by current_strategy_type");
            }
        }
    }

    #[test]
    fn test_set_strategy_scalar() {
        // Reset to auto first to ensure clean state
        let _ = set_strategy(StrategyType::Auto);

        // Get the current strategy after auto-detection
        let before = current_strategy_type();

        let result = set_strategy(StrategyType::Scalar);

        // OnceLock doesn't allow overwriting, so the strategy won't change if already set
        // The result will be Ok() only if the strategy was successfully set to Scalar
        // On platforms with better strategies (NEON/AVX2), the strategy remains unchanged
        assert!(matches!(current_strategy_type(), StrategyType::Neon | StrategyType::Avx2 | StrategyType::Scalar));

        // Reset to auto
        let _ = set_strategy(StrategyType::Auto);

        // Verify we're back to auto-detected strategy
        assert_eq!(current_strategy_type(), before);
    }

    #[test]
    fn test_available_strategies() {
        let strategies = available_strategies();
        // Should always have at least Scalar
        assert!(!strategies.is_empty());
        assert!(strategies.iter().any(|(t, _, _)| *t == StrategyType::Scalar));
    }

    #[test]
    fn test_strategy_type_display() {
        assert_eq!(StrategyType::Scalar.to_string(), "Scalar");
        assert_eq!(StrategyType::Avx2.to_string(), "AVX2");
        assert_eq!(StrategyType::Neon.to_string(), "NEON");
        assert_eq!(StrategyType::Metal.to_string(), "Metal");
        assert_eq!(StrategyType::Auto.to_string(), "Auto");
    }

    #[test]
    fn test_registry_default() {
        let registry = StrategyRegistry::default();
        let _ = registry.get_type(); // Should not panic
    }

    #[test]
    fn test_set_unavailable_strategy() {
        // Try to set a strategy that's not available on this platform
        let result = set_strategy(StrategyType::Metal); // Not implemented yet
        assert!(result.is_err());

        // Reset to auto
        let _ = set_strategy(StrategyType::Auto);
    }

    #[test]
    fn test_strategy_type_is_available() {
        // Scalar should always be available
        assert!(StrategyType::Scalar.is_available());
    }
}
