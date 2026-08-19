// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

#![cfg_attr(not(test), no_std)]

//! # ST25R95 NFC Transceiver Driver
//!
//! This crate provides a high-level, type-safe driver for the ST25R95 NFC transceiver
//! chip from STMicroelectronics. The ST25R95 is a multi-protocol NFC transceiver
//! supporting reader and card emulation modes for various NFC protocols.
//!
//! ## Features
//!
//! - **Multi-protocol support**: ISO/IEC 14443A/B, ISO/IEC 15693, and FeliCa protocols
//! - **Reader and Card Emulation modes**: Operate as either an NFC reader or emulate a
//!   card
//! - **Type-state pattern**: Compile-time guarantees for correct usage sequences
//! - **Hardware abstraction**: Trait-based SPI and GPIO interfaces for flexibility
//! - **Embedded-friendly**: `no_std` compatible with minimal dependencies
//! - **Register-level control**: Fine-tuned configuration of all ST25R95 parameters
//!
//! ## Type State Pattern
//!
//! This driver uses a sophisticated type-state pattern to prevent incorrect usage at
//! compile time. The main `St25r95` struct is parameterized by five type state markers:
//!
//! - **Field State** (`F`): `FieldOn` or `FieldOff` - controls whether the RF field is
//!   active
//! - **Role State** (`R`): `Reader`, `CardEmulation`, or `NoRole` - defines the operating
//!   mode
//! - **Protocol State** (`P`): `Iso15693`, `Iso14443A`, `Iso14443B`, `FeliCa`, or
//!   `NoProtocol`
//! - **SPI Interface** (`S`): User-provided implementation of `St25r95Spi`
//! - **GPIO Interface** (`G`): User-provided implementation of `St25r95Gpio`
//!
//! This ensures that operations are only available when the chip is in the correct state.
//! For example, you can only send/receive data when the field is on and a protocol is
//! selected.
//!
//! ## Basic Usage
//!
//! ```rust
//! use st25r95::{
//!     mock::{MockGpio, MockSpi},
//!     St25r95,
//! };
//!
//! // Initialize the driver with your own `St25r95Spi` and `St25r95Gpio`
//! // implementations; the mocks stand in for hardware here.
//! let nfc = St25r95::new(MockSpi::default(), MockGpio)?;
//!
//! // Select ISO14443A reader mode, which turns the RF field on
//! let mut reader = nfc.protocol_select_iso14443a(Default::default())?;
//!
//! // Send a command to a tag and receive the response
//! let response = reader.send_receive(&[0x26])?; // REQA command
//!
//! // Turn off the RF field
//! let _nfc = reader.field_off()?;
//! # Ok::<(), st25r95::Error>(())
//! ```
//!
//! ## Hardware Requirements
//!
//! - ST25R95 NFC transceiver chip
//! - SPI interface for communication (MOSI, MISO, SCLK, CS)
//! - GPIO pin for interrupt handling (IRQ_OUT)
//! - Optional: GPIO pin for wake-up control (IRQ_IN)
//!
//! ## Protocol Support
//!
//! ### Reader Mode
//! - **ISO/IEC 14443A**: MIFARE Classic, MIFARE Ultralight, NTAG, etc.
//! - **ISO/IEC 14443B**: Calypso, etc.
//! - **ISO/IEC 15693**: Vicinity cards, ICODE SLI, Tag-it HF-I, etc.
//! - **FeliCa**: FeliCa cards and tags
//!
//! ### Card Emulation Mode
//! - **ISO/IEC 14443A**: Emulate Type A cards with anti-collision support
//!
//! ## Error Handling
//!
//! The driver provides comprehensive error handling with specific error types for:
//! - Hardware communication failures
//! - Invalid parameter ranges
//! - Protocol-specific errors
//! - Timeout conditions
//!
//! See the `Error` enum for detailed information about possible error conditions.

mod command;
mod control;
mod error;
mod gpio;
pub mod mock;
mod protocol;
mod register;
mod spi;

pub use {
    crate::{
        command::{Command, CtrlResConf, DacData, IdleParams, LFOFreq, WaitForField, WakeUpSource},
        control::{Control, PollFlags},
        protocol::*,
        register::{
            arc_b::{ModulationIndex, ReceiverGain},
            *,
        },
    },
    error::{Error, Result, St25r95Error},
    gpio::St25r95Gpio,
    spi::St25r95Spi,
};
use {
    acc_a::{AccA, DemodulatorSensitivity, LoadModulationIndex},
    arc_b::ArcB,
    auto_detect_filter::AutoDetectFilter,
    core::{fmt::Debug, marker::PhantomData, str::from_utf8},
    iso14443a::{
        card_emulation::{AntiColState, Listen},
        ATQA,
        SAK,
        UID,
    },
    iso15693::reader::Modulation,
    timer_window::TimerWindow,
    wakeup::Wakeup,
};

// === Type State Field ===

/// Marker type indicating the RF field is turned on
///
/// When in this state, the ST25R95 is actively generating an RF field and can
/// communicate with NFC tags or act as a card in card emulation mode.
///
/// This is a required state for:
/// - Sending commands to tags (reader mode)
/// - Receiving tag responses (reader mode)
/// - Listening for reader commands (card emulation mode)
/// - Sending responses to readers (card emulation mode)
#[derive(Debug, Default)]
pub struct FieldOn;

/// Marker type indicating the RF field is turned off
///
/// When in this state, the ST25R95 is not generating an RF field and has minimal
/// power consumption. This is the default state after initialization and the
/// recommended state when the driver is not actively communicating.
///
/// From this state, you must select a protocol to transition to `FieldOn`.
#[derive(Debug, Default)]
pub struct FieldOff;

// === Type State Role ===

/// Marker type indicating reader mode operation
///
/// In reader mode, the ST25R95 acts as an NFC reader/writer, communicating with
/// external NFC tags. The driver generates the RF field and sends commands to
/// tags, then receives and processes their responses.
///
/// Available operations:
/// - `send_receive()`: Send commands and receive responses from tags
/// - Protocol-specific register configuration (ARC_B, Timer Window, etc.)
/// - `field_off()`: Turn off the RF field
#[derive(Debug, Default)]
pub struct Reader;

/// Marker type indicating card emulation mode operation
///
/// In card emulation mode, the ST25R95 emulates an NFC tag/card and responds to
/// commands from external NFC readers. The driver listens for reader commands
/// and sends appropriate responses.
///
/// The boolean parameter indicates whether the driver is currently in
/// listening mode (`true`) or not (`false`).
///
/// Available operations:
/// - `listen()`: Enter listening mode to wait for reader commands
/// - `receive()`: Receive commands from external readers
/// - `send()`: Send responses to external readers
/// - Anti-collision filter management for Type A emulation
#[derive(Debug, Default)]
pub struct CardEmulation(Listen);

/// Marker type indicating no role has been selected
///
/// This is the initial state after driver creation. From this state, you must
/// select either reader mode or card emulation mode to proceed with NFC operations.
#[derive(Debug)]
pub struct NoRole;

// === Type State Protocol ===

/// Marker type for ISO/IEC 15693 protocol (Vicinity cards)
///
/// The `Modulation` parameter specifies the modulation type used for communication:
/// - 10% modulation: Standard, compatible with most 15693 tags
/// - 100% modulation: Higher power, longer range but less compatible
///
/// This protocol supports:
/// - Long-range communication (up to 1.5m)
/// - Single and multiple card anticollision
/// - Read/write operations on 15693 tags
/// - Inventory commands for tag detection
//
// `Default` is required by the reader methods shared with the other
// protocols, such as `send_receive` and `field_off`.
#[derive(Debug, Default)]
pub struct Iso15693(Modulation);

/// Marker type for ISO/IEC 14443 Type A protocol
///
/// This protocol supports:
/// - MIFARE Classic, MIFARE Ultralight, NTAG series tags
/// - Short-range communication (up to 10cm)
/// - Anticollision and cascading for UID detection
/// - Authentication and encrypted communication (MIFARE Classic)
/// - Fast read/write operations
///
/// In reader mode, this provides access to:
/// - Configurable ARC_B register for optimal performance
/// - Timer window configuration for improved demodulation
///
/// In card emulation mode, this provides:
/// - Anti-collision filter for selective emulation
/// - Configurable ACC_A register for load modulation
#[derive(Debug, Default)]
pub struct Iso14443A;

/// Marker type for ISO/IEC 14443 Type B protocol
///
/// This protocol supports:
/// - Calypso cards and other Type B tags
/// - Short-range communication (up to 10cm)
/// - Different anticollision mechanism than Type A
/// - Asynchronous communication
///
/// Reader mode features:
/// - Configurable ARC_B register with different modulation options
/// - Compatible with public transport and access control systems
#[derive(Debug, Default)]
pub struct Iso14443B;

/// Marker type for FeliCa protocol
///
/// This protocol supports:
/// - FeliCa cards and tags (primarily used in Japan)
/// - High-speed communication (212 kbps or 424 kbps)
/// - Advanced security features
/// - Suica, Pasmo, and other Japanese transit cards
///
/// Reader mode features:
/// - Configurable ARC_B register for FeliCa-specific modulation
/// - Auto-detect filter for improved synchronization
/// - Optimized for high-speed polling applications
#[derive(Debug, Default)]
pub struct FeliCa;

/// Marker type indicating no protocol has been selected
///
/// This is the initial state after driver creation. From this state, you must
/// select a specific protocol to proceed with NFC operations. Each protocol
/// enables different capabilities and optimizations specific to that protocol.
#[derive(Debug)]
pub struct NoProtocol;

type ResultSt25r95FieldOff<S, G, R, P> = Result<St25r95<S, G, FieldOff, R, P>>;
type ResultSt25r95ReaderIso15693<S, G> = Result<St25r95<S, G, FieldOn, Reader, Iso15693>>;
type ResultSt25r95ReaderIso14443A<S, G> = Result<St25r95<S, G, FieldOn, Reader, Iso14443A>>;
type ResultSt25r95ReaderIso14443B<S, G> = Result<St25r95<S, G, FieldOn, Reader, Iso14443B>>;
type ResultSt25r95ReaderFelica<S, G> = Result<St25r95<S, G, FieldOn, Reader, FeliCa>>;
type ResultSt25r95CardEmulationIso14443A<S, G> =
    Result<St25r95<S, G, FieldOn, CardEmulation, Iso14443A>>;

/// Maximum buffer size for SPI communication with the ST25R95 chip
pub const MAX_BUFFER_SIZE: usize = 530;

/// Maximum payload of a single host-to-chip command
///
/// Commands are sent to the ST25R95 as `<CMD><Len><Data>` (datasheet DS12807
/// §4.2) where `Len` is a single byte, so no command can carry more than 255
/// payload bytes. Every public method that forwards a caller-provided slice
/// rejects a longer payload with [`Error::InvalidDataLen`] before reaching the
/// [`St25r95Spi`] implementation, so an implementation never has to decide
/// what to do with a length it cannot encode.
pub const MAX_COMMAND_DATA_LEN: usize = u8::MAX as usize;

/// Byte the ST25R95 answers to an `Echo` command
pub const ECHO_BYTE: u8 = 0x55;

/// Maximum length of the raw answer to an `Echo` command
///
/// An ordinary Echo answers with the single byte [`ECHO_BYTE`]. An Echo that
/// leaves Listen mode answers with that byte followed by the two-byte framed
/// response `0x85 0x00`, i.e. [`St25r95Error::UserStop`] with no data
/// (datasheet DS12807).
pub const ECHO_RESPONSE_MAX_LEN: usize = 3;

/// Host-side deadline used when no device-side timeout is configured
///
/// Commands that the chip answers immediately (identification, register
/// access, protocol selection, Echo) never need more than this.
pub const DEFAULT_RESPONSE_TIMEOUT_MS: u32 = 100;

/// Margin added on top of a configured device-side timeout
///
/// Covers the chip's own processing and the SPI turnaround, so the host only
/// gives up once the chip provably has.
pub const RESPONSE_TIMEOUT_MARGIN_MS: u32 = 50;

