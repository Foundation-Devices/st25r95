# st25r95

A Rust embedded driver for the ST25R95 NFC transceiver chip, providing a safe and ergonomic interface for NFC communication protocols.

[![crates.io](https://img.shields.io/crates/v/st25r95.svg)](https://crates.io/crates/st25r95)
[![docs.rs](https://docs.rs/st25r95/badge.svg)](https://docs.rs/st25r95)
[![Rust](https://img.shields.io/badge/rust-1.87.0-orange.svg)](https://www.rust-lang.org)

## Features

- **Memory Safe**: Written in Rust with comprehensive error handling
- **No Std Compatible**: Designed for embedded systems with `#![no_std]`
- **Type State Pattern**: Compile-time guarantees for correct usage
- **Protocol Support**: ISO14443A/B, ISO15693, and FeliCa protocols
- **Reader & Card Emulation**: Full support for both operating modes
- **Blocking API**: Synchronous SPI and GPIO traits you implement for your platform

## Supported Protocols

- **ISO14443A**: MIFARE Classic, MIFARE Ultralight, NTAG, etc.
- **ISO14443B**: Various Type B cards and tags
- **ISO15693**: Vicinity cards (VCD/ICC)
- **FeliCa**: Sony's contactless smart card system

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
st25r95 = "0.6"
```

### Basic Example

The driver talks to the chip through two traits you implement for your
platform: [`St25r95Spi`](https://docs.rs/st25r95/latest/st25r95/trait.St25r95Spi.html)
for the SPI transport and
[`St25r95Gpio`](https://docs.rs/st25r95/latest/st25r95/trait.St25r95Gpio.html)
for IRQ_IN and IRQ_OUT. The `mock` module implements both without hardware, so
the example below - like every example in this README - is compiled and run by
the test suite.

```rust
use st25r95::{
    mock::{MockGpio, MockSpi},
    St25r95,
};

// Create the driver: `MockSpi` and `MockGpio` stand in for your own
// `St25r95Spi` and `St25r95Gpio` implementations.
let nfc = St25r95::new(MockSpi::default(), MockGpio)?;

// Selecting a protocol turns the RF field on and moves the driver into
// ISO14443A reader mode
let mut reader = nfc.protocol_select_iso14443a(Default::default())?;

// Send REQA and read the tag response
let response = reader.send_receive(&[0x26])?;
response.ensure_ok()?;

// Turn the field off when done
let nfc = reader.field_off()?;
# let _ = nfc;
# Ok::<(), st25r95::Error>(())
```

## Architecture

The driver is organized into several layers:

- **Core**: Low-level SPI communication and register access
- **Protocols**: High-level protocol implementations (ISO14443A/B, ISO15693, FeliCa)
- **Commands**: Command building and response parsing
- **Registers**: Register interface (`arc_b`, `acc_a`, `timer_window`, ...)
- **GPIO**: IRQ_IN and IRQ_OUT management

### Type State Pattern

The driver uses Rust's type system to ensure correct usage: the field state,
role and protocol are type parameters, so a method that only makes sense in
card emulation cannot be called on a reader.

```rust
use st25r95::{
    mock::{MockGpio, MockSpi},
    St25r95,
};

// Driver starts in the FieldOff, NoRole, NoProtocol state
let nfc = St25r95::new(MockSpi::default(), MockGpio)?;

// Card emulation exposes Listen mode, which a reader does not have
let mut card = nfc.protocol_select_ce_iso14443a(Default::default())?;
card.listen()?;
# Ok::<(), st25r95::Error>(())
```

## Timeouts and Recovery

Every command has a host-side deadline, derived from the frame timeout
configured for the selected protocol (`FDT`, `FWT` or `RWT`) and never shorter
than 100 ms. When the deadline elapses the chip is still running the
operation, so the driver reports `Error::ResponseInProgress` and refuses to
send another command until the outstanding answer is either awaited or given
up.

```rust
use st25r95::{
    mock::{MockGpio, MockSpi},
    Error,
    ReadResponse,
    St25r95,
};

let nfc = St25r95::new(MockSpi::default(), MockGpio)?;
let mut reader = nfc.protocol_select_iso14443a(Default::default())?;

let result = reader.send_receive(&[0x26]);

let response: ReadResponse = match result {
    Ok(response) => response,
    Err(Error::ResponseInProgress) => {
        // The chip has not answered yet: keep waiting for that same
        // operation, or call `discard_pending_response` to give it up.
        reader.poll_pending_response(1_000)?
    }
    Err(error) => return Err(error),
};
# let _ = response;
# Ok::<(), Error>(())
```

## Error Handling

All operations return `Result<T, Error>`. Hardware status bytes reported by
the chip are wrapped in `Error::Hw`:

```rust
use st25r95::{Error, ReadResponse, Result, St25r95Error};

fn handle(response: ReadResponse) -> Result<()> {
    match response.ensure_ok() {
        Ok(()) => Ok(()),
        Err(Error::Hw(St25r95Error::FrameTimeoutOrNoTag)) => Ok(()), // no tag
        Err(Error::Hw(St25r95Error::CrcError)) => Ok(()),            // retry
        Err(error) => Err(error),
    }
}
# let response = ReadResponse::try_from([0x87, 0x00].as_slice())?;
# assert_eq!(handle(response), Ok(()));
# Ok::<(), Error>(())
```

## Configuration

Analog settings are configured through the chip's registers. The valid values
depend on the selected protocol, so the constructors are methods on the
driver:

```rust
use st25r95::{
    mock::{MockGpio, MockSpi},
    ModulationIndex,
    ReceiverGain,
    St25r95,
};

let nfc = St25r95::new(MockSpi::default(), MockGpio)?;
let mut reader = nfc.protocol_select_iso14443a(Default::default())?;

// Antenna configuration (ARC_B): modulation index and receiver gain
let arc_b = reader.new_arc_b(ModulationIndex::Percent95, ReceiverGain::Db8)?;
reader.write_arc_b(arc_b)?;

// Timing window recommended for the selected protocol
let timer_window = reader.recommended_timer_window();
reader.write_timer_windows(timer_window)?;
# Ok::<(), st25r95::Error>(())
```

## Testing

Run the test suite, which includes every example in this README and in the API
documentation:

```bash
cargo test
```

## Minimum Supported Rust Version

This crate requires Rust 1.87.0 or later, matching `rust-version` in
`Cargo.toml`. Embedded builds are checked against the `thumbv7em-none-eabi`
target.

## License

This project is licensed under GPL-v3-or-newer ([LICENSE](LICENSE)).

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
