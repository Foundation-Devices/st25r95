// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! ST25R95 Register Interface Module
//!
//! This module provides access to the ST25R95's internal registers for fine-tuned
//! configuration and status monitoring. The ST25R95 contains numerous registers
//! that control RF parameters, timing, power management, and protocol-specific
//! settings.
//!
//! ## Register Categories
//!
//! ### RF Control Registers
//! - **ARC_B**: Analog RF Control B - Modulation index and receiver gain
//! - **ACC_A**: Analog Control A - Load modulation and demodulator sensitivity
//!
//! ### Timing and Control Registers  
//! - **Timer Window**: ISO14443A timing optimization
//! - **Auto-Detect Filter**: FeliCa synchronization filter
//! - **Wakeup**: Wake-up source status monitoring
//!
//! ### Register Access Pattern
//!
//! The ST25R95 uses a specific register access protocol:
//!
//! 1. **Direct access**: Most registers can be read/written directly
//! 2. **Indexed access**: Some registers require setting an index first
//! 3. **Command-based**: Registers accessed via RdReg/WrReg commands
//!
//! ## Register Programming Guidelines
/// ### Protocol-Specific Optimization
///
/// Each NFC protocol has optimal register settings:
///
/// **ISO15693**:
/// ```rust
/// # use st25r95::{
/// #     iso15693,
/// #     mock::{MockGpio, MockSpi},
/// #     ModulationIndex,
/// #     ReceiverGain,
/// #     St25r95,
/// # };
/// # let nfc = St25r95::new(MockSpi::default(), MockGpio)?;
/// # let params = iso15693::reader::Parameters::default()
/// #     .modulation(iso15693::reader::Modulation::Percent10);
/// # let mut reader = nfc.protocol_select_iso15693(params)?;
/// // 33% modulation index, 27dB gain, for 10% modulation
/// let arc_b = reader.new_arc_b(ModulationIndex::Percent33, ReceiverGain::Db27)?;
/// reader.write_arc_b(arc_b)?;
/// # Ok::<(), st25r95::Error>(())
/// ```
///
/// **ISO14443A**:
/// ```rust
/// # use st25r95::{
/// #     mock::{MockGpio, MockSpi},
/// #     ModulationIndex,
/// #     ReceiverGain,
/// #     St25r95,
/// # };
/// # let nfc = St25r95::new(MockSpi::default(), MockGpio)?;
/// # let mut reader = nfc.protocol_select_iso14443a(Default::default())?;
/// // 95% modulation, 8dB gain, optimized timing
/// let arc_b = reader.new_arc_b(ModulationIndex::Percent95, ReceiverGain::Db8)?;
/// let timer_w = reader.recommended_timer_window();
/// reader.write_arc_b(arc_b)?;
/// reader.write_timer_windows(timer_w)?;
/// # Ok::<(), st25r95::Error>(())
/// ```
///
/// **Card Emulation**:
/// ```rust
/// # use st25r95::{
/// #     acc_a::{DemodulatorSensitivity, LoadModulationIndex},
/// #     mock::{MockGpio, MockSpi},
/// #     St25r95,
/// # };
/// # let nfc = St25r95::new(MockSpi::default(), MockGpio)?;
/// # let mut card = nfc.protocol_select_ce_iso14443a(Default::default())?;
/// // Optimize load modulation for card emulation
/// let acc_a = card.new_acc_a(
///     LoadModulationIndex::max(),
///     DemodulatorSensitivity::Percent100,
/// )?;
/// card.write_acc_a(acc_a)?;
/// # Ok::<(), st25r95::Error>(())
/// ```
///
/// ### Safety Considerations
///
/// - Always validate parameter ranges before writing
/// - Some registers have protocol-specific valid values
/// - Register writes take effect immediately
/// - Save original values before experimentation
///
/// ## Usage Examples
///
/// ```rust
/// # use st25r95::{
/// #     mock::{MockGpio, MockSpi},
/// #     St25r95,
/// # };
/// # let nfc = St25r95::new(MockSpi::default(), MockGpio)?;
/// # let mut reader = nfc.protocol_select_iso14443a(Default::default())?;
/// // Read current register state
/// let arc_b = reader.read_arc_b()?;
/// let (modulation, gain) = (arc_b.modulation_index, arc_b.receiver_gain);
///
/// // Write optimized configuration
/// let new_arc_b = reader.default_arc_b();
/// reader.write_arc_b(new_arc_b)?;
///
/// // Fine-tune timing for the protocol in use
/// let timer_w = reader.recommended_timer_window();
/// reader.write_timer_windows(timer_w)?;
/// # let _ = (modulation, gain);
/// # Ok::<(), st25r95::Error>(())
/// ```
pub mod acc_a;
pub mod arc_b;
pub mod auto_detect_filter;
pub mod timer_window;
pub mod wakeup;

/// Trait for ST25R95 register access
///
/// This trait defines the interface for reading and writing ST25R95 internal
/// registers. It provides methods for register addressing, indexed access,
/// and value manipulation required by the SPI communication protocol.
///
/// ## Register Access Protocol
///
/// The ST25R95 supports two register access methods:
///
/// 1. **Direct Access**: Most registers can be accessed directly using their read/write
///    addresses without additional setup.
///
/// 2. **Indexed Access**: Some registers (typically with multiple instances) require
///    setting an index first, then accessing the register.
///
/// ## Implementation Notes
///
/// This trait is implemented by individual register types and used internally
/// by the driver for register read/write operations. The methods return
/// the raw values required by the SPI protocol.
///
/// ## Method Descriptions
///
/// - `read_addr()`: Register address for read operations
/// - `write_addr()`: Register address for write operations
/// - `index_confirmation()`: Index value for indexed register access
/// - `has_index()`: Whether this register requires indexed access
/// - `value()`: Current register value to be written
pub(crate) trait Register {
    /// Get the read address for this register
    ///
    /// Returns the register address used when reading the register value
    /// via the RdReg command.
    fn read_addr(&self) -> u8;

    /// Get the write address for this register
    ///
    /// Returns the register address used when writing the register value
    /// via the WrReg command.
    fn write_addr(&self) -> u8;

    /// Get the index confirmation value
    ///
    /// For indexed registers, this returns the index value that was set
    /// prior to accessing the register. For non-indexed registers, this
    /// is typically the same as the register value.
    fn index_confirmation(&self) -> u8;

    /// Check if this register requires indexed access
    ///
    /// Returns `true` if the register must be accessed via indexed mode
    /// (set index first, then read/write), `false` for direct access.
    fn has_index(&self) -> bool;

    /// Get the current register value
    ///
    /// Returns the 8-bit value to be written to the register. For
    /// configuration registers, this represents the desired settings.
    fn value(&self) -> u8;
}