/// Turn a device-side timeout in microseconds into a host-side deadline in
/// milliseconds.
///
/// The result is never shorter than [`DEFAULT_RESPONSE_TIMEOUT_MS`], and never
/// shorter than the device timeout itself, so a valid long frame wait cannot be
/// cut short by the host.
fn host_timeout_ms(device_timeout_us: Option<f32>) -> u32 {
    let Some(us) = device_timeout_us else {
        return DEFAULT_RESPONSE_TIMEOUT_MS;
    };
    // `f32::ceil` needs `std`; truncating and adding one rounds up instead, and
    // the `as` cast saturates for values beyond `u32`.
    ((us / 1000.0) as u32)
        .saturating_add(1)
        .saturating_add(RESPONSE_TIMEOUT_MARGIN_MS)
        .max(DEFAULT_RESPONSE_TIMEOUT_MS)
}

/// Main driver struct for the ST25R95 NFC transceiver chip
///
/// This struct uses a type-state pattern to ensure correct usage at compile time.
/// The generic parameters represent different states of the chip:
///
/// - **S**: SPI interface implementation (`St25r95Spi` trait)
/// - **G**: GPIO interface implementation (`St25r95Gpio` trait)
/// - **F**: Field state - `FieldOn` or `FieldOff`
/// - **R**: Role state - `Reader`, `CardEmulation`, or `NoRole`
/// - **P**: Protocol state - `Iso15693`, `Iso14443A`, `Iso14443B`, `FeliCa`, or
///   `NoProtocol`
///
/// ## State Transitions
///
/// The driver enforces a specific sequence of operations:
///
/// 1. **Initial state**: `FieldOff`, `NoRole`, `NoProtocol`
/// 2. **Select protocol**: Transitions to `FieldOn`, `Reader`/`CardEmulation`, specific
///    protocol
/// 3. **Use protocol**: Send/receive data in the selected mode
/// 4. **Turn off field**: Return to `FieldOff` state
///
/// ## Examples
///
/// ```rust
/// # use st25r95::{
/// #     mock::{MockGpio, MockSpi},
/// #     St25r95,
/// # };
/// // Create new driver instance
/// let nfc = St25r95::new(MockSpi::default(), MockGpio)?;
///
/// // Select ISO14443A reader mode
/// let mut reader = nfc.protocol_select_iso14443a(Default::default())?;
///
/// // Send REQA command to detect cards
/// let response = reader.send_receive(&[0x26])?;
///
/// // Turn off field when done
/// let _nfc = reader.field_off()?;
/// # Ok::<(), st25r95::Error>(())
/// ```
///
/// ```rust
/// # use st25r95::{
/// #     mock::{MockGpio, MockSpi},
/// #     St25r95,
/// # };
/// // Card emulation example
/// let nfc = St25r95::new(MockSpi::default(), MockGpio)?;
///
/// // Select card emulation mode
/// let mut card = nfc.protocol_select_ce_iso14443a(Default::default())?;
///
/// // Enter listening mode
/// card.listen()?;
///
/// // Wait up to a second for a reader command
/// let command = card.receive(1_000)?;
///
/// // Process the command, then answer the reader
/// let response_data = [0x04, 0x00];
/// card.send(&response_data)?;
/// # Ok::<(), st25r95::Error>(())
/// ```
pub struct St25r95<S, G, F, R, P> {
    spi: S,
    gpio: G,
    dac_ref: Option<u8>,
    dac_guard: u8,
    /// Host-side deadline for the response of the selected configuration.
    response_timeout: u32,
    /// Set while a command has been sent and its response not yet consumed.
    pending_response: bool,
    field: PhantomData<F>,
    role: R,
    protocol: P,
}

impl<S: St25r95Spi, G: St25r95Gpio> St25r95<S, G, FieldOff, NoRole, NoProtocol> {
    pub fn new(spi: S, gpio: G) -> Result<Self> {
        // do not assume any state
        let mut st25r95 = Self {
            spi,
            gpio,
            dac_ref: None,
            dac_guard: 0,
            response_timeout: DEFAULT_RESPONSE_TIMEOUT_MS,
            pending_response: false,
            field: PhantomData,
            role: NoRole,
            protocol: NoProtocol,
        };
        // Startup sequence §3.2
        st25r95.gpio.irq_in_pulse_low();
        // should be in Ready state
        st25r95.reset()?;
        let (idn_str, _) = st25r95.idn()?;
        if !idn_str.starts_with("NFC") {
            return Err(Error::IdentificationError);
        }
        Ok(st25r95)
    }

    /// The Echo command verifies the possibility of communication between a Host and the
    /// ST25R95.
    ///
    /// Succeeds only when the chip answers with the single byte
    /// [`ECHO_BYTE`]. If it answers the Echo byte followed by a framed
    /// status - which is what happens when the Echo leaves Listen mode - that
    /// status is returned as an error instead of being discarded; use
    /// `cancel_listen` to leave Listen mode on purpose.
    pub fn echo(&mut self) -> Result<()> {
        self.spi.poll(PollFlags::CAN_SEND)?;
        self.send_command(Command::Echo, &[])?;
        self.poll_irq_out(DEFAULT_RESPONSE_TIMEOUT_MS)?;
        match self.read_echo()? {
            EchoResponse::Echo => Ok(()),
            EchoResponse::ListenCancelled(response) => response.expect_data_len(0),
        }
    }
}

impl<S: St25r95Spi, G: St25r95Gpio, R: Default, P: Default> St25r95<S, G, FieldOn, R, P> {
    pub fn field_off(mut self) -> ResultSt25r95FieldOff<S, G, R, P> {
        self.select_protocol(Protocol::FieldOff, protocol::FieldOff)?;
        Ok(self.transition(R::default(), P::default()))
    }

    /// The Echo command verifies the possibility of communication between a Host and the
    /// ST25R95.
    ///
    /// Succeeds only when the chip answers with the single byte
    /// [`ECHO_BYTE`]. If it answers the Echo byte followed by a framed
    /// status - which is what happens when the Echo leaves Listen mode - that
    /// status is returned as an error instead of being discarded; use
    /// `cancel_listen` to leave Listen mode on purpose.
    pub fn echo(&mut self) -> Result<()> {
        self.spi.poll(PollFlags::CAN_SEND)?;
        self.send_command(Command::Echo, &[])?;
        self.poll_irq_out(DEFAULT_RESPONSE_TIMEOUT_MS)?;
        match self.read_echo()? {
            EchoResponse::Echo => Ok(()),
            EchoResponse::ListenCancelled(response) => response.expect_data_len(0),
        }
    }
}

impl<S: St25r95Spi, G: St25r95Gpio, F, R, P> St25r95<S, G, F, R, P> {
    /// Move to another type state, carrying the runtime state along.
    fn transition<F2, R2, P2>(self, role: R2, protocol: P2) -> St25r95<S, G, F2, R2, P2> {
        St25r95 {
            spi: self.spi,
            gpio: self.gpio,
            dac_ref: self.dac_ref,
            dac_guard: self.dac_guard,
            response_timeout: self.response_timeout,
            pending_response: self.pending_response,
            field: PhantomData,
            role,
            protocol,
        }
    }

    fn reset(&mut self) -> Result<()> {
        self.spi.reset()?;
        // should be in Power-up state
        self.gpio.irq_in_pulse_low();
        Ok(())
    }

    /// Send a command packet after checking that the ST25R95 can encode its
    /// length.
    ///
    /// The host-to-chip wire format has a one-byte length field, so a payload
    /// longer than [`MAX_COMMAND_DATA_LEN`] cannot be announced to the chip.
    /// Rejecting it here means every [`St25r95Spi`] implementation is handed a
    /// slice it can encode, instead of having to truncate the length itself and
    /// desynchronize the command.
    ///
    /// Refuses to send anything while a previous response is still outstanding:
    /// the datasheet requires the previous operation to complete first, and a
    /// second command would race the answer to the first.
    fn send_command(&mut self, cmd: Command, data: &[u8]) -> Result<()> {
        if self.pending_response {
            return Err(Error::ResponseInProgress);
        }
        if data.len() > MAX_COMMAND_DATA_LEN {
            return Err(Error::InvalidDataLen(data.len()));
        }
        self.spi.send_command(cmd, data, false)?;
        self.pending_response = true;
        Ok(())
    }

    /// Read and validate the answer to an `Echo` command.
    ///
    /// Echo is the one command whose answer is not a `<Status><Len><Data>`
    /// frame, so it is read through [`St25r95Spi::read_echo`] rather than
    /// [`St25r95Spi::read_data`]: a bare [`ECHO_BYTE`] cannot be represented
    /// as a [`ReadResponse`], and squeezing it into one loses the framed
    /// status that follows when the Echo leaves Listen mode.
    fn read_echo(&mut self) -> Result<EchoResponse> {
        let mut buf = [0u8; ECHO_RESPONSE_MAX_LEN];
        let len = self.spi.read_echo(&mut buf)?;
        self.pending_response = false;
        // An implementation that reports more bytes than it could have written
        // is not trustworthy enough to parse.
        let read = buf.get(..len).ok_or(Error::EchoFailed)?;
        EchoResponse::try_from(read)
    }

    fn poll_irq_out(&mut self, timeout: u32) -> Result<()> {
        self.gpio
            .wait_irq_out_falling_edge(timeout)
            .map_err(|_| Error::ResponseInProgress)
    }

    /// Read a framed response and mark the outstanding command as answered.
    fn read_data(&mut self) -> Result<ReadResponse> {
        let response = self.spi.read_data()?;
        self.pending_response = false;
        Ok(response)
    }

    /// Wait for the response of the command in flight, within the deadline of
    /// the selected configuration.
    fn read(&mut self) -> Result<ReadResponse> {
        self.read_with_timeout(self.response_timeout)
    }

    fn read_with_timeout(&mut self, timeout: u32) -> Result<ReadResponse> {
        self.poll_irq_out(timeout)?;
        self.read_data()
    }

    /// Host-side deadline, in milliseconds, for the response of a command
    ///
    /// Derived from the timing parameters of the selected protocol (`FDT`,
    /// `FWT` or `RWT`) plus [`RESPONSE_TIMEOUT_MARGIN_MS`], and never shorter
    /// than [`DEFAULT_RESPONSE_TIMEOUT_MS`], so the host cannot give up on a
    /// frame wait the chip is still legitimately serving.
    pub fn response_timeout(&self) -> u32 {
        self.response_timeout
    }

    /// Override the host-side deadline for the response of a command
    ///
    /// Selecting a protocol recomputes the deadline from that protocol's
    /// timing parameters, so call this after `protocol_select_*`. Setting a
    /// deadline shorter than the configured device timeout makes
    /// [`Error::ResponseInProgress`] the expected outcome of a slow frame.
    pub fn set_response_timeout(&mut self, timeout: u32) {
        self.response_timeout = timeout;
    }

    /// Whether a command has been sent whose response has not been read yet
    ///
    /// This is what a host-side timeout leaves behind: the chip is still
    /// working on the command, so no new command can be sent until the answer
    /// is drained with [`poll_pending_response`](Self::poll_pending_response)
    /// or given up with
    /// [`discard_pending_response`](Self::discard_pending_response).
    pub fn is_response_pending(&self) -> bool {
        self.pending_response
    }

    /// Keep waiting for the response of a command that timed out on the host
    ///
    /// This is the recovery path after [`Error::ResponseInProgress`]: it waits
    /// for the operation already in flight rather than starting a new one, so a
    /// late answer can never be consumed as the response of a following
    /// command.
    pub fn poll_pending_response(&mut self, timeout: u32) -> Result<ReadResponse> {
        self.read_with_timeout(timeout)
    }

    /// Give up a pending response and put the SPI interface back in a known
    /// state
    ///
    /// Flushes whatever the chip may still be holding and clears the pending
    /// state, so commands can be sent again. The answer to the abandoned
    /// command is lost; prefer
    /// [`poll_pending_response`](Self::poll_pending_response) when the result
    /// still matters.
    pub fn discard_pending_response(&mut self) -> Result<()> {
        self.spi.flush()?;
        self.pending_response = false;
        Ok(())
    }

