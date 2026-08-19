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
- `cargo clippy -- -D warnings` - Run clippy linter (stable toolchain)
- `cargo fmt` - Format code with rustfmt (devenv pins the nightly-2025-06-24
  rustfmt the CI `fmt` job uses, because rustfmt.toml enables unstable options)
- `cargo sort` - Sort imports (cargo-sort tool required)

## Code Style Guidelines

### Rust Version & Toolchain
- Builds, tests and clippy run on stable 1.87.0 (the manifest MSRV)
- `rustfmt` is nightly-2025-06-24, pinned in devenv.nix to match the CI `fmt` job
- Target: thumbv7em-none-eabi for embedded builds
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
- Every README and rustdoc example is a doctest and must compile and pass
- `st25r95::mock` provides the SPI and GPIO implementations examples use
- Test data validation and error conditions