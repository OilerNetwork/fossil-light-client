# Rust Project Context Template

## Project Overview
- **Project Type**: [web service / CLI tool / library / desktop app / embedded / other]
- **Target Audience**: [internal team / public API / end users / developers]
- **Performance Requirements**: [low latency / high throughput / memory constrained / standard]
- **Deployment Environment**: [cloud / on-premises / embedded / WASM / other]

Here are the most important Rust patterns for writing clean, maintainable, and reusable code:

## Ownership and Borrowing Patterns

**RAII (Resource Acquisition Is Initialization)** - Let Rust's ownership system handle resource cleanup automatically. Prefer owned types when possible, and use references (`&T`, `&mut T`) for temporary access.

**Builder Pattern** - Create complex objects step by step, especially useful for structs with many optional fields:

```rust
pub struct Config {
    host: String,
    port: u16,
    timeout: Option<Duration>,
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }
}

pub struct ConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
    timeout: Option<Duration>,
}

impl ConfigBuilder {
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    pub fn build(self) -> Result<Config, &'static str> {
        // validation and construction logic
    }
}
```

## Error Handling Patterns

**Result and Option chaining** - Use `?` operator, `map`, `and_then`, and `unwrap_or_else` to handle errors gracefully:

```rust
fn process_data(input: &str) -> Result<ProcessedData, MyError> {
    let parsed = parse_input(input)?;
    let validated = validate_data(parsed)?;
    transform_data(validated)
}
```

**Custom Error types** - Define domain-specific errors using enums and implement `std::error::Error`:

```rust
#[derive(Debug)]
pub enum DatabaseError {
    ConnectionFailed(String),
    QueryFailed(String),
    NotFound,
}
```

## Type Safety Patterns

**Newtype Pattern** - Wrap primitive types to add semantic meaning and prevent mix-ups:

```rust
pub struct UserId(u64);
pub struct OrderId(u64);

// Now you can't accidentally pass a UserId where OrderId is expected
```

**Type State Pattern** - Use the type system to enforce state transitions:

```rust
pub struct Connection<State> {
    inner: TcpStream,
    state: PhantomData<State>,
}

pub struct Connected;
pub struct Authenticated;

impl Connection<Connected> {
    pub fn authenticate(self, credentials: &str) -> Result<Connection<Authenticated>, Error> {
        // authentication logic
    }
}

impl Connection<Authenticated> {
    pub fn send_data(&self, data: &[u8]) -> Result<(), Error> {
        // can only send data when authenticated
    }
}
```

## Trait-Based Patterns

**Trait Objects for Dynamic Dispatch** - Use `dyn Trait` when you need runtime polymorphism:

```rust
trait Processor {
    fn process(&self, data: &str) -> String;
}

fn handle_processors(processors: Vec<Box<dyn Processor>>) {
    // work with different processor implementations
}
```

**Associated Types vs Generics** - Use associated types when there's one logical implementation per type, generics when there could be multiple:

```rust
trait Iterator {
    type Item;  // associated type - one Item type per Iterator
    fn next(&mut self) -> Option<Self::Item>;
}

trait From<T> {  // generic - can convert from many types
    fn from(value: T) -> Self;
}
```

## Composition Patterns

**Prefer Composition over Inheritance** - Use struct embedding and trait implementations:

```rust
pub struct Database {
    connection: Connection,
    cache: Cache,
}

impl Database {
    pub fn new(conn: Connection, cache: Cache) -> Self {
        Self { connection: conn, cache }
    }
}
```

**Extension Traits** - Add functionality to existing types without modifying them:

```rust
trait StringExtensions {
    fn truncate_words(&self, max_words: usize) -> String;
}

impl StringExtensions for str {
    fn truncate_words(&self, max_words: usize) -> String {
        // implementation
    }
}
```

## Module and Visibility Patterns

**Controlled API Surface** - Keep internal implementation details private, expose only what's necessary:

```rust
pub mod user {
    pub struct User {
        pub id: UserId,
        pub name: String,
        // private fields
        internal_state: InternalState,
    }

    impl User {
        pub fn new(name: String) -> Self { /* */ }
        // public methods only
    }

    // private helper types
    struct InternalState { /* */ }
}
```

**Re-exports for Clean APIs** - Use `pub use` to create clean module interfaces:

```rust
// in lib.rs
pub use crate::user::{User, UserId};
pub use crate::order::{Order, OrderStatus};

// Users can now do: use mylib::{User, Order};
```

These patterns work together to create code that's safe, expressive, and easy to maintain. The key is leveraging Rust's type system and ownership model to catch errors at compile time while keeping the code readable and modular.

## Architecture & Standards

### Code Organization Principles
- Follow domain-driven design with clear module boundaries
- Separate concerns: core logic, I/O, presentation, infrastructure
- Use dependency injection through trait abstractions
- Prefer composition over complex inheritance hierarchies

### Error Handling Strategy
- Use `Result<T, E>` for all fallible operations
- Define domain-specific error types using enums
- Implement `std::error::Error` for all custom errors
- Use `anyhow` for application errors, `thiserror` for library errors
- Never use `unwrap()` or `expect()` in production code without clear justification