    /// The IDN command gives brief information about the ST25R95 and its revision.
    pub fn idn(&mut self) -> Result<(heapless::String<13>, u16)> {
        self.send_command(Command::Idn, &[])?;

        let response = self.read()?;
        response.expect_data_len(15)?;

        let idn_str = from_utf8(&response.data[..13])?;
        let mut idn_string = heapless::String::new();
        idn_string.push_str(idn_str).unwrap();
        let rom_crc = ((response.data[13] as u16) << 8) | response.data[14] as u16; // §4.1.1 SPI communication is MSB first.
        Ok((idn_string, rom_crc))
    }

    fn select_protocol(&mut self, protocol: Protocol, params: impl ProtocolParams) -> Result<()> {
        // The chip may legitimately spend the configured frame timeout on a
        // single exchange, so the host deadline follows it.
        let response_timeout = host_timeout_ms(params.timeout_us());
        let mut data = [0u8; 9];
        data[0] = protocol as u8;
        let (d, data_len) = params.data();
        if data_len > 0 {
            data[1..1 + data_len].copy_from_slice(&d[..data_len]);
        }

        self.send_command(Command::ProtocolSelect, &data[..1 + data_len])?;

        let response = self.read()?;
        response.expect_data_len(0)?;
        self.response_timeout = response_timeout;
        Ok(())
    }

    /// This command selects the RF communication protocol and prepares the ST25R95 for
    /// communication with contactless ISO/IEC 15693 tag.
    pub fn protocol_select_iso15693(
        mut self,
        params: iso15693::reader::Parameters,
    ) -> ResultSt25r95ReaderIso15693<S, G> {
        let modulation = params.get_modulation();
        self.select_protocol(Protocol::Iso15693, params)?;
        Ok(self.transition(Reader, Iso15693(modulation)))
    }

    /// This command selects the RF communication protocol and prepares the ST25R95 for
    /// communication with contactless ISO/IEC 14443-A tag.
    pub fn protocol_select_iso14443a(
        mut self,
        params: iso14443a::reader::Parameters,
    ) -> ResultSt25r95ReaderIso14443A<S, G> {
        self.select_protocol(Protocol::Iso14443A, params)?;
        Ok(self.transition(Reader, Iso14443A))
    }

    /// This command selects the RF communication protocol and prepares the ST25R95 for
    /// communication with contactless ISO/IEC 14443-B tag.
    pub fn protocol_select_iso14443b(
        mut self,
        params: iso14443b::reader::Parameters,
    ) -> ResultSt25r95ReaderIso14443B<S, G> {
        self.select_protocol(Protocol::Iso14443B, params)?;
        Ok(self.transition(Reader, Iso14443B))
    }

    /// This command selects the RF communication protocol and prepares the ST25R95 for
    /// communication with contactless FeliCa tag.
    pub fn protocol_select_felica(
        mut self,
        params: felica::reader::Parameters,
    ) -> ResultSt25r95ReaderFelica<S, G> {
        self.select_protocol(Protocol::FeliCa, params)?;
        Ok(self.transition(Reader, FeliCa))
    }

    /// This command selects the RF communication protocol and prepares the ST25R95 for
    /// communication with a reader in Card Emulation with ISO/IEC 14443-A.
    pub fn protocol_select_ce_iso14443a(
        mut self,
        params: iso14443a::card_emulation::Parameters,
    ) -> ResultSt25r95CardEmulationIso14443A<S, G> {
        self.select_protocol(Protocol::CardEmulationIso14443A, params)?;
        Ok(self.transition(CardEmulation(false), Iso14443A))
    }

    /// This command can be used to detect the presence/absence of an HF field by
    /// monitoring the field detector (FieldDet) flag. It can be used as well to wait for
    /// HF field appearance or disappearance until a defined timeout expires. The answer
    /// to the PollField command is the value of the FieldDet flag.
    /// The result of this command depends on the protocol selected. If a reader mode
    /// protocol is selected, the flag FieldDet is set to ‘1’ because the RF field is
    /// turned ON by the reader.
    pub fn poll_field(&mut self, wff: Option<WaitForField>) -> Result<bool> {
        match wff {
            None => self.send_command(Command::PollField, &[])?,
            Some(WaitForField {
                apparance,
                presc,
                timer,
            }) => self.send_command(Command::PollField, &[apparance as u8, presc, timer])?,
        }

        let response = self.read()?;
        response.ensure_ok()?;
        match response.data.len() {
            0 => Ok(false),
            1 => Ok(response.data[0] & 0x01 == 1),
            _ => Err(Error::InvalidResponseLength {
                expected: 1,
                actual: response.data.len(),
            }),
        }
    }

    fn _idle_send(&mut self, mut params: IdleParams, check_params: bool) -> Result<()> {
        if check_params && params.wus.tag_detection {
            match self.dac_ref {
                None => return Err(Error::CalibrationNeeded),
                Some(dac_ref) => {
                    params.dac_data.high =
                        dac_ref
                            .checked_add(self.dac_guard)
                            .ok_or(Error::TagDetector {
                                dac_ref,
                                dac_guard: self.dac_guard,
                            })?;
                    params.dac_data.low =
                        dac_ref
                            .checked_sub(self.dac_guard)
                            .ok_or(Error::TagDetector {
                                dac_ref,
                                dac_guard: self.dac_guard,
                            })?;
                }
            }
        }
        self.send_command(Command::Idle, &params.data())?;
        Ok(())
    }

    fn _ack_idle(&mut self) -> Result<WakeUpSource> {
        let response = self.read_data()?;
        response.expect_data_len(1)?;
        WakeUpSource::try_from(response.data[0])
            .map_err(|_| Error::InvalidWakeUpSource(response.data[0]))
    }

    fn _idle(
        &mut self,
        params: IdleParams,
        check_params: bool,
        timeout: u32,
    ) -> Result<WakeUpSource> {
        self._idle_send(params, check_params)?;
        self.poll_irq_out(timeout)?;
        self._ack_idle()
    }

    /// This command switches the ST25R95 into low power consumption mode and defines the
    /// way to return to Ready state.
    ///
    /// Caution:
    /// In low power consumption mode the device does not support SPI poll mechanism.
    /// Application has to rely on IRQ_OUT before reading the answer to the Idle command.
    ///
    /// `timeout` is the host-side deadline in milliseconds. An Idle can
    /// legitimately last seconds, so there is no useful default: pass at least
    /// as long as the configured wake-up sources can take.
    /// [`IdleParams::duration_before_timeout`] gives that duration, in
    /// microseconds, for a timeout wake-up. If the configured wake-up has no
    /// bound - field or tag detection - use [`idle_async`](Self::idle_async)
    /// and let the application wait on IRQ_OUT instead.
    ///
    /// Returning [`Error::ResponseInProgress`] means the deadline elapsed while
    /// the chip was still asleep; it stays asleep, and the answer can be
    /// collected later with [`ack_idle`](Self::ack_idle).
    pub fn idle(&mut self, params: IdleParams, timeout: u32) -> Result<WakeUpSource> {
        self._idle(params, true, timeout)
    }

    /// Send the Idle command without waiting for the chip to wake up.
    ///
    /// Use this when the wake-up event will be signalled out-of-band (typically a
    /// GPIO interrupt on IRQ_OUT delivered to the application). After IRQ_OUT
    /// goes low the caller must read the deferred response by calling
    /// [`ack_idle`](Self::ack_idle); failing to do so leaves the response byte
    /// in the chip's SPI buffer, which the next SPI command may read instead of
    /// its own response.
    ///
    /// As with [`idle`](Self::idle), passing a [`WakeUpSource`] with
    /// `tag_detection: true` requires a prior successful
    /// [`calibrate_tag_detector`](Self::calibrate_tag_detector) call.
    pub fn idle_async(&mut self, params: IdleParams) -> Result<()> {
        self._idle_send(params, true)
    }

    /// Read and parse the deferred response of an [`idle_async`](Self::idle_async).
    ///
    /// The caller is responsible for ensuring IRQ_OUT has gone low before
    /// calling this method (otherwise the SPI read will return whatever stale
    /// bytes are in the chip's buffer). On success returns the
    /// [`WakeUpSource`] indicating which wake-up event woke the chip.
    pub fn ack_idle(&mut self) -> Result<WakeUpSource> {
        self._ack_idle()
    }

    fn _write_register(
        &mut self,
        reg: &impl Register,
        inc_addr: bool,
        value: Option<u8>,
    ) -> Result<()> {
        let mut data = [0u8; 4];
        data[0] = reg.write_addr();
        data[1] = inc_addr as u8;

        let data_len = if reg.has_index() {
            data[2] = reg.index_confirmation();
            if let Some(value) = value {
                data[3] = value;
                4
            } else {
                3
            }
        } else if let Some(value) = value {
            data[2] = value;
            3
        } else {
            2
        };
        self.send_command(Command::WrReg, &data[..data_len])?;

        let response = self.read()?;
        response.expect_data_len(0)
    }

    fn read_register(&mut self, reg: &impl Register) -> Result<u8> {
        if reg.has_index() {
            // Set register index first
            self._write_register(reg, false, None)?;
        }
        let mut data = [0u8; 3];
        data[0] = reg.read_addr();
        data[1] = 0x01;
        data[2] = 0x00;
        self.send_command(Command::RdReg, &data)?;

        let response = self.read()?;
        response.expect_data_len(1)?;
        Ok(response.data[0])
    }

    /// This command is used to read the Wakeup register.
    pub fn wakeup_source(&mut self) -> Result<WakeUpSource> {
        let reg = Wakeup;
        let value = self.read_register(&reg)?;
        value
            .try_into()
            .map_err(|_| Error::InvalidWakeUpSource(value))
    }

    /// Calibrate the tag detector as wake-up source by an iterrative process.
    pub fn calibrate_tag_detector(&mut self) -> Result<u8> {
        let mut params = IdleParams {
            wus: WakeUpSource {
                lfo_freq: LFOFreq::KHz32,
                ss_low_pulse: false,
                irq_in_low_pulse: false,
                field_detection: false,
                tag_detection: true,
                timeout: true,
            },
            enter_ctrl: CtrlResConf {
                field_detect_aux_enabled: false,
                field_detector_enabled: false,
                iref_enabled: false,
                dac_comp_high: true,
                lfo_enabled: true,
                hfo_enabled: false,
                vdda_enabled: false,
                hibernate_state_enabled: false,
                sleep_state_enabled: true,
            },
            wu_ctrl: CtrlResConf {
                field_detect_aux_enabled: false,
                field_detector_enabled: false,
                iref_enabled: true,
                dac_comp_high: true,
                lfo_enabled: true,
                hfo_enabled: true,
                vdda_enabled: true,
                hibernate_state_enabled: false,
                sleep_state_enabled: false,
            },
            wu_period: 0,
            dac_data: DacData {
                low: 0x00,
                high: 0x00,
            },
            max_sleep: 0x01,
            ..Default::default()
        };
        let idle_timeout = host_timeout_ms(Some(params.duration_before_timeout()));
        let wus = self._idle(params, false, idle_timeout)?;
        if !wus.tag_detection {
            return Err(Error::CalibTagDetectionFailed);
        }
        params.dac_data.high = 0xFC; // max value
        let mut wus = self._idle(params, false, idle_timeout)?;
        if !wus.timeout {
            return Err(Error::CalibTimeoutFailed);
        }
        for &val in [0x80, 0x40, 0x20, 0x10, 0x08, 0x04].iter() {
            if wus.timeout {
                params.dac_data.high = adjust_calibration_dac_high(params.dac_data.high, -val)?;
            } else if wus.tag_detection {
                params.dac_data.high = adjust_calibration_dac_high(params.dac_data.high, val)?;
            }
            wus = self._idle(params, false, idle_timeout)?;
        }
        if wus.timeout {
            params.dac_data.high = adjust_calibration_dac_high(params.dac_data.high, -0x04)?;
        }
        self.dac_ref = Some(params.dac_data.high);
        Ok(params.dac_data.high)
    }
}

