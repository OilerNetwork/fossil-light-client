/// Trait for proof generation operations
pub trait ProofGeneratorTrait: Send + Sync {
    // This trait can be extended with specific proof generation operations as needed
    // For now, we define it as a marker trait that the concrete type implements
}

/// Implementation of `ProofGeneratorTrait` for the concrete `ProofGenerator`
impl<T: Send + Sync> ProofGeneratorTrait for crate::core::ProofGenerator<T> {
    // Concrete implementation is handled by the ProofGenerator type directly
}
