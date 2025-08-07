use starknet_handler::account::StarknetAccount;

/// Trait for Starknet account operations
pub trait StarknetAccountTrait: Send + Sync {
    // This trait can be extended with specific account operations as needed
    // For now, we define it as a marker trait that the concrete type implements
}

/// Implementation of  `StarknetAccountTrait` for the concrete `StarknetAccount`
impl StarknetAccountTrait for StarknetAccount {
    // Concrete implementation is handled by the StarknetAccount type directly
}