fn adjust_calibration_dac_high(dac_high: u8, delta: i16) -> Result<u8> {
    let next = i16::from(dac_high) + delta;
    if (0..=i16::from(u8::MAX)).contains(&next) {
        Ok(next as u8)
    } else {
        Err(Error::CalibDacOutOfRange { dac_high, delta })
    }
}

impl<S: St25r95Spi, G: St25r95Gpio, P: Default> St25r95<S, G, FieldOn, Reader, P> {
    /// This command sends data to a contactless tag and receives its reply.
    /// If the tag response was received and decoded correctly, the `<Data>` field can
    /// contain additional information which is protocol-specific.
    ///
    /// This returns the raw command response. Protocol operations can validly surface
    /// status bytes that callers may want to inspect themselves, so this method does
    /// not call [`ReadResponse::ensure_ok`].
    ///
    /// `data` must be at most [`MAX_COMMAND_DATA_LEN`] bytes long: the command
    /// announces its payload with a single length byte, so a longer slice is
    /// rejected with [`Error::InvalidDataLen`] before anything is sent.
    pub fn send_receive(&mut self, data: &[u8]) -> Result<ReadResponse> {
        self.send_command(Command::SendRecv, data)?;
        self.read()
    }

    /// This command is used to read the ARC_B register.
    pub fn read_arc_b(&mut self) -> Result<ArcB> {
        ArcB::from_u8(self.read_register(&ArcB::fake())?)
    }

    pub fn write_arc_b(&mut self, arc_b: ArcB) -> Result<()> {
        self._write_register(&arc_b, false, Some(arc_b.value()))
    }
}

impl<S: St25r95Spi, G: St25r95Gpio> St25r95<S, G, FieldOn, Reader, Iso15693> {
    pub fn new_arc_b(
        &self,
        modulation_index: ModulationIndex,
        receiver_gain: ReceiverGain,
    ) -> Result<ArcB> {
        // See Table 35
        if match self.protocol.0 {
            Modulation::Percent10 => [
                ModulationIndex::Percent30,
                ModulationIndex::Percent33,
                ModulationIndex::Percent36,
            ]
            .contains(&modulation_index),
            Modulation::Percent100 => [ModulationIndex::Percent95].contains(&modulation_index),
        } {
            Ok(ArcB {
                modulation_index,
                receiver_gain,
            })
        } else {
            Err(Error::InvalidModulationIndex(modulation_index as u8))
        }
    }

    pub fn default_arc_b(&self) -> ArcB {
        // See Table 35
        ArcB {
            modulation_index: match self.protocol.0 {
                Modulation::Percent10 => ModulationIndex::Percent33,
                Modulation::Percent100 => ModulationIndex::Percent95,
            },
            receiver_gain: ReceiverGain::Db27,
        }
    }
}

impl<S: St25r95Spi, G: St25r95Gpio> St25r95<S, G, FieldOn, Reader, Iso14443A> {
    pub fn new_arc_b(
        &self,
        modulation_index: ModulationIndex,
        receiver_gain: ReceiverGain,
    ) -> Result<ArcB> {
        // See Table 35
        if [ModulationIndex::Percent95].contains(&modulation_index) {
            Ok(ArcB {
                modulation_index,
                receiver_gain,
            })
        } else {
            Err(Error::InvalidModulationIndex(modulation_index as u8))
        }
    }

    pub fn default_arc_b(&self) -> ArcB {
        // See Table 35
        ArcB {
            modulation_index: ModulationIndex::Percent95,
            receiver_gain: ReceiverGain::Db8,
        }
    }

    pub fn new_timer_window(&self, timer_w: u8) -> Result<TimerWindow> {
        if (0x50..=0x60).contains(&timer_w) {
            Ok(TimerWindow(timer_w))
        } else {
            Err(Error::InvalidU8Parameter {
                min: 0x50,
                max: 0x60,
                actual: timer_w,
            })
        }
    }

    pub fn default_timer_window(&self) -> TimerWindow {
        // See §5.11.2
        TimerWindow(0x52)
    }

    pub fn recommended_timer_window(&self) -> TimerWindow {
        // See §5.11.2
        TimerWindow(0x56)
    }

    /// To improve ST25R95 demodulation when communicating with ISO/IEC 14443 Type A tags,
    /// it is possible to adjust the synchronization between digital and analog inputs
    /// by fine-tuning the Timer Window value.
    /// The default values of these parameters are set by the ProtocolSelect command, but
    /// they can be overwritten using this function.
    pub fn write_timer_windows(&mut self, timer_w: TimerWindow) -> Result<()> {
        self._write_register(&timer_w, false, Some(timer_w.value()))
    }
}

impl<S: St25r95Spi, G: St25r95Gpio> St25r95<S, G, FieldOn, Reader, Iso14443B> {
    pub fn new_arc_b(
        &self,
        modulation_index: ModulationIndex,
        receiver_gain: ReceiverGain,
    ) -> Result<ArcB> {
        // See Table 35
        if [
            ModulationIndex::Percent10,
            ModulationIndex::Percent17,
            ModulationIndex::Percent25,
            ModulationIndex::Percent30,
        ]
        .contains(&modulation_index)
        {
            Ok(ArcB {
                modulation_index,
                receiver_gain,
            })
        } else {
            Err(Error::InvalidModulationIndex(modulation_index as u8))
        }
    }

    pub fn default_arc_b(&self) -> ArcB {
        // See Table 35
        ArcB {
            modulation_index: ModulationIndex::Percent17,
            receiver_gain: ReceiverGain::Db34,
        }
    }
}

impl<S: St25r95Spi, G: St25r95Gpio> St25r95<S, G, FieldOn, Reader, FeliCa> {
    pub fn new_arc_b(
        &self,
        modulation_index: ModulationIndex,
        receiver_gain: ReceiverGain,
    ) -> Result<ArcB> {
        // See Table 35
        if [
            ModulationIndex::Percent10,
            ModulationIndex::Percent17,
            ModulationIndex::Percent25,
            ModulationIndex::Percent30,
        ]
        .contains(&modulation_index)
        {
            Ok(ArcB {
                modulation_index,
                receiver_gain,
            })
        } else {
            Err(Error::InvalidModulationIndex(modulation_index as u8))
        }
    }

    pub fn default_arc_b(&self) -> ArcB {
        // See Table 35
        ArcB {
            modulation_index: ModulationIndex::Percent17,
            receiver_gain: ReceiverGain::Db34,
        }
    }

    /// To improve ST25R95 reception when communicating with FeliCa™ tags, it is possible
    /// to enable an AutoDetect filter to synchronize FeliCa™ tags with the ST25R95.
    /// By default, this filter is disabled after the execution of the ProtocolSelect
    /// command, but it can be enabled using this function.
    pub fn enable_autodetect_filter(&mut self) -> Result<()> {
        let reg = AutoDetectFilter;
        self._write_register(&reg, false, Some(reg.value()))
    }
}

impl<S: St25r95Spi, G: St25r95Gpio> St25r95<S, G, FieldOn, CardEmulation, Iso14443A> {
    /// In card emulation mode, this function puts the ST25R95 in Listening mode.
    /// The ST25R95 will exit Listening mode as soon it receives the Echo command from the
    /// Host Controller (MCU) or a command from an external reader (not including commands
    /// supported by the AC filter command).
    /// If no command from an external reader has been received, then the Echo command
    /// must be used to exit the Listening mode prior to sending a new command to the
    /// ST25R95.
    pub fn listen(&mut self) -> Result<()> {
        self.send_command(Command::Listen, &[])?;

        let response = self.read()?;
        response.expect_data_len(0)?;
        self.role.0 = true;
        Ok(())
    }

    /// Receive data from the reader through the ST25R95 in Listen mode.
    ///
    /// Blocks until the chip reports a command from an external reader or
    /// `timeout` milliseconds elapse. On timeout it returns
    /// [`Error::ResponseInProgress`] and the chip stays in Listen mode: call
    /// again to keep waiting, or [`cancel_listen`](Self::cancel_listen) to
    /// leave it.
    pub fn receive(&mut self, timeout: u32) -> Result<ReadResponse> {
        self.read_with_timeout(timeout)
    }

    /// Immediately sends data to the reader using the Load Modulation method.
    ///
    /// `data` must be at most [`MAX_COMMAND_DATA_LEN`] bytes long: the command
    /// announces its payload with a single length byte, so a longer slice is
    /// rejected with [`Error::InvalidDataLen`] before anything is sent.
    pub fn send(&mut self, data: &[u8]) -> Result<()> {
        self.send_command(Command::Send, data)?;

        let response = self.read()?;
        response.expect_data_len(0)
    }

    pub fn new_acc_a(
        &self,
        load_modulation_index: LoadModulationIndex,
        demodulator_sensitivity: DemodulatorSensitivity,
    ) -> Result<AccA> {
        // See Table 36
        if demodulator_sensitivity != DemodulatorSensitivity::Percent100 {
            Err(Error::InvalidDemodulatorSensitivity(
                demodulator_sensitivity as u8,
            ))
        } else {
            Ok(AccA {
                load_modulation_index,
                demodulator_sensitivity,
            })
        }
    }

    pub fn default_acc_a(&self) -> AccA {
        AccA {
            load_modulation_index: LoadModulationIndex::default(),
            demodulator_sensitivity: DemodulatorSensitivity::Percent100,
        }
    }

    pub fn recommended_acc_a(&self) -> AccA {
        self.default_acc_a()
    }

    /// This command is used to read the ACC_A register.
    pub fn read_acc_a(&mut self) -> Result<AccA> {
        AccA::from_u8(self.read_register(&self.default_acc_a())?)
    }

    /// Adjusting the Load modulation index and Demodulator sensitivity parameters in card
    /// emulation mode can help to improve application behavior.
    /// The default values of these parameters are set by the ProtocolSelect command, but
    /// they can be overwritten using this function.
    pub fn write_acc_a(&mut self, acc_a: AccA) -> Result<()> {
        self._write_register(&acc_a, false, Some(acc_a.value()))
    }

    /// This command activates the anti-collision filter in Type A card emulation mode.
    ///
    /// ## Parameters
    /// - cascade_level_filter: 1 to 3 UIDs, other number will return
    ///   InvalidCascadeLevelFilterCount
    pub fn activate_ac_filter(
        &mut self,
        atqa: ATQA,
        sak: SAK,
        cascade_level_filter: impl IntoIterator<Item = UID>,
    ) -> Result<()> {
        let mut uid_count = 0;
        let mut data = [0u8; 15];
        data[0..2].copy_from_slice(&atqa.to_le_bytes());
        data[2] = sak;
        for uid in cascade_level_filter.into_iter() {
            if uid_count >= 3 {
                return Err(Error::InvalidCascadeLevelFilterCount(uid_count + 1));
            }
            let uid_offset = 3 + uid_count * uid.len();
            data[uid_offset..uid_offset + uid.len()].copy_from_slice(uid.as_slice());
            uid_count += 1;
        }
        if uid_count == 0 {
            return Err(Error::InvalidCascadeLevelFilterCount(uid_count));
        }
        let data_len = 3 + uid_count * core::mem::size_of::<UID>();
        self.send_command(Command::ACFilter, &data[..data_len])?;

        let response = self.read()?;
        response.expect_data_len(0)
    }

    fn ac_filter_state(&mut self, data: &[u8]) -> Result<AntiColState> {
        self.send_command(Command::ACFilter, data)?;

        let response = self.read()?;
        response.expect_data_len(1)?;
        AntiColState::try_from(response.data[0])
            .map_err(|_| Error::InvalidAntiColState(response.data[0]))
    }

    /// This command de-activates the anti-collision filter in Type A card emulation mode.
    pub fn deactivate_ac_filter(&mut self) -> Result<AntiColState> {
        self.ac_filter_state(&[])
    }

