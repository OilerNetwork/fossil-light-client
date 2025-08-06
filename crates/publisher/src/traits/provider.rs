use starknet_handler::provider::StarknetProvider;

/// Trait for Starknet provider operations
pub trait StarknetProviderTrait: Send + Sync {
    // This trait serves as a marker for dependency injection
    // Concrete implementation is handled by the StarknetProvider type directly
}

/// Implementation of StarknetProviderTrait for the concrete StarknetProvider
impl StarknetProviderTrait for StarknetProvider {
    // Concrete implementation is handled by the StarknetProvider type directly
}
