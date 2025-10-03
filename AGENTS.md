# Agent Guidelines for st25r95

## Build/Lint/Test Commands

### Building
- `cargo build --verbose` - Build the project
- `cargo build --release` - Build optimized release version

### Testing
- `cargo test --verbose` - Run all tests
- `cargo test <test_name>` - Run specific test
- `cargo test -- --exact <test_name>` - Run single test exactly matching name

### Linting & Formatting
- `cargo clippy` - Run clippy linter (requires nightly toolchain)
- `cargo fmt` - Format code with rustfmt
- `cargo sort` - Sort imports (cargo-sort tool required)

## Code Style Guidelines

### Rust Version & Toolchain
- Uses nightly Rust 2025-06-24
- Target: armv7a-none-eabi for embedded builds
- Components: rustfmt, clippy, rustc

### Formatting (rustfmt.toml)
- `imports_layout = "HorizontalVertical"`
- `imports_granularity = "One"`
- `group_imports = "StdExternalCrate"`
- `use_try_shorthand = true`
- `wrap_comments = true`
- `comment_width = 90`
- `format_code_in_doc_comments = true`
- `format_strings = true`
- `normalize_doc_attributes = true`
- `remove_nested_parens = true`
- `use_field_init_shorthand = true`

### Naming Conventions
- **Types/Enums/Structs**: PascalCase (e.g., `St25r95`, `Command`, `Error`)
- **Functions/Methods/Variables**: snake_case (e.g., `send_receive`, `protocol_select`)
- **Constants**: SCREAMING_SNAKE_CASE
- **Modules**: snake_case (e.g., `iso14443a`, `card_emulation`)

### Import Style
- Use grouped imports: `use {crate::module::Type, other::Type};`
- Sort imports with `cargo sort`
- Group by: std, external crates, then internal modules
- Prefer qualified imports for clarity

### Error Handling
- Custom `Result<T> = core::result::Result<T, Error>` type
- Use `derive_more::From` for automatic error conversions
- Return `Result<()>` for operations that can fail
- Use `?` operator for error propagation
- Prefer specific error variants over generic ones

### Code Patterns
- **Type State Pattern**: Extensively used for compile-time guarantees
- **PhantomData**: Used for type state implementation
- **No std**: `#[cfg_attr(not(test), no_std)]` for embedded compatibility
- **Documentation**: Use `///` for public API documentation
- **Derives**: Common: `Debug, Clone, Copy, PartialEq, Default`

### Architecture
- SPI-based communication with ST25R95 NFC transceiver
- Protocol abstraction layers (ISO14443A/B, ISO15693, FeliCa)
- Reader and Card Emulation modes
- Register-based configuration system

### Testing
- Unit tests in same files as implementation
- Integration tests for protocol interactions
- Mock SPI interface for testing
- Test data validation and error conditions