    /// This command read the Anti-Collision Filter state in Type A card emulation mode.
    /// Does not de-activate the filter.
    pub fn anti_collision_state(&mut self) -> Result<AntiColState> {
        self.ac_filter_state(&[0x00, 0x00])
    }

    /// This command sets the Anti-Collision Filter state in Type A card emulation mode.
    pub fn set_anti_collision_state(&mut self, state: AntiColState) -> Result<()> {
        self.send_command(Command::ACFilter, &[state as u8])?;

        let response = self.read()?;
        response.expect_data_len(0)
    }

    /// Cancel an active Listen mode by sending an Echo command from the host.
    ///
    /// Per the datasheet, the chip exits Listen mode when it receives an Echo
    /// from the MCU. It then answers [`ECHO_BYTE`] followed by the framed
    /// `UserStop` response reporting why listening stopped; this helper
    /// consumes both and flips the internal listen flag back to false. Use it
    /// when the application needs to abort an outstanding `listen()` (e.g.
    /// session timeout) before sending another command.
    ///
    /// A chip that answers the Echo byte alone had already left Listen mode -
    /// an external reader command ended it - which is also a success.
    pub fn cancel_listen(&mut self) -> Result<()> {
        self.spi.poll(PollFlags::CAN_SEND)?;
        self.send_command(Command::Echo, &[])?;
        self.poll_irq_out(DEFAULT_RESPONSE_TIMEOUT_MS)?;
        match self.read_echo()? {
            EchoResponse::Echo => {}
            EchoResponse::ListenCancelled(response) => match response.expect_data_len(0) {
                // Leaving Listen mode on request is reported as `UserStop`;
                // here that is the expected outcome, not a failure.
                Ok(()) | Err(Error::Hw(St25r95Error::UserStop)) => {}
                Err(error) => return Err(error),
            },
        }
        self.role.0 = false;
        Ok(())
    }
}

/// Response from ST25R95 command operations
///
/// This structure represents the complete response received from the ST25R95
/// after executing a command. It contains both the status/error information
/// and any data payload returned by the chip.
///
/// ## Response Format
///
/// The ST25R95 uses a specific response format that supports both short and
/// long frames:
///
/// ```text
/// [Status Byte] [Length Byte 1] [Length Byte 2] [Data...]
/// ```
///
/// - **Status Byte**: Error flags and status indicators
/// - **Length Bytes**: Data length (supports up to 530 bytes)
/// - **Data**: Optional payload data (protocol-specific)
///
/// ## Status Code Interpretation
///
/// The status code contains error information and status flags:
///
/// - **Bit 7**: Protocol error flag
/// - **Bit 6**: Collision detected flag
/// - **Bit 5-4**: Reserved
/// - **Bit 3-0**: Error code (see St25r95Error enum)
///
/// A status code of 0x00 indicates success with no errors.
/// Non-zero status codes should be interpreted using the `St25r95Error` enum.
///
/// ## Data Length Handling
///
/// The ST25R95 supports long frames for protocols that require large data
/// transfers. The length encoding follows a specific pattern:
///
/// - **Short frames** (0-255 bytes): Standard single-byte length
/// - **Long frames** (256-530 bytes): Extended length encoding
///
/// ## Usage Examples
///
/// ```rust
/// # use st25r95::{Error, ReadResponse};
/// // A response to a REQA command, as `send_receive` returns it
/// let response = ReadResponse::try_from([0x80, 0x02, 0x44, 0x00].as_slice())?;
///
/// // Check for errors
/// response.ensure_ok()?;
///
/// // Process response data
/// match response.data.len() {
///     0 => { /* no data returned */ }
///     2 => { /* ATQA */ }
///     _ => { /* protocol-specific payload */ }
/// }
///
/// // Validate expected response length
/// response.expect_data_len(2)?; // Expect exactly 2 bytes for ATQA
/// # Ok::<(), Error>(())
/// ```
///
/// ## Error Handling
///
/// Always check the status code before processing data:
///
/// ```rust
/// # use st25r95::{Error, ReadResponse, Result, St25r95Error};
/// fn inspect(response: &ReadResponse) -> Result<()> {
///     match response.ensure_ok() {
///         Ok(()) => Ok(()),
///         Err(Error::Hw(St25r95Error::FrameTimeoutOrNoTag)) => {
///             // No tag present, handle gracefully
///             Ok(())
///         }
///         Err(Error::Hw(St25r95Error::CrcError)) => {
///             // Data corruption, the caller may retry the operation
///             Ok(())
///         }
///         // Other hardware error
///         Err(error) => Err(error),
///     }
/// }
/// # let timeout = ReadResponse::try_from([0x87, 0x00].as_slice())?;
/// # assert_eq!(inspect(&timeout), Ok(()));
/// # Ok::<(), Error>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ReadResponse {
    /// Status/error code from the ST25R95
    ///
    /// This byte contains error flags and status information:
    /// - 0x00: Success, no errors
    /// - Other values: Error conditions (see St25r95Error)
    ///
    /// Use [`ReadResponse::ensure_ok`] to convert error status codes into
    /// [`Error::Hw`].
    pub code: u8,

    /// Data payload returned by the ST25R95
    ///
    /// Contains the response data specific to the command and protocol used.
    /// The content and format depend on the operation performed:
    ///
    /// - **Identification**: Chip information and version
    /// - **Protocol operations**: Tag responses, authentication data, etc.
    /// - **Register reads**: Register values
    /// - **Field status**: Field detection results
    ///
    /// The maximum size is limited by `MAX_BUFFER_SIZE` (530 bytes).
    pub data: heapless::Vec<u8, MAX_BUFFER_SIZE>,
}

impl ReadResponse {
    /// Extract the status code from a response header byte
    ///
    /// This method preserves hardware error status bytes, while normalizing the
    /// long-frame success header variants that encode payload length bits in the
    /// status byte.
    ///
    /// ## Status Byte Format
    /// ```text
    /// Bit 7: Protocol error flag
    /// Bit 6: Collision detected flag
    /// Bit 5-4: Reserved
    /// Bit 3-0: Error code (filtered by this method)
    /// ```
    ///
    /// ## Parameters
    /// - `value`: Raw status byte from the ST25R95 response
    ///
    /// ## Returns
    /// Status code that can be converted to `St25r95Error`
    ///
    /// ## Example
    /// ```rust
    /// # use st25r95::{ReadResponse, St25r95Error};
    /// let raw_status = 0x8D; // CRC error with protocol flag
    /// let error_code = ReadResponse::code(raw_status);
    /// assert_eq!(error_code, 0x8D);
    /// assert_eq!(St25r95Error::from(error_code), St25r95Error::CrcError);
    /// ```
    pub fn code(value: u8) -> u8 {
        if value & 0x8F == 0x80 {
            value & 0x90
        } else {
            value
        }
    }

    /// Validate that the response status is not a hardware error.
    ///
    /// `0x80` and `0x90` are accepted because they are used by the ST25R95
    /// response framing for successfully received frames.
    pub fn ensure_ok(&self) -> Result<()> {
        match self.code {
            0x00 | 0x80 | 0x90 => Ok(()),
            code => Err(Error::Hw(St25r95Error::from(code))),
        }
    }

    /// Decode data length from ST25R95 response header
    ///
    /// The ST25R95 uses a variable-length encoding to support both short and
    /// long frames. This method decodes the length bytes from the response
    /// header according to the protocol specification.
    ///
    /// ## Length Encoding
    ///
    /// **Short frames (0-255 bytes)**:
    /// ```text
    /// [0x00-0x7F] [Length] -> Length = second byte
    /// ```
    ///
    /// **Long frames (256-530 bytes)**:
    /// ```text
    /// [0x80-0x8F] [Length] -> Length = 256 + second byte + (bits 5-6 << 8)
    /// ```
    ///
    /// ## Parameters
    /// - `value`: Array containing the two length bytes from response header
    ///
    /// ## Returns
    /// Decoded data length in bytes (0-530)
    ///
    /// ## Example
    /// ```rust
    /// # use st25r95::ReadResponse;
    /// // Short frame: 10 bytes
    /// assert_eq!(ReadResponse::data_len([0x00, 10]), 10);
    ///
    /// // Long frame: 300 bytes
    /// assert_eq!(ReadResponse::data_len([0xA0, 44]), 300);
    /// ```
    pub fn data_len(value: [u8; 2]) -> usize {
        // See datasheet section 4.3 (Support of long frames)
        value[1] as usize
            | if value[0] & 0x8F == 0x80 {
                (value[0] as usize & 0b0110_0000) << 3
            } else {
                0
            }
    }

    /// Validate that response contains expected number of data bytes
    ///
    /// This convenience method first validates the response status with
    /// [`ReadResponse::ensure_ok`], then validates that the response data length
    /// matches the expected length for the specific command/protocol.
    ///
    /// ## Parameters
    /// - `expected`: Expected number of data bytes
    ///
    /// ## Returns
    /// - `Ok(())`: Response status is OK and length matches expectation
    /// - `Err(Error::Hw(_))`: Hardware reported a non-success status
    /// - `Err(Error::InvalidResponseLength)`: Length mismatch
    ///
    /// ## Example
    /// ```rust
    /// # use st25r95::{Error, ReadResponse};
    /// # let response = ReadResponse::try_from([0x80, 0x02, 0x44, 0x00].as_slice())?;
    /// // ISO14443A ATQA should be exactly 2 bytes
    /// response.expect_data_len(2)?;
    /// let atqa = u16::from_le_bytes([response.data[0], response.data[1]]);
    /// assert_eq!(atqa, 0x0044);
    /// # Ok::<(), Error>(())
    /// ```
    ///
    /// ## Common Expected Lengths
    ///
    /// - **REQA (ISO14443A)**: 2 bytes (ATQA)
    /// - **Read UID (ISO14443A)**: 5-10 bytes (depending on UID size)
    /// - **Inventory (ISO15693)**: Variable, depends on number of tags
    /// - **IDN command**: 13 bytes + 2 bytes CRC
    pub fn expect_data_len(&self, expected: usize) -> Result<()> {
        self.ensure_ok()?;
        if self.data.len() != expected {
            Err(Error::InvalidResponseLength {
                expected,
                actual: self.data.len(),
            })
        } else {
            Ok(())
        }
    }
}

impl TryFrom<&[u8]> for ReadResponse {
    type Error = crate::Error;
    fn try_from(value: &[u8]) -> core::result::Result<Self, Self::Error> {
        if value.len() < 2 {
            return Err(Error::InvalidDataLen(value.len()));
        }
        // See datasheet section 4.3 (Support of long frames)
        let data_len = Self::data_len(value[..2].try_into().unwrap());
        if data_len != value.len() - 2 {
            return Err(Error::InvalidDataLen(value.len()));
        }
        let mut data = heapless::Vec::new();
        data.extend_from_slice(&value[2..data_len + 2])?;
        Ok(Self {
            code: ReadResponse::code(value[0]),
            data,
        })
    }
}

/// Answer of the ST25R95 to an `Echo` command
///
/// Echo is the only command whose answer is not a `<Status><Len><Data>` frame,
/// so it cannot be described by a [`ReadResponse`] alone. The chip has two
/// documented ways of answering (datasheet DS12807):
///
/// ```text
/// Ordinary Echo:      0x55
/// Echo leaving Listen: 0x55 0x85 0x00
/// ```
///
/// The second form is the Echo byte followed by a normal frame carrying the
/// reason listening stopped, which is why both parts are kept here instead of
/// being collapsed into a single status byte.
// `ReadResponse` embeds the maximum frame buffer, and `no_std` leaves nowhere to
// box it. The value only lives on the stack for the duration of one Echo, just
// like every other frame the driver reads.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub enum EchoResponse {
    /// The chip answered with exactly [`ECHO_BYTE`] and nothing else.
    Echo,

    /// The chip answered [`ECHO_BYTE`] followed by a framed response.
    ///
    /// This happens when the Echo was used to leave Listen mode; the frame
    /// then reports [`St25r95Error::UserStop`].
    ListenCancelled(ReadResponse),
}

impl TryFrom<&[u8]> for EchoResponse {
    type Error = crate::Error;