### Async/Concurrency Approach
- Runtime: [tokio / async-std / none]
- Use `async/await` for I/O-bound operations
- Prefer `tokio::spawn` for CPU-bound work in async contexts
- Use channels (`mpsc`, `broadcast`, `watch`) for communication between tasks
- Avoid `Arc<Mutex<T>>` in async code; prefer `Arc<Tokio::Mutex<T>>` or channels

### Memory Management Guidelines
- Prefer owned types (`String`, `Vec<T>`) for data that needs to live beyond function scope
- Use borrowing (`&str`, `&[T]`) for temporary access
- Apply the newtype pattern for type safety with primitives
- Use `Cow<T>` when you might need either owned or borrowed data
- Leverage `Box<T>` for heap allocation when stack size is a concern

## Dependencies & Ecosystem

### Standard Dependencies
```toml
[dependencies]
# Error handling
anyhow = "1.0"          # Application-level error handling
thiserror = "1.0"       # Library error types

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Async runtime (if applicable)
tokio = { version = "1.0", features = ["full"] }

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Testing
proptest = "1.0"        # Property-based testing

[dev-dependencies]
tokio-test = "0.4"      # Async testing utilities
mockall = "0.11"        # Mocking framework
```

### Preferred Crates by Domain
- **Web**: axum, tower, hyper
- **CLI**: clap, indicatif, console
- **Database**: sqlx, diesel, sea-orm
- **HTTP Client**: reqwest, surf
- **Configuration**: config, figment
- **DateTime**: chrono, time
- **UUID**: uuid
- **Crypto**: ring, rustls

## Code Style & Formatting

### Formatting Rules
- Use `rustfmt` with default settings
- Line length: 100 characters maximum
- Use trailing commas in multi-line structures
- Prefer explicit return statements for clarity in complex functions

### Naming Conventions
- Types: `PascalCase` (e.g., `UserRepository`, `DatabaseError`)
- Functions/variables: `snake_case` (e.g., `process_request`, `user_id`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `MAX_RETRY_ATTEMPTS`)
- Modules: `snake_case` (e.g., `user_service`, `db_models`)

### Documentation Standards
- Document all public APIs with `///` comments
- Include usage examples in documentation
- Use `#[doc(hidden)]` for internal public items
- Write module-level documentation explaining purpose and usage patterns

## Testing Strategy

### Test Organization
```
src/
├── lib.rs
├── user/
│   ├── mod.rs
│   ├── repository.rs
│   └── service.rs
tests/
├── integration/
│   ├── user_tests.rs
│   └── common/
│       └── mod.rs
└── fixtures/
    └── test_data.json
```

### Testing Guidelines
- Unit tests: Test individual functions and methods in isolation
- Integration tests: Test component interactions and API contracts
- Property-based tests: Use `proptest` for testing invariants
- Mock external dependencies using `mockall` or manual trait implementations
- Use `#[cfg(test)]` for test-only code and utilities

### Test Naming
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_create_user_when_valid_input_provided() {
        // Arrange
        let input = CreateUserRequest { /* ... */ };

        // Act
        let result = create_user(input);

        // Assert
        assert!(result.is_ok());
    }
}
```

## Performance & Security

### Performance Considerations
- Use `cargo flamegraph` for profiling
- Prefer zero-copy operations where possible
- Use `String::with_capacity()` when final size is known
- Consider `smallvec` for small collections
- Profile before optimizing; measure actual performance impact

### Security Guidelines
- Validate all inputs at boundaries
- Use type-safe wrappers for sensitive data (passwords, tokens)
- Prefer `rustls` over OpenSSL for TLS
- Use `secrecy` crate for handling secrets
- Regular `cargo audit` runs for dependency vulnerabilities

## CI/CD & Tooling

### Essential Tools
```toml
# In your workspace or project root
[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
cargo = "warn"
```

### Pre-commit Hooks
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`
- `cargo audit`

### Continuous Integration
- Test on multiple Rust versions (stable, beta, nightly)
- Run tests with different feature combinations
- Check documentation builds (`cargo doc --no-deps`)
- Verify no unused dependencies (`cargo machete`)

## Project-Specific Customizations

### Domain-Specific Patterns
[Add your specific domain patterns here, e.g.:]
- **Web Services**: Request/Response DTOs, middleware patterns, database connection pooling
- **CLI Tools**: Command parsing, progress indication, configuration management
- **Libraries**: Public API design, feature flags, backward compatibility

### Custom Lint Rules
```rust
// Add project-specific lint configurations
#![warn(clippy::todo)]
#![warn(clippy::unimplemented)]
#![deny(missing_debug_implementations)]
```

### Feature Flags
```toml
[features]
default = ["std"]
std = []
serde = ["dep:serde"]
async = ["dep:tokio"]
```

## Communication & Collaboration

### Code Review Guidelines
- Focus on correctness, clarity, and maintainability
- Check error handling paths
- Verify test coverage for new functionality
- Ensure documentation is updated
- Review for potential security implications

### Git Workflow
- Use conventional commits (feat:, fix:, docs:, refactor:, test:)
- Keep commits focused and atomic
- Write descriptive commit messages
- Squash feature branches before merging
