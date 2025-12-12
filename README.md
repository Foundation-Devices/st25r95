# st25r95

A Rust embedded driver for the ST25R95 NFC transceiver chip, providing a safe and ergonomic interface for NFC communication protocols.

[![crates.io](https://img.shields.io/crates/v/st25r95.svg)](https://crates.io/crates/st25r95)
[![docs.rs](https://docs.rs/st25r95/badge.svg)](https://docs.rs/st25r95)
[![Rust](https://img.shields.io/badge/rust-2025--06--24-orange.svg)](https://www.rust-lang.org)

## Features

- **Memory Safe**: Written in Rust with comprehensive error handling
- **No Std Compatible**: Designed for embedded systems with `#![no_std]`
- **Type State Pattern**: Compile-time guarantees for correct usage
- **Protocol Support**: ISO14443A/B, ISO15693, and FeliCa protocols
- **Reader & Card Emulation**: Full support for both operating modes
- **Async Ready**: Compatible with embedded-hal-async

## Supported Protocols

- **ISO14443A**: MIFARE Classic, MIFARE Ultralight, NTAG, etc.
- **ISO14443B**: Various Type B cards and tags
- **ISO15693**: Vicinity cards (VCD/ICC)
- **FeliCa**: Sony's contactless smart card system

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
st25r95 = "0.1.0"
```

### Basic Example

```rust
use st25r95::{St25r95, St25r95Spi, St25r95Gpio};

// Implement the SPI interface
struct NfcSpi;
impl St25r95Spi for NfcSpi {
    ...
}
let spi = NfcSpi::default();

// Implement the GPIOs interface
struct NfcGpio;
impl St25r95Gpio for NfcGpio {
    ...
}
let gpio = NfcGpio::default();

// Create driver instance
let mut nfc = St25r95::new(spi, gpio)?;

// Perform Calibration
let _ = nfc.calibrate_tag_detector()?;
```

## Architecture

The driver is organized into several layers:

- **Core**: Low-level SPI communication and register access
- **Protocols**: High-level protocol implementations (ISO14443A/B, ISO15693, FeliCa)
- **Commands**: Command building and response parsing
- **Registers**: Memory-mapped register interface
- **GPIO**: GPIO pin management

### Type State Pattern

The driver uses Rust's type system to ensure correct usage:

```rust
// Driver starts in FieldOff state
let mut nfc = St25r95::new(spi, gpio)?;

// After init(), driver is ready for operations
let ready_nfc = nfc.init()?;

// Protocol-specific operations are available
let iso14443a = ready_nfc.protocol_iso14443a();
```

## Error Handling

All operations return `Result<T, Error>` with comprehensive error types:

```rust
use st25r95::Error;

match nfc.send_command(command) {
    Ok(response) => println!("Success: {:?}", response),
    Err(Error::SpiError(e)) => eprintln!("SPI communication failed"),
    Err(Error::CrcError) => eprintln!("CRC check failed"),
    Err(e) => eprintln!("Other error: {:?}", e),
}
```

## Configuration

The ST25R95 can be configured through register settings:

```rust
use st25r95::register::{AccA, ArcB};

// Configure antenna A settings
nfc.register_modify(AccA::new().with_amplitude(0x7F))?;

// Configure antenna B settings
nfc.register_modify(ArcB::new().with_resonance_frequency(0x4B))?;
```

## Testing

Run the test suite:

```bash
cargo test
```

## Minimum Supported Rust Version

This crate requires Rust 1.86.0 or later.

## License

This project is licensed under GPL-v3-or-newer ([LICENSE-GPL](LICENSE)).

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## Documentation

Full API documentation is available on [docs.rs](https://docs.rs/st25r95).

## Resources

- [ST25R95 Datasheet](https://www.st.com/resource/en/datasheet/st25r95.pdf)
- [ST25R95 Application Notes](https://www.st.com/en/nfc/st25r95.html)