    /// Parse the raw bytes returned by [`St25r95Spi::read_echo`].
    ///
    /// Anything that is not an exact Echo byte, optionally followed by a
    /// well-formed frame, is rejected: a short read, trailing bytes, or a
    /// stale response left over from a previous command all fail here rather
    /// than being reported as a successful Echo.
    fn try_from(value: &[u8]) -> core::result::Result<Self, Self::Error> {
        match value {
            [ECHO_BYTE] => Ok(Self::Echo),
            [ECHO_BYTE, frame @ ..] => Ok(Self::ListenCancelled(ReadResponse::try_from(frame)?)),
            _ => Err(Error::EchoFailed),
        }
    }
}

/// The crate README, compiled as a doctest so its examples cannot go stale.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct Readme;

#[cfg(test)]
mod tests {
    use {super::*, core::ops::Range};

    struct NoopSpi;

    impl St25r95Spi for NoopSpi {
        fn poll(&mut self, _flags: PollFlags) -> Result<()> {
            Ok(())
        }

        fn reset(&mut self) -> Result<()> {
            Ok(())
        }

        fn send_command(&mut self, _cmd: Command, _data: &[u8], _sod: bool) -> Result<()> {
            Ok(())
        }

        fn read_data(&mut self) -> Result<ReadResponse> {
            Ok(ReadResponse {
                code: 0,
                data: heapless::Vec::new(),
            })
        }

        fn read_echo(&mut self, buf: &mut [u8; ECHO_RESPONSE_MAX_LEN]) -> Result<usize> {
            buf[0] = ECHO_BYTE;
            Ok(1)
        }

        fn flush(&mut self) -> Result<()> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingSpi {
        command: Option<Command>,
        data: heapless::Vec<u8, MAX_BUFFER_SIZE>,
        sod: bool,
        response_code: u8,
        response_data: heapless::Vec<u8, MAX_BUFFER_SIZE>,
    }

    impl RecordingSpi {
        fn with_response_code(response_code: u8) -> Self {
            Self {
                response_code,
                ..Default::default()
            }
        }
    }

    impl St25r95Spi for RecordingSpi {
        fn poll(&mut self, _flags: PollFlags) -> Result<()> {
            Ok(())
        }

        fn reset(&mut self) -> Result<()> {
            Ok(())
        }

        fn send_command(&mut self, cmd: Command, data: &[u8], sod: bool) -> Result<()> {
            self.command = Some(cmd);
            self.data.clear();
            self.data.extend_from_slice(data).unwrap();
            self.sod = sod;
            Ok(())
        }

        fn read_data(&mut self) -> Result<ReadResponse> {
            Ok(ReadResponse {
                code: self.response_code,
                data: self.response_data.clone(),
            })
        }

        fn read_echo(&mut self, buf: &mut [u8; ECHO_RESPONSE_MAX_LEN]) -> Result<usize> {
            buf[0] = ECHO_BYTE;
            Ok(1)
        }

        fn flush(&mut self) -> Result<()> {
            Ok(())
        }
    }

    struct WakeSequenceSpi<const N: usize> {
        wake_sources: [u8; N],
        index: usize,
    }

    impl<const N: usize> WakeSequenceSpi<N> {
        fn new(wake_sources: [u8; N]) -> Self {
            Self {
                wake_sources,
                index: 0,
            }
        }
    }

    impl<const N: usize> St25r95Spi for WakeSequenceSpi<N> {
        fn poll(&mut self, _flags: PollFlags) -> Result<()> {
            Ok(())
        }

        fn reset(&mut self) -> Result<()> {
            Ok(())
        }

        fn send_command(&mut self, _cmd: Command, _data: &[u8], _sod: bool) -> Result<()> {
            Ok(())
        }

        fn read_data(&mut self) -> Result<ReadResponse> {
            let wake_source = self.wake_sources[self.index];
            self.index += 1;

            let mut data = heapless::Vec::new();
            data.push(wake_source).unwrap();
            Ok(ReadResponse { code: 0, data })
        }

        fn read_echo(&mut self, buf: &mut [u8; ECHO_RESPONSE_MAX_LEN]) -> Result<usize> {
            buf[0] = ECHO_BYTE;
            Ok(1)
        }

        fn flush(&mut self) -> Result<()> {
            Ok(())
        }
    }

    /// SPI mock that answers `Echo` at the wire level.
    ///
    /// `read_data` panics on purpose: the Echo paths must never route the
    /// unframed answer through the framed response parser.
    struct EchoSpi<const N: usize> {
        response: [u8; N],
        reported_len: Option<usize>,
    }

    impl<const N: usize> EchoSpi<N> {
        fn new(response: [u8; N]) -> Self {
            Self {
                response,
                reported_len: None,
            }
        }

        /// Answer `response` but claim a different number of bytes was read,
        /// as a misbehaving implementation would.
        fn reporting(response: [u8; N], reported_len: usize) -> Self {
            Self {
                response,
                reported_len: Some(reported_len),
            }
        }
    }

    impl<const N: usize> St25r95Spi for EchoSpi<N> {
        fn poll(&mut self, _flags: PollFlags) -> Result<()> {
            Ok(())
        }

        fn reset(&mut self) -> Result<()> {
            Ok(())
        }

        fn send_command(&mut self, _cmd: Command, _data: &[u8], _sod: bool) -> Result<()> {
            Ok(())
        }

        fn read_data(&mut self) -> Result<ReadResponse> {
            panic!("an Echo answer must not be read as a framed response")
        }

        fn read_echo(&mut self, buf: &mut [u8; ECHO_RESPONSE_MAX_LEN]) -> Result<usize> {
            let len = self.response.len().min(buf.len());
            buf[..len].copy_from_slice(&self.response[..len]);
            Ok(self.reported_len.unwrap_or(self.response.len()))
        }

        fn flush(&mut self) -> Result<()> {
            Ok(())
        }
    }

    struct NoopGpio;

    impl St25r95Gpio for NoopGpio {
        fn irq_in_pulse_low(&mut self) {}

        fn wait_irq_out_falling_edge(&mut self, _timeout: u32) -> core::result::Result<(), ()> {
            Ok(())
        }
    }

    /// GPIO mock with a virtual clock.
    ///
    /// `delays[n]` is how long, in milliseconds, IRQ_OUT takes to fall for the
    /// `n`th wait; that wait succeeds only if the driver was willing to wait at
    /// least that long. Every requested deadline is recorded.
    struct SequencedGpio<const N: usize> {
        delays: [u32; N],
        requested: heapless::Vec<u32, N>,
    }

    impl<const N: usize> SequencedGpio<N> {
        fn new(delays: [u32; N]) -> Self {
            Self {
                delays,
                requested: heapless::Vec::new(),
            }
        }
    }

    impl<const N: usize> St25r95Gpio for SequencedGpio<N> {
        fn irq_in_pulse_low(&mut self) {}

        fn wait_irq_out_falling_edge(&mut self, timeout: u32) -> core::result::Result<(), ()> {
            let delay = self.delays[self.requested.len()];
            self.requested.push(timeout).unwrap();
            if timeout >= delay {
                Ok(())
            } else {
                Err(())
            }
        }
    }

    fn reader<P>(protocol: P) -> St25r95<RecordingSpi, NoopGpio, FieldOn, Reader, P> {
        reader_with_spi(RecordingSpi::default(), protocol)
    }

    fn reader_with_spi<P>(
        spi: RecordingSpi,
        protocol: P,
    ) -> St25r95<RecordingSpi, NoopGpio, FieldOn, Reader, P> {
        St25r95 {
            spi,
            gpio: NoopGpio,
            dac_ref: None,
            dac_guard: 0,
            response_timeout: DEFAULT_RESPONSE_TIMEOUT_MS,
            pending_response: false,
            field: PhantomData::<FieldOn>,
            role: Reader,
            protocol,
        }
    }

    fn card_emulation<P>(
        protocol: P,
    ) -> St25r95<RecordingSpi, NoopGpio, FieldOn, CardEmulation, P> {
        card_emulation_with_spi(RecordingSpi::default(), CardEmulation(false), protocol)
    }

    fn card_emulation_with_spi<S, P>(
        spi: S,
        role: CardEmulation,
        protocol: P,
    ) -> St25r95<S, NoopGpio, FieldOn, CardEmulation, P> {
        St25r95 {
            spi,
            gpio: NoopGpio,
            dac_ref: None,
            dac_guard: 0,
            response_timeout: DEFAULT_RESPONSE_TIMEOUT_MS,
            pending_response: false,
            field: PhantomData::<FieldOn>,
            role,
            protocol,
        }
    }

    fn driver_with_gpio<S, G>(spi: S, gpio: G) -> St25r95<S, G, FieldOff, NoRole, NoProtocol> {
        St25r95 {
            spi,
            gpio,
            dac_ref: None,
            dac_guard: 0,
            response_timeout: DEFAULT_RESPONSE_TIMEOUT_MS,
            pending_response: false,
            field: PhantomData::<FieldOff>,
            role: NoRole,
            protocol: NoProtocol,
        }
    }

    fn reader_with_gpio<G, P>(
        gpio: G,
        protocol: P,
    ) -> St25r95<RecordingSpi, G, FieldOn, Reader, P> {
        St25r95 {
            spi: RecordingSpi::default(),
            gpio,
            dac_ref: None,
            dac_guard: 0,
            response_timeout: DEFAULT_RESPONSE_TIMEOUT_MS,
            pending_response: false,
            field: PhantomData::<FieldOn>,
            role: Reader,
            protocol,
        }
    }

    fn card_emulation_with_gpio<G, P>(
        gpio: G,
        role: CardEmulation,
        protocol: P,
    ) -> St25r95<RecordingSpi, G, FieldOn, CardEmulation, P> {
        St25r95 {
            spi: RecordingSpi::default(),
            gpio,
            dac_ref: None,
            dac_guard: 0,
            response_timeout: DEFAULT_RESPONSE_TIMEOUT_MS,
            pending_response: false,
            field: PhantomData::<FieldOn>,
            role,
            protocol,
        }
    }

    fn unselected_driver<S>(spi: S) -> St25r95<S, NoopGpio, FieldOff, NoRole, NoProtocol> {
        driver_with_gpio(spi, NoopGpio)
    }

    fn assert_wr_reg(spi: &RecordingSpi, data: &[u8]) {
        assert_eq!(spi.command, Some(Command::WrReg));
        assert_eq!(spi.data.as_slice(), data);
        assert!(!spi.sod);
    }

    fn assert_ac_filter(spi: &RecordingSpi, data: &[u8]) {
        assert_eq!(spi.command, Some(Command::ACFilter));
        assert_eq!(spi.data.as_slice(), data);
        assert!(!spi.sod);
    }

    fn assert_hw_error<T>(result: Result<T>, expected: St25r95Error) {
        match result {
            Err(Error::Hw(actual)) => assert_eq!(actual, expected),
            Err(error) => panic!("expected hardware error, got {error:?}"),
            Ok(_) => panic!("expected hardware error, got success"),
        }
    }

    fn timeout_wake() -> u8 {
        u8::from(WakeUpSource {
            lfo_freq: LFOFreq::KHz32,
            ss_low_pulse: false,
            irq_in_low_pulse: false,
            field_detection: false,
            tag_detection: false,
            timeout: true,
        })
    }

    fn tag_detection_wake() -> u8 {
        u8::from(WakeUpSource {
            lfo_freq: LFOFreq::KHz32,
            ss_low_pulse: false,
            irq_in_low_pulse: false,
            field_detection: false,
            tag_detection: true,
            timeout: false,
        })
    }

    fn calibration_reader<const N: usize>(
        wake_sources: [u8; N],
    ) -> St25r95<WakeSequenceSpi<N>, NoopGpio, FieldOn, Reader, NoProtocol> {
        St25r95 {
            spi: WakeSequenceSpi::new(wake_sources),
            gpio: NoopGpio,
            dac_ref: None,
            dac_guard: 0,
            response_timeout: DEFAULT_RESPONSE_TIMEOUT_MS,
            pending_response: false,
            field: PhantomData::<FieldOn>,
            role: Reader,
            protocol: NoProtocol,
        }
    }

    #[test]
    pub fn test_felica_default_arc_b() {
        let reader = St25r95 {
            spi: NoopSpi,
            gpio: NoopGpio,
            dac_ref: None,
            dac_guard: 0,
            response_timeout: DEFAULT_RESPONSE_TIMEOUT_MS,
            pending_response: false,
            field: PhantomData::<FieldOn>,
            role: Reader,
            protocol: FeliCa,
        };

        assert_eq!(
            reader.default_arc_b(),
            ArcB {
                modulation_index: ModulationIndex::Percent17,
                receiver_gain: ReceiverGain::Db34,
            }
        );
    }

    #[test]
    pub fn test_register_write_payloads() {
        let mut iso14443b = reader(Iso14443B);
        let arc_b = iso14443b.default_arc_b();
        iso14443b.write_arc_b(arc_b).unwrap();
        assert_wr_reg(&iso14443b.spi, &[0x68, 0x00, 0x01, 0x20]);

        let mut card = card_emulation(Iso14443A);
        let acc_a = card.default_acc_a();
        card.write_acc_a(acc_a).unwrap();
        assert_wr_reg(&card.spi, &[0x68, 0x00, 0x04, 0x27]);

        let mut iso14443a = reader(Iso14443A);
        let timer_window = iso14443a.default_timer_window();
        iso14443a.write_timer_windows(timer_window).unwrap();
        assert_wr_reg(&iso14443a.spi, &[0x3A, 0x00, 0x52]);

        let mut felica = reader(FeliCa);
        felica.enable_autodetect_filter().unwrap();
        assert_wr_reg(&felica.spi, &[0x0A, 0x00, 0x02]);
    }

    #[test]
    pub fn test_activate_ac_filter_one_uid_payload() {
        let mut card = card_emulation(Iso14443A);

        card.activate_ac_filter(0x1234, 0x56, [[0x01, 0x02, 0x03, 0x04]])
            .unwrap();

        assert_ac_filter(&card.spi, &[0x34, 0x12, 0x56, 0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    pub fn test_activate_ac_filter_two_uid_payload() {
        let mut card = card_emulation(Iso14443A);

        card.activate_ac_filter(
            0x1234,
            0x56,
            [[0x01, 0x02, 0x03, 0x04], [0x05, 0x06, 0x07, 0x08]],
        )
        .unwrap();

        assert_ac_filter(
            &card.spi,
            &[
                0x34, 0x12, 0x56, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            ],
        );
    }

    #[test]
    pub fn test_activate_ac_filter_three_uid_payload() {
        let mut card = card_emulation(Iso14443A);

        card.activate_ac_filter(
            0x1234,
            0x56,
            [
                [0x01, 0x02, 0x03, 0x04],
                [0x05, 0x06, 0x07, 0x08],
                [0x09, 0x0A, 0x0B, 0x0C],
            ],
        )
        .unwrap();

        assert_ac_filter(
            &card.spi,
            &[
                0x34, 0x12, 0x56, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
                0x0C,
            ],
        );
    }

    #[test]
    pub fn test_activate_ac_filter_invalid_uid_counts() {
        let mut card = card_emulation(Iso14443A);
        let no_uids: [UID; 0] = [];

        assert_eq!(
            card.activate_ac_filter(0x1234, 0x56, no_uids),
            Err(Error::InvalidCascadeLevelFilterCount(0))
        );

        let mut card = card_emulation(Iso14443A);
        assert_eq!(
            card.activate_ac_filter(
                0x1234,
                0x56,
                [
                    [0x01, 0x02, 0x03, 0x04],
                    [0x05, 0x06, 0x07, 0x08],
                    [0x09, 0x0A, 0x0B, 0x0C],
                    [0x0D, 0x0E, 0x0F, 0x10],
                ],
            ),
            Err(Error::InvalidCascadeLevelFilterCount(4))
        );
    }

    #[test]
    pub fn test_expect_data_len_rejects_hardware_status_codes() {
        for (status, expected) in [
            (0x63, St25r95Error::EmdSOFerror23),
            (0x8F, St25r95Error::NoField),
            (0x82, St25r95Error::InvalidCommandLength),
            (0x87, St25r95Error::FrameTimeoutOrNoTag),
            (0x84, St25r95Error::UnknownError(0x84)),
        ] {
            let response = ReadResponse::try_from([status, 0x00].as_slice()).unwrap();
            assert_eq!(response.expect_data_len(0), Err(Error::Hw(expected)));
        }
    }

    #[test]
    pub fn test_send_receive_keeps_raw_hardware_status() {
        let mut reader = reader_with_spi(RecordingSpi::with_response_code(0x87), Iso14443A);

        let response = reader.send_receive(&[0x26]).unwrap();

        assert_eq!(response.code, 0x87);
        assert!(response.data.is_empty());
    }

    #[test]
    pub fn test_send_receive_length_boundaries() {
        // An empty payload and the largest encodable payload are accepted.
        let mut nfc = reader(Iso14443A);
        assert!(nfc.send_receive(&[]).is_ok());
        assert!(nfc.spi.data.is_empty());
        assert!(nfc.send_receive(&[0xAA; MAX_COMMAND_DATA_LEN]).is_ok());
        assert_eq!(nfc.spi.data.len(), MAX_COMMAND_DATA_LEN);

        // One byte more cannot be announced by the command's one-byte length
        // field. The comparison is made on the whole slice length, so it holds
        // for any longer payload up to `usize::MAX`, and nothing is sent.
        for len in [MAX_COMMAND_DATA_LEN + 1, 528, MAX_BUFFER_SIZE] {
            let mut nfc = reader(Iso14443A);
            assert_eq!(
                nfc.send_receive(&[0xAA; MAX_BUFFER_SIZE][..len]),
                Err(Error::InvalidDataLen(len))
            );
            assert_eq!(nfc.spi.command, None);
        }
    }

    #[test]
    pub fn test_card_emulation_send_length_boundaries() {
        let mut card = card_emulation(Iso14443A);
        assert_eq!(card.send(&[]), Ok(()));
        assert!(card.spi.data.is_empty());
        assert_eq!(card.send(&[0xAA; MAX_COMMAND_DATA_LEN]), Ok(()));
        assert_eq!(card.spi.data.len(), MAX_COMMAND_DATA_LEN);

        for len in [MAX_COMMAND_DATA_LEN + 1, 528, MAX_BUFFER_SIZE] {
            let mut card = card_emulation(Iso14443A);
            assert_eq!(
                card.send(&[0xAA; MAX_BUFFER_SIZE][..len]),
                Err(Error::InvalidDataLen(len))
            );
            assert_eq!(card.spi.command, None);
        }
    }

    #[test]
    pub fn test_protocol_select_paths_reject_hardware_status() {
        assert_hw_error(
            unselected_driver(RecordingSpi::with_response_code(0x82))
                .protocol_select_iso15693(Default::default()),
            St25r95Error::InvalidCommandLength,
        );
        assert_hw_error(
            unselected_driver(RecordingSpi::with_response_code(0x82))
                .protocol_select_iso14443a(Default::default()),
            St25r95Error::InvalidCommandLength,
        );
        assert_hw_error(
            unselected_driver(RecordingSpi::with_response_code(0x82))
                .protocol_select_iso14443b(Default::default()),
            St25r95Error::InvalidCommandLength,
        );
        assert_hw_error(
            unselected_driver(RecordingSpi::with_response_code(0x82))
                .protocol_select_felica(Default::default()),
            St25r95Error::InvalidCommandLength,
        );
        assert_hw_error(
            unselected_driver(RecordingSpi::with_response_code(0x82))
                .protocol_select_ce_iso14443a(Default::default()),
            St25r95Error::InvalidCommandLength,
        );
    }

    #[test]
    pub fn test_acknowledgement_paths_reject_hardware_status() {
        assert_hw_error(
            reader_with_spi(RecordingSpi::with_response_code(0x82), Iso14443A).field_off(),
            St25r95Error::InvalidCommandLength,
        );

        let mut reader = reader_with_spi(RecordingSpi::with_response_code(0x82), Iso14443A);
        assert_hw_error(
            reader.write_timer_windows(TimerWindow(0x52)),
            St25r95Error::InvalidCommandLength,
        );

        let mut card = card_emulation_with_spi(
            RecordingSpi::with_response_code(0x82),
            CardEmulation(false),
            Iso14443A,
        );
        assert_hw_error(card.listen(), St25r95Error::InvalidCommandLength);
        assert!(!card.role.0);

        let mut card = card_emulation_with_spi(
            RecordingSpi::with_response_code(0x82),
            CardEmulation(false),
            Iso14443A,
        );
        assert_hw_error(card.send(&[0x01]), St25r95Error::InvalidCommandLength);

        let mut card = card_emulation_with_spi(
            RecordingSpi::with_response_code(0x82),
            CardEmulation(false),
            Iso14443A,
        );
        assert_hw_error(
            card.activate_ac_filter(0x1234, 0x56, [[0x01, 0x02, 0x03, 0x04]]),
            St25r95Error::InvalidCommandLength,
        );

        let mut card = card_emulation_with_spi(
            RecordingSpi::with_response_code(0x82),
            CardEmulation(false),
            Iso14443A,
        );
        assert_hw_error(
            card.set_anti_collision_state(AntiColState::Idle),
            St25r95Error::InvalidCommandLength,
        );
    }

    #[test]
    pub fn test_acknowledgement_paths_map_zero_length_hardware_errors() {
        for (status, expected) in [
            (0x8F, St25r95Error::NoField),
            (0x82, St25r95Error::InvalidCommandLength),
            (0x87, St25r95Error::FrameTimeoutOrNoTag),
            (0x84, St25r95Error::UnknownError(0x84)),
        ] {
            let mut card = card_emulation_with_spi(
                RecordingSpi::with_response_code(status),
                CardEmulation(false),
                Iso14443A,
            );
            assert_hw_error(card.listen(), expected);
            assert!(!card.role.0);
        }
    }

    #[test]
    pub fn test_echo_response_wire_forms() {
        // Ordinary Echo.
        assert_eq!(
            EchoResponse::try_from([ECHO_BYTE].as_slice()),
            Ok(EchoResponse::Echo)
        );

        // Echo that leaves Listen mode, followed by its framed UserStop.
        match EchoResponse::try_from([ECHO_BYTE, 0x85, 0x00].as_slice()) {
            Ok(EchoResponse::ListenCancelled(response)) => {
                assert_eq!(response.code, 0x85);
                assert!(response.data.is_empty());
                assert_eq!(
                    response.expect_data_len(0),
                    Err(Error::Hw(St25r95Error::UserStop))
                );
            }
            other => panic!("expected a cancelled listen, got {other:?}"),
        }

        // Short reads.
        assert_eq!(
            EchoResponse::try_from([].as_slice()),
            Err(Error::EchoFailed)
        );
        assert_eq!(
            EchoResponse::try_from([ECHO_BYTE, 0x85].as_slice()),
            Err(Error::InvalidDataLen(1))
        );

        // Extra bytes trailing an otherwise complete answer.
        assert_eq!(
            EchoResponse::try_from([ECHO_BYTE, 0x85, 0x00, 0x00].as_slice()),
            Err(Error::InvalidDataLen(3))
        );

        // A late response left over from a previous command is not an Echo.
        assert_eq!(
            EchoResponse::try_from([0x80, 0x02, 0x26].as_slice()),
            Err(Error::EchoFailed)
        );
        for byte in [0x00, 0x54, 0x56, 0xFF] {
            assert_eq!(
                EchoResponse::try_from([byte].as_slice()),
                Err(Error::EchoFailed)
            );
        }
    }

    #[test]
    pub fn test_echo_requires_an_exact_echo_byte() {
        let mut nfc = unselected_driver(EchoSpi::new([ECHO_BYTE]));
        assert_eq!(nfc.echo(), Ok(()));

        let mut nfc = unselected_driver(EchoSpi::new([0x00]));
        assert_eq!(nfc.echo(), Err(Error::EchoFailed));

        let mut nfc = unselected_driver(EchoSpi::new([]));
        assert_eq!(nfc.echo(), Err(Error::EchoFailed));

        // Echoing while the chip is listening surfaces the framed status
        // rather than reporting a successful Echo.
        let mut nfc = unselected_driver(EchoSpi::new([ECHO_BYTE, 0x85, 0x00]));
        assert_hw_error(nfc.echo(), St25r95Error::UserStop);

        // An implementation claiming more bytes than it wrote is refused.
        let mut nfc = unselected_driver(EchoSpi::reporting([ECHO_BYTE], ECHO_RESPONSE_MAX_LEN + 1));
        assert_eq!(nfc.echo(), Err(Error::EchoFailed));
    }

    #[test]
    pub fn test_cancel_listen_consumes_echo_byte_and_user_stop_frame() {
        // The documented cancellation answer: Echo byte, then the UserStop
        // frame. Both are consumed by the single Echo read.
        let mut card = card_emulation_with_spi(
            EchoSpi::new([ECHO_BYTE, 0x85, 0x00]),
            CardEmulation(true),
            Iso14443A,
        );
        assert_eq!(card.cancel_listen(), Ok(()));
        assert!(!card.role.0);

        // An external reader command had already ended Listen mode, so only the
        // Echo byte comes back.
        let mut card =
            card_emulation_with_spi(EchoSpi::new([ECHO_BYTE]), CardEmulation(true), Iso14443A);
        assert_eq!(card.cancel_listen(), Ok(()));
        assert!(!card.role.0);

        // Any other framed status is reported and leaves the listen state.
        let mut card = card_emulation_with_spi(
            EchoSpi::new([ECHO_BYTE, 0x82, 0x00]),
            CardEmulation(true),
            Iso14443A,
        );
        assert_hw_error(card.cancel_listen(), St25r95Error::InvalidCommandLength);
        assert!(card.role.0);

        // A truncated answer is not a cancellation.
        let mut card = card_emulation_with_spi(
            EchoSpi::new([ECHO_BYTE, 0x85]),
            CardEmulation(true),
            Iso14443A,
        );
        assert_eq!(card.cancel_listen(), Err(Error::InvalidDataLen(1)));
        assert!(card.role.0);
    }

    #[test]
    pub fn test_calibrate_tag_detector_all_timeout_fails_without_dac_ref() {
        let mut reader = calibration_reader([timeout_wake()]);

        assert_eq!(
            reader.calibrate_tag_detector(),
            Err(Error::CalibTagDetectionFailed)
        );
        assert_eq!(reader.dac_ref, None);
    }

    #[test]
    pub fn test_calibrate_tag_detector_all_tag_detection_fails_without_dac_ref() {
        let mut reader = calibration_reader([tag_detection_wake(), tag_detection_wake()]);

        assert_eq!(
            reader.calibrate_tag_detector(),
            Err(Error::CalibTimeoutFailed)
        );
        assert_eq!(reader.dac_ref, None);
    }

    #[test]
    pub fn test_calibrate_tag_detector_boundary_timeout_fails_without_updating_dac_ref() {
        let mut reader = calibration_reader([
            tag_detection_wake(),
            timeout_wake(),
            timeout_wake(),
            timeout_wake(),
            timeout_wake(),
            timeout_wake(),
            timeout_wake(),
            timeout_wake(),
        ]);
        reader.dac_ref = Some(0x55);

        assert_eq!(
            reader.calibrate_tag_detector(),
            Err(Error::CalibDacOutOfRange {
                dac_high: 0x00,
                delta: -0x04,
            })
        );
        assert_eq!(reader.dac_ref, Some(0x55));
    }

    #[test]
    pub fn test_calibrate_tag_detector_boundary_zero_succeeds() {
        let mut reader = calibration_reader([
            tag_detection_wake(),
            timeout_wake(),
            timeout_wake(),
            timeout_wake(),
            timeout_wake(),
            timeout_wake(),
            timeout_wake(),
            tag_detection_wake(),
        ]);

        assert_eq!(reader.calibrate_tag_detector(), Ok(0x00));
        assert_eq!(reader.dac_ref, Some(0x00));
    }

    #[test]
    pub fn test_host_timeout_ms_derivation() {
        // No configured device timeout: the default deadline applies.
        assert_eq!(host_timeout_ms(None), DEFAULT_RESPONSE_TIMEOUT_MS);
        assert_eq!(host_timeout_ms(Some(0.0)), DEFAULT_RESPONSE_TIMEOUT_MS);
        assert_eq!(host_timeout_ms(Some(10_000.0)), DEFAULT_RESPONSE_TIMEOUT_MS);

        // Just below, at, and above the default 100 ms deadline the host waits
        // for the device timeout plus its margin.
        assert_eq!(host_timeout_ms(Some(99_000.0)), 150);
        assert_eq!(host_timeout_ms(Some(100_000.0)), 151);
        assert_eq!(host_timeout_ms(Some(101_000.0)), 152);

        // Multi-second frame waits are honoured, and the arithmetic saturates.
        assert_eq!(host_timeout_ms(Some(3_000_000.0)), 3_051);
        assert_eq!(host_timeout_ms(Some(f32::MAX)), u32::MAX);
    }

    #[test]
    pub fn test_protocol_select_derives_the_host_deadline() {
        // A Type B FWT of about 154 ms, well above the default host wait.
        let fwt = iso14443b::reader::FWT::new(9, 0, 0).unwrap();
        assert!(fwt.us() > 100_000.0);
        let expected = host_timeout_ms(Some(fwt.us()));

        let nfc = driver_with_gpio(RecordingSpi::default(), SequencedGpio::new([0, 200]));
        let mut reader = nfc
            .protocol_select_iso14443b(iso14443b::reader::Parameters::default().fwt(fwt))
            .unwrap();

        assert_eq!(reader.response_timeout(), expected);
        // The tag answers after 200 ms: a fixed 100 ms host wait would have
        // abandoned a frame the chip was still serving.
        assert!(reader.send_receive(&[0x05]).is_ok());
        assert_eq!(
            reader.gpio.requested.as_slice(),
            &[DEFAULT_RESPONSE_TIMEOUT_MS, expected]
        );
    }

    #[test]
    pub fn test_multi_second_frame_wait_is_honoured() {
        // FeliCa RWT at the largest allowed exponent: about 4.9 s.
        let rwt = felica::reader::RWT::new(MAX_PP, 0).unwrap();
        let expected = host_timeout_ms(Some(rwt.us()));
        assert_eq!(expected, 5_000);

        let nfc = driver_with_gpio(RecordingSpi::default(), SequencedGpio::new([0, 4_900]));
        let mut reader = nfc
            .protocol_select_felica(felica::reader::Parameters::default().rwt(rwt))
            .unwrap();

        assert_eq!(reader.response_timeout(), expected);
        assert!(reader.send_receive(&[0x00]).is_ok());
    }

    #[test]
    pub fn test_default_deadline_boundaries() {
        // Just below, at, and above the default deadline.
        for (delay, answered) in [(99, true), (100, true), (101, false)] {
            let mut reader = reader_with_gpio(SequencedGpio::new([delay]), Iso14443A);

            assert_eq!(reader.send_receive(&[0x26]).is_ok(), answered);
            assert_eq!(
                reader.gpio.requested.as_slice(),
                &[DEFAULT_RESPONSE_TIMEOUT_MS]
            );
        }
    }

    #[test]
    pub fn test_host_timeout_keeps_the_operation_and_blocks_new_commands() {
        // The chip answers after 3 s while the host waits the default 100 ms.
        let mut reader = reader_with_gpio(SequencedGpio::new([3_000, 0, 0]), Iso14443A);

        assert_eq!(reader.send_receive(&[0x26]), Err(Error::ResponseInProgress));
        assert!(reader.is_response_pending());

        // A second command would race the answer to the first, so it never
        // reaches the chip and cannot consume the late response.
        assert_eq!(reader.send_receive(&[0x26]), Err(Error::ResponseInProgress));
        assert_eq!(reader.gpio.requested.len(), 1);

        // Waiting longer completes the operation that was already in flight.
        let response = reader.poll_pending_response(5_000).unwrap();
        assert_eq!(response.code, 0);
        assert!(!reader.is_response_pending());
        assert!(reader.send_receive(&[0x26]).is_ok());
    }

    #[test]
    pub fn test_discard_pending_response_unblocks_the_driver() {
        let mut reader = reader_with_gpio(SequencedGpio::new([3_000, 0]), Iso14443A);

        assert_eq!(reader.send_receive(&[0x26]), Err(Error::ResponseInProgress));
        assert!(reader.is_response_pending());

        assert_eq!(reader.discard_pending_response(), Ok(()));
        assert!(!reader.is_response_pending());
        assert!(reader.send_receive(&[0x26]).is_ok());
    }

    #[test]
    pub fn test_idle_uses_the_caller_supplied_deadline() {
        // A deadline shorter than the configured sleep leaves the chip asleep.
        let mut nfc = driver_with_gpio(
            WakeSequenceSpi::new([timeout_wake()]),
            SequencedGpio::new([3_000]),
        );

        assert_eq!(
            nfc.idle(IdleParams::default(), DEFAULT_RESPONSE_TIMEOUT_MS),
            Err(Error::ResponseInProgress)
        );
        assert!(nfc.is_response_pending());

        // The wake-up answer is still collectable once it arrives.
        assert_eq!(nfc.ack_idle().map(|wus| wus.timeout), Ok(true));
        assert!(!nfc.is_response_pending());
        assert_eq!(
            nfc.gpio.requested.as_slice(),
            &[DEFAULT_RESPONSE_TIMEOUT_MS]
        );

        // A deadline that covers the sleep returns the wake-up source directly.
        let mut nfc = driver_with_gpio(
            WakeSequenceSpi::new([timeout_wake()]),
            SequencedGpio::new([3_000]),
        );

        assert_eq!(
            nfc.idle(IdleParams::default(), 5_000)
                .map(|wus| wus.timeout),
            Ok(true)
        );
        assert!(!nfc.is_response_pending());
    }

    #[test]
    pub fn test_receive_timeout_leaves_the_card_able_to_cancel_listen() {
        // No reader command arrives within the deadline.
        let mut card = card_emulation_with_gpio(
            SequencedGpio::new([3_000, 0]),
            CardEmulation(true),
            Iso14443A,
        );

        assert_eq!(
            card.receive(DEFAULT_RESPONSE_TIMEOUT_MS),
            Err(Error::ResponseInProgress)
        );

        // Nothing was sent, so the card can still leave Listen mode.
        assert!(!card.is_response_pending());
        assert_eq!(card.cancel_listen(), Ok(()));
        assert!(!card.role.0);
    }

    #[test]
    pub fn test_len_decode() {
        // See datasheet Table 8.
        check_range(0x80, 0x00..0xff, 0..255);
        check_range(0xA0, 0x00..0xff, 256..511);
        check_range(0xC0, 0x00..0x10, 512..528);
        check_range(0x90, 0x00..0xff, 0..255);
        check_range(0xB0, 0x00..0xff, 256..511);
        check_range(0xD0, 0x00..0x10, 512..528);
        assert_eq!(ReadResponse::data_len([0, 0]), 0);
        assert_eq!(ReadResponse::data_len([0x90, 8]), 8);
        assert_eq!(ReadResponse::data_len([0x87, 9]), 9);
    }

    fn check_range(code: u8, len_range: Range<u8>, res_range: Range<usize>) {
        for len in len_range {
            let res = ReadResponse::data_len([code, len]);
            assert!(res_range.contains(&res))
        }
    }

    #[test]
    pub fn test_from_slice() {
        assert_eq!(
            ReadResponse::try_from([0, 0].as_slice()),
            Ok(ReadResponse {
                code: 0,
                data: heapless::Vec::<u8, MAX_BUFFER_SIZE>::new()
            })
        );
    }
}
