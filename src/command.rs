// SPDX-FileCopyrightText: 2024 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! ST25R95 Command System
//!
//! This module defines the complete command set for the ST25R95 NFC transceiver.
//! Each command corresponds to a specific operation that can be performed by the
//! chip, from basic identification to complex protocol operations.
//!
//! ## Command Categories
//!
//! ### Configuration Commands
//! - **ProtocolSelect**: Select RF protocol and configure parameters
//! - **RdReg/WrReg**: Read/write internal registers for fine-tuning
//! - **Idle**: Enter low-power mode with configurable wake-up sources
//!
//! ### Communication Commands  
//! - **SendRecv**: Send command to tag and receive response (reader mode)
//! - **Send**: Send response to reader (card emulation mode)
//! - **Listen**: Enter listening mode for card emulation
//!
//! ### Diagnostic Commands
//! - **Idn**: Read chip identification and version information
//! - **Echo**: Test communication link with the chip
//! - **PollField**: Detect presence of external RF fields
//!
//! ### Special Commands
//! - **ACFilter**: Configure anti-collision filtering for card emulation
//!
//! ## Command Flow
//!
//! Commands follow a specific sequence:
//!
//! 1. **Send command** via SPI with parameters
//! 2. **Wait for IRQ_OUT** indicating completion
//! 3. **Read response** containing status and optional data
//!
//! All commands are sent through the SPI interface using the `St25r95Spi` trait.
//! The numeric values correspond to the command bytes sent to the ST25R95.

use crate::Error;

/// ST25R95 Command Enumeration
///
/// Represents all supported commands for the ST25R95 NFC transceiver.
/// Each variant corresponds to a specific operation and has an associated
/// command byte value used in SPI communication.
///
/// See ST25R95 datasheet Section 4.4 for complete command specifications.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Command {
    /// Read device identification
    ///
    /// Returns chip identification string including:
    /// - Product name ("NFC" prefix)
    /// - Device type and version
    /// - ROM CRC for verification
    ///
    /// Used for chip detection and version checking during initialization.
    Idn = 0x01,

    /// Select RF communication protocol
    ///
    /// Configures the ST25R95 for a specific NFC protocol with associated
    /// parameters. This command must be called before any RF communication.
    ///
    /// Parameters vary by protocol and include:
    /// - Data rate configuration
    /// - Timing parameters (FDT)
    /// - Modulation settings
    /// - Protocol-specific options
    ProtocolSelect = 0x02,

    /// Poll for RF field presence
    ///
    /// Detects whether an external RF field is active. Can also wait for
    /// field appearance/disappearance with configurable timeout.
    ///
    /// Useful for:
    /// - Detecting nearby readers
    /// - Multi-reader collision avoidance
    /// - Power-saving field detection
    PollField = 0x03,

    /// Send command to tag and receive response
    ///
    /// Primary communication command for reader mode. Sends data to an
    /// NFC tag and automatically receives the tag's response.
    ///
    /// Features:
    /// - Automatic CRC generation/checking
    /// - Protocol-specific framing
    /// - Configurable timeouts
    /// - Automatic retransmission if needed
    SendRecv = 0x04,

    /// Enter listening mode (card emulation)
    ///
    /// Places the ST25R95 in card emulation mode where it waits for
    /// commands from external NFC readers. The chip remains in this
    /// state until a command is received or the mode is cancelled.
    ///
    /// Used for:
    /// - Payment card emulation
    /// - Access token simulation
    /// - Peer-to-peer communication
    Listen = 0x05,

    /// Send data to external reader (card emulation)
    ///
    /// Sends response data to an external NFC reader while in card
    /// emulation mode. This command is used after receiving a reader
    /// command via the `Listen` mode.
    ///
    /// Uses load modulation to respond to reader queries.
    Send = 0x06,

    /// Enter low-power idle mode
    ///
    /// Places the ST25R95 in a low-power state with configurable
    /// wake-up sources. This is the primary power-saving command.
    ///
    /// Wake-up sources include:
    /// - IRQ_IN pulse
    /// - External RF field detection
    /// - Tag detection (with calibration)
    /// - Timeout expiration
    /// - SPI communication attempt
    Idle = 0x07,

    /// Read internal register
    ///
    /// Reads a single byte from an internal ST25R95 register.
    /// Used for status checking and configuration verification.
    ///
    /// Supports both direct and indexed register access.
    RdReg = 0x08,

    /// Write internal register
    ///
    /// Writes a byte to an internal ST25R95 register.
    /// Used for configuration and runtime parameter adjustment.
    ///
    /// Supports both direct and indexed register access.
    WrReg = 0x09,

    /// Anti-collision filter command (Type A card emulation)
    ///
    /// Configures and manages the anti-collision filter for ISO14443-A
    /// card emulation. This allows selective card presence emulation.
    ///
    /// Functions:
    /// - Activate/deactivate filter
    /// - Set filter parameters (ATQA, SAK, UID)
    /// - Read filter state
    ACFilter = 0x0D,

    /// Echo test command
    ///
    /// Simple diagnostic command that tests the SPI communication
    /// link with the ST25R95. The chip responds with a simple
    /// acknowledgement when the command is received.
    ///
    /// Used for:
    /// - Communication verification
    /// - SPI interface testing
    /// - Basic connectivity checks
    Echo = 0x55,
}

impl TryFrom<u8> for Command {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::Idn),
            0x02 => Ok(Self::ProtocolSelect),
            0x03 => Ok(Self::PollField),
            0x04 => Ok(Self::SendRecv),
            0x05 => Ok(Self::Listen),
            0x06 => Ok(Self::Send),
            0x07 => Ok(Self::Idle),
            0x08 => Ok(Self::RdReg),
            0x09 => Ok(Self::WrReg),
            // 0x0B => Ok(Self::SubFreqRes),
            0x0D => Ok(Self::ACFilter),
            0x55 => Ok(Self::Echo),
            c => Err(c),
        }
    }
}

/// Field detection timing parameters for PollField command
///
/// This structure configures the timing for field detection operations
/// when using the PollField command with a timeout. It allows waiting
/// for RF field appearance or disappearance with precise timing control.
///
/// ## Parameters
///
/// - **apparance**: `true` to wait for field appearance, `false` for disappearance
/// - **presc**: Prescaler value for timing calculation (0-255)
/// - **timer**: Timer value for timing calculation (0-255)
///
/// ## Timing Formula
///
/// The timeout is calculated as:
/// ```text
/// timeout_us = ((presc + 1) * (timer + 1)) / 13.56
/// ```
///
/// This provides a range from approximately 74μs to 3.6 seconds.
///
/// ## Usage Examples
///
/// ```rust
/// # use st25r95::{
/// #     mock::{MockGpio, MockSpi},
/// #     St25r95,
/// #     WaitForField,
/// # };
/// # let mut nfc = St25r95::new(MockSpi::default(), MockGpio)?;
/// // Wait for field appearance
/// let wff = WaitForField {
///     apparance: true,
///     presc: 15, // (15 + 1) * (13 + 1) / 13.56
///     timer: 13,
/// };
/// nfc.poll_field(Some(wff))?;
///
/// // Wait longer for field disappearance
/// let wff = WaitForField {
///     apparance: false,
///     presc: 200, // (200 + 1) * (67 + 1) / 13.56
///     timer: 67,
/// };
/// nfc.poll_field(Some(wff))?;
/// # Ok::<(), st25r95::Error>(())
/// ```
#[derive(Debug, Copy, Clone, Default)]
pub struct WaitForField {
    /// Wait for field appearance (true) or disappearance (false)
    pub apparance: bool,
    /// Prescaler value for timing calculation
    pub presc: u8,
    /// Timer value for timing calculation
    pub timer: u8,
}

impl WaitForField {
    /// Calculate the timeout value in microseconds
    ///
    /// Returns the configured timeout duration in microseconds based on
    /// the prescaler and timer values.
    ///
    /// ## Formula
    /// ```text
    /// timeout_us = ((presc + 1) × (timer + 1)) / 13.56
    /// ```
    ///
    /// ## Examples
    /// ```rust
    /// # use st25r95::WaitForField;
    /// let wff = WaitForField {
    ///     apparance: true,
    ///     presc: 15,
    ///     timer: 13,
    /// };
    /// assert!((wff.us() - 16.52).abs() < 0.01);
    /// ```
    pub fn us(self) -> f32 {
        (((self.presc) as f32 + 1f32) * ((self.timer as f32) + 1f32)) / 13.56f32
    }
}

/// Low-Frequency Oscillator (LFO) frequency selection
///
/// The LFO is used during low-power idle mode to generate periodic
/// wake-up intervals. The frequency determines how often the ST25R95
/// checks for wake-up conditions when in sleep mode.
///
/// ## Frequency Selection Trade-offs
///
/// - **Higher frequency** (32 kHz): Faster wake-up response, higher power consumption
/// - **Lower frequency** (4 kHz): Lower power consumption, slower wake-up response
///
/// ## Usage Context
///
/// This setting affects:
/// - Wake-up timing precision
/// - Field detection responsiveness
/// - Tag detection calibration intervals
/// - Overall power consumption in idle mode
///
/// ## Typical Applications
///
/// - **32 kHz**: Applications requiring fast response (payment systems)
/// - **16 kHz**: Balanced performance (access control)
/// - **8 kHz**: Power-conscious applications (inventory tracking)
/// - **4 kHz**: Maximum power saving (remote sensors)
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub enum LFOFreq {
    /// 32 kHz LFO frequency
    ///
    /// Provides fastest wake-up response but highest power consumption.
    /// Suitable for applications requiring quick response times.
    #[default]
    KHz32 = 0b00,

    /// 16 kHz LFO frequency
    ///
    /// Balanced performance with moderate power consumption.
    /// Good general-purpose choice for most applications.
    KHz16 = 0b01,

    /// 8 kHz LFO frequency
    ///
    /// Lower power consumption with acceptable response time.
    /// Suitable for battery-powered applications.
    KHz8 = 0b10,

    /// 4 kHz LFO frequency
    ///
    /// Lowest power consumption but slowest wake-up response.
    /// Ideal for ultra-low-power applications.
    KHz4 = 0b11,
}

impl LFOFreq {
    /// Calculate the LFO period in microseconds
    ///
    /// Returns the period of one LFO cycle in microseconds.
    ///
    /// ## Formula
    /// ```text
    /// period_us = 1 / frequency_kHz * 1000
    /// ```
    ///
    /// ## Examples
    /// ```rust
    /// # use st25r95::LFOFreq;
    /// assert_eq!(LFOFreq::KHz32.period_us(), 31.25);
    /// assert_eq!(LFOFreq::KHz4.period_us(), 250.0);
    /// ```
    pub fn period_us(self) -> f32 {
        match self {
            LFOFreq::KHz32 => 31.25,
            LFOFreq::KHz16 => 62.5,
            LFOFreq::KHz8 => 125.0,
            LFOFreq::KHz4 => 250.0,
        }
    }

    /// Calculate the reference period in milliseconds
    ///
    /// Returns the reference period (tREF) which equals 256 LFO cycles.
    /// This value is used internally by the ST25R95 for timing calculations.
    ///
    /// ## Formula
    /// ```text
    /// t_ref_ms = 256 × period_us / 1000
    /// ```
    ///
    /// ## Examples
    /// ```rust
    /// # use st25r95::LFOFreq;
    /// assert_eq!(LFOFreq::KHz32.t_ref_ms(), 8);
    /// assert_eq!(LFOFreq::KHz4.t_ref_ms(), 64);
    /// ```
    pub fn t_ref_ms(self) -> u8 {
        match self {
            LFOFreq::KHz32 => 8,
            LFOFreq::KHz16 => 16,
            LFOFreq::KHz8 => 32,
            LFOFreq::KHz4 => 64,
        }
    }
}

impl TryFrom<u8> for LFOFreq {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0b00 => Ok(LFOFreq::KHz32),
            0b01 => Ok(LFOFreq::KHz16),
            0b10 => Ok(LFOFreq::KHz8),
            0b11 => Ok(LFOFreq::KHz4),
            _ => Err(()),
        }
    }
}

/// Wake-up source configuration for idle mode
///
/// This structure defines which events can wake the ST25R95 from low-power
/// idle mode. Multiple wake-up sources can be enabled simultaneously,
/// providing flexible power management strategies.
///
/// ## Wake-up Source Types
///
/// ### Hardware Wake-up
/// - **IRQ_IN pulse**: Host-controlled wake-up via GPIO pulse
/// - **Field detection**: External RF field detected
/// - **Tag detection**: Tag enters the RF field (requires calibration)
///
/// ### Timed Wake-up
/// - **Timeout**: Automatic wake-up after specified period
///
/// ## Calibration Requirements
///
/// **Tag detection requires calibration** using `calibrate_tag_detector()`:
///
/// ```rust
/// # use st25r95::IdleParams;
/// // Calibrate first with `St25r95::calibrate_tag_detector`; the constructor
/// // enables the resources a detection trial needs. Adding the timeout makes
/// // the chip report back even when no tag ever arrives.
/// let mut params = IdleParams::tag_detection(0x20, 0x08)?;
/// params.wus.timeout = true;
/// assert!(params.wus.tag_detection && params.wus.timeout);
/// # params.validate()?;
/// # Ok::<(), st25r95::Error>(())
/// ```
///
/// ## Power Consumption Impact
///
/// Each enabled wake-up source affects power consumption:
/// - **IRQ_IN only**: Lowest power (host-controlled)
/// - **Field detection**: Moderate power (continuous monitoring)
/// - **Tag detection**: Higher power (periodic field generation)
/// - **Timeout**: Low power (internal timer only)
///
/// ## Recommended Configurations
///
/// A wake-up source is only half of a working Idle: the chip also has to be
/// told which circuits to keep powered while it sleeps, and the two have to
/// agree. Build the pair with the [`IdleParams`] constructors rather than by
/// hand - a field-detection source without its EnterCtrl resources, for
/// instance, is the combination that leaves the chip needing a power cycle.
///
/// ```rust
/// # use st25r95::IdleParams;
/// // Host-controlled wake-up (lowest power)
/// let host = IdleParams::hibernate();
///
/// // Field detection for reader collision avoidance
/// let field = IdleParams::field_detection();
///
/// // Tag detection with timeout failsafe
/// let tag = IdleParams::tag_detection(0x20, 0x08)?;
///
/// assert!(host.wus.irq_in_low_pulse);
/// assert!(field.wus.field_detection && field.enter_ctrl.field_detect_aux_enabled);
/// assert!(tag.wus.tag_detection && tag.enter_ctrl.lfo_enabled);
/// # Ok::<(), st25r95::Error>(())
/// ```
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct WakeUpSource {
    /// Low-Frequency Oscillator frequency setting
    ///
    /// Determines the timing resolution for periodic wake-up checks
    /// and affects overall power consumption. Higher frequencies provide
    /// faster response but consume more power.
    pub lfo_freq: LFOFreq,

    /// Special sleep mode wake-up
    ///
    /// Reserved for special sleep mode operations. Typically left disabled
    /// in normal applications.
    pub ss_low_pulse: bool,

    /// IRQ_IN pulse wake-up
    ///
    /// Wake up when the host sends a low pulse on the IRQ_IN GPIO pin.
    /// This provides host-controlled wake-up with precise timing.
    ///
    /// **Use cases**:
    /// - Host-initiated wake-up for scheduled operations
    /// - Synchronization with other system events
    /// - Emergency wake-up from external triggers
    pub irq_in_low_pulse: bool,

    /// External RF field detection wake-up
    ///
    /// Wake up when an external RF field is detected. This is useful for:
    /// - Detecting nearby NFC readers
    /// - Multi-reader collision avoidance
    /// - Power-saving in reader-dense environments
    ///
    /// **Note**: Only works when the ST25R95 is not generating its own field.
    pub field_detection: bool,

    /// Tag presence detection wake-up
    ///
    /// Wake up when a tag enters the RF field during periodic field generation.
    /// This requires **prior calibration** using `calibrate_tag_detector()`.
    ///
    /// **Use cases**:
    /// - Automated inventory systems
    /// - Tag presence monitoring
    /// - Smart shelf applications
    ///
    /// **Warning**: Requires calibration and always enable timeout as failsafe.
    pub tag_detection: bool,

    /// Timeout wake-up
    ///
    /// Wake up after a specified time period. This provides a reliable
    /// failsafe mechanism and is recommended when using other wake-up
    /// sources to prevent infinite idle periods.
    ///
    /// **Configuration**: The timeout duration is set by the `max_sleep`
    /// parameter in `IdleParams`.
    pub timeout: bool,
}

impl From<WakeUpSource> for u8 {
    fn from(wus: WakeUpSource) -> Self {
        (wus.lfo_freq as u8) << 6
            | (wus.ss_low_pulse as u8) << 4
            | (wus.irq_in_low_pulse as u8) << 3
            | (wus.field_detection as u8) << 2
            | (wus.tag_detection as u8) << 1
            | wus.timeout as u8
    }
}

impl TryFrom<u8> for WakeUpSource {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(WakeUpSource {
            lfo_freq: LFOFreq::try_from((value >> 6) & 0b11)?,
            ss_low_pulse: (value >> 4) & 1 == 1,
            irq_in_low_pulse: (value >> 3) & 1 == 1,
            field_detection: (value >> 2) & 1 == 1,
            tag_detection: (value >> 1) & 1 == 1,
            timeout: value & 1 == 1,
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CtrlResConf {
    /// EnterCtrl/WuCtrl bit 14. Required to be set in EnterCtrl when waking
    /// on FieldDetect — observed empirically: the ST25R95 reference RFAL
    /// driver hardcodes it (`0x42` in the Idle template's high byte) along
    /// with `field_detector_enabled` (bit 9), and without it the chip
    /// enters a power state from which `IRQ_IN pulse + Reset + IDN` does
    /// not revive it (the SPI MISO line stays at `0xff`, chip not driving).
    /// Likely the analog support circuit for the FieldDetector (the
    /// digital comparator output is `field_detector_enabled`); the bit is
    /// not symbolically named in the ST25R95 driver headers shipped with
    /// RFAL, so the name here is a best guess pending datasheet access.
    pub field_detect_aux_enabled: bool,
    pub field_detector_enabled: bool,
    pub iref_enabled: bool, /* TODO: Must to be set to 1 in WUCtrl for tag detection
                             * operations, otherwise must be put to 0 */
    pub dac_comp_high: bool,
    pub lfo_enabled: bool, // TODO: Must be set to 1 in WUCtrl
    pub hfo_enabled: bool, // TODO: Must be set to 1 in WUCtrl
    pub vdda_enabled: bool,
    pub hibernate_state_enabled: bool,
    pub sleep_state_enabled: bool,
}

impl Default for CtrlResConf {
    fn default() -> Self {
        Self {
            field_detect_aux_enabled: false,
            field_detector_enabled: false,
            iref_enabled: false,
            dac_comp_high: false,
            lfo_enabled: false,
            hfo_enabled: false,
            vdda_enabled: false,
            hibernate_state_enabled: true,
            sleep_state_enabled: false,
        }
    }
}

impl CtrlResConf {
    /// Every resource off.
    ///
    /// [`CtrlResConf::default`] keeps the hibernate state enabled, which is
    /// the right answer for one Idle configuration and the wrong one for the
    /// rest, so the [`IdleParams`] constructors start from this and name only
    /// the resources they need.
    pub const NONE: Self = Self {
        field_detect_aux_enabled: false,
        field_detector_enabled: false,
        iref_enabled: false,
        dac_comp_high: false,
        lfo_enabled: false,
        hfo_enabled: false,
        vdda_enabled: false,
        hibernate_state_enabled: false,
        sleep_state_enabled: false,
    };
}

impl From<CtrlResConf> for u16 {
    fn from(ctrl: CtrlResConf) -> Self {
        (ctrl.field_detect_aux_enabled as u16) << 14
            | (ctrl.field_detector_enabled as u16) << 9
            | (ctrl.iref_enabled as u16) << 8
            | (ctrl.dac_comp_high as u16) << 7
            | (ctrl.lfo_enabled as u16) << 5
            | (ctrl.hfo_enabled as u16) << 4
            | (ctrl.vdda_enabled as u16) << 3
            | (ctrl.hibernate_state_enabled as u16) << 2
            | ctrl.sleep_state_enabled as u16
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DacData {
    /// Lower compare value for tag detection.
    /// This value must be set to 0x00 during tag detection calibration.
    pub low: u8,
    /// Higher compare value for tag detection.
    /// This is a variable used during tag detection calibration.
    pub high: u8,
}

/// Configuration parameters for the Idle command
///
/// This structure defines the complete configuration for entering low-power
/// idle mode with the ST25R95. It controls wake-up sources, power management,
/// timing parameters, and advanced features like tag detection.
///
/// ## Power Management States
///
/// The Idle command can place the ST25R95 in several low-power states:
/// - **Sleep**: Moderate power savings, fast wake-up
/// - **Deep Sleep**: Higher power savings, slower wake-up
/// - **Hibernate**: Maximum power savings, reset required
///
/// ## Wake-up Sources
///
/// Multiple wake-up sources can be enabled simultaneously:
/// - **IRQ_IN pulse**: Host-controlled wake-up
/// - **Field detection**: External RF field detected
/// - **Tag detection**: Tag enters RF field (requires calibration)
/// - **Timeout**: Automatic wake-up after specified time
/// - **SPI communication**: SPI activity detection
///
/// ## Tag Detection Calibration
///
/// For reliable tag detection, the DAC comparator thresholds must be
/// calibrated using the `calibrate_tag_detector()` method. This establishes
/// the optimal `dac_data.high` and `dac_data.low` values for the specific
/// hardware environment.
///
/// ## Usage Examples
///
/// ```rust
/// # use st25r95::{
/// #     mock::{MockGpio, MockSpi},
/// #     IdleParams,
/// #     St25r95,
/// # };
/// # let mut nfc = St25r95::new(MockSpi::default(), MockGpio)?;
/// // Simple idle with IRQ_IN wake-up, waiting up to 5 s for the host pulse
/// let wake_source = nfc.idle(IdleParams::irq_in(), 5_000)?;
///
/// // Tag detection mode, which needs a prior `calibrate_tag_detector`:
/// // 0x20 wake-up periods per trial, at most 8 trials before the timeout
/// let params = IdleParams::tag_detection(0x20, 0x08)?;
/// # let _ = (wake_source, params);
/// # Ok::<(), st25r95::Error>(())
/// ```
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct IdleParams {
    /// Wake-up sources configuration and LFO frequency
    ///
    /// Defines which events can wake the ST25R95 from idle mode and
    /// the LFO frequency used for periodic checking.
    pub wus: WakeUpSource,

    /// Power management settings for entering idle mode
    ///
    /// Configures which circuits remain active when entering idle mode.
    /// This affects power consumption and wake-up capabilities.
    pub enter_ctrl: CtrlResConf,

    /// Power management settings for wake-up period
    ///
    /// Configures which circuits are activated during periodic wake-up
    /// checks (e.g., for field or tag detection).
    pub wu_ctrl: CtrlResConf,

    /// Power management settings for leaving idle mode
    ///
    /// Configures which circuits are activated when leaving idle mode
    /// and returning to normal operation.
    pub leave_ctrl: CtrlResConf,

    /// Wake-up period timing
    ///
    /// Specifies the time interval between periodic wake-up checks
    /// when using timed wake-up or tag detection. Value is multiplied
    /// by the LFO period to determine the actual timing.
    ///
    /// - **Range**: 0-255
    /// - **Effect**: Higher values = longer intervals = lower power
    pub wu_period: u8,

    /// Oscillator startup delay
    ///
    /// Defines the wait time for the High-Frequency Oscillator (HFO)
    /// to stabilize after wake-up before RF operations can begin.
    ///
    /// - **Range**: 0-255
    /// - **Effect**: Must be long enough for stable oscillation
    pub osc_start: u8,

    /// DAC startup delay  
    ///
    /// Defines the wait time for the Digital-to-Analog Converter (DAC)
    /// to stabilize before tag detection measurements.
    ///
    /// - **Range**: 0-255
    /// - **Effect**: Must be long enough for DAC settling
    pub dac_start: u8,

    /// Tag detection threshold values
    ///
    /// Contains the high and low comparator thresholds for tag detection.
    /// These values must be calibrated for reliable operation using
    /// `calibrate_tag_detector()`.
    pub dac_data: DacData,

    /// Number of RF field swings during tag detection
    ///
    /// Specifies how many cycles of the RF field are generated during
    /// each tag detection attempt. More swings provide better detection
    /// but consume more power.
    ///
    /// - **Range**: 0-255
    /// - **Trade-off**: Power consumption vs. detection reliability
    pub swing_count: u8,

    /// Maximum number of detection attempts before timeout
    ///
    /// Specifies how many tag detection trials are performed before
    /// giving up and reporting a timeout. During calibration, this
    /// should be set to 0x01.
    ///
    /// - **Range**: 0-255
    /// - **Calibration**: Must be 0x01 during `calibrate_tag_detector()`
    /// - **Normal use**: Higher values for better reliability
    pub max_sleep: u8,
}

impl Default for IdleParams {
    /// The timings and thresholds every Idle configuration starts from.
    ///
    /// This names no wake-up source, so it is not on its own a configuration
    /// the chip can be asked to enter: [`St25r95::idle`](crate::St25r95::idle)
    /// rejects it with [`Error::IdleNoWakeUpSource`]. Reach for one of the
    /// constructors - [`hibernate`](Self::hibernate), [`irq_in`](Self::irq_in),
    /// [`spi_ss`](Self::spi_ss), [`timeout`](Self::timeout),
    /// [`field_detection`](Self::field_detection) or
    /// [`tag_detection`](Self::tag_detection) - and adjust the result.
    fn default() -> Self {
        Self {
            wus: WakeUpSource::default(),
            enter_ctrl: CtrlResConf::default(),
            wu_ctrl: CtrlResConf::default(),
            leave_ctrl: CtrlResConf {
                field_detect_aux_enabled: false,
                field_detector_enabled: false,
                iref_enabled: false,
                dac_comp_high: false,
                lfo_enabled: false,
                hfo_enabled: true,
                vdda_enabled: true,
                hibernate_state_enabled: false,
                sleep_state_enabled: false,
            },
            wu_period: 0x20,
            osc_start: 0x60,
            dac_start: 0x60,
            dac_data: DacData {
                low: 0x64,
                high: 0x74,
            },
            swing_count: 0x3F,
            max_sleep: 0x08,
        }
    }
}

/// Smallest `MaxSleep` the ST25R95 accepts
///
/// The datasheet (DS12807) constrains the field to `0x00 < MaxSleep < 0x1F`.
pub const MIN_MAX_SLEEP: u8 = 0x01;

/// Largest `MaxSleep` the ST25R95 accepts
///
/// See [`MIN_MAX_SLEEP`].
pub const MAX_MAX_SLEEP: u8 = 0x1E;

impl IdleParams {
    /// Idle in hibernate, woken only by a low pulse on IRQ_IN.
    ///
    /// The deepest state the chip offers, and the only one that stops the
    /// low-frequency oscillator: nothing left running can count time or watch
    /// the field, so IRQ_IN is the only way back and the chip comes back
    /// through its power-up sequence.
    ///
    /// Reproduces the datasheet's "Active mode to Hibernate state" example.
    pub fn hibernate() -> Self {
        Self {
            wus: WakeUpSource {
                irq_in_low_pulse: true,
                ..WakeUpSource::default()
            },
            enter_ctrl: CtrlResConf {
                hibernate_state_enabled: true,
                ..CtrlResConf::NONE
            },
            wu_ctrl: CtrlResConf {
                hibernate_state_enabled: true,
                ..CtrlResConf::NONE
            },
            wu_period: 0x00,
            osc_start: 0x00,
            dac_start: 0x00,
            dac_data: DacData {
                low: 0x00,
                high: 0x00,
            },
            swing_count: 0x00,
            max_sleep: 0x00,
            ..Self::default()
        }
    }

    /// Idle in sleep, woken by a low pulse on IRQ_IN.
    ///
    /// Draws more than [`hibernate`](Self::hibernate) but returns to the Ready
    /// state, so the host can send the next command straight away.
    ///
    /// Reproduces the datasheet's "Active to WFE mode, wake-up by low pulse on
    /// IRQ_IN" example.
    pub fn irq_in() -> Self {
        Self {
            wus: WakeUpSource {
                irq_in_low_pulse: true,
                ..WakeUpSource::default()
            },
            enter_ctrl: CtrlResConf {
                sleep_state_enabled: true,
                ..CtrlResConf::NONE
            },
            wu_ctrl: CtrlResConf {
                lfo_enabled: true,
                hfo_enabled: true,
                vdda_enabled: true,
                ..CtrlResConf::NONE
            },
            wu_period: 0x00,
            osc_start: 0x60,
            dac_start: 0x00,
            dac_data: DacData {
                low: 0x00,
                high: 0x00,
            },
            swing_count: 0x00,
            max_sleep: 0x00,
            ..Self::default()
        }
    }

    /// Idle in sleep, woken by a low pulse on SPI_SS.
    ///
    /// The same state as [`irq_in`](Self::irq_in), reached by asserting the
    /// SPI chip select instead of driving IRQ_IN, which saves a GPIO on boards
    /// that have none to spare.
    ///
    /// Reproduces the datasheet's "Active to WFE mode, wake-up by low pulse on
    /// SPI_SS" example.
    pub fn spi_ss() -> Self {
        Self {
            wus: WakeUpSource {
                ss_low_pulse: true,
                ..WakeUpSource::default()
            },
            ..Self::irq_in()
        }
    }

    /// Idle in sleep for a bounded time, woken by the internal timer.
    ///
    /// The low-frequency oscillator keeps running to count the sleep, so this
    /// cannot be combined with hibernate. How long the sleep lasts follows
    /// from `lfo_freq`, `wu_period` and `max_sleep`;
    /// [`duration_before_timeout`](Self::duration_before_timeout) computes it.
    ///
    /// Reproduces the datasheet's "wake-up by Timeout" example when called
    /// with `(LFOFreq::KHz32, 0x00, 0x08)`.
    ///
    /// Returns [`Error::InvalidU8Parameter`] when `max_sleep` is outside
    /// [`MIN_MAX_SLEEP`]`..=`[`MAX_MAX_SLEEP`].
    pub fn timeout(lfo_freq: LFOFreq, wu_period: u8, max_sleep: u8) -> crate::Result<Self> {
        let params = Self {
            wus: WakeUpSource {
                lfo_freq,
                timeout: true,
                ..WakeUpSource::default()
            },
            enter_ctrl: CtrlResConf {
                lfo_enabled: true,
                sleep_state_enabled: true,
                ..CtrlResConf::NONE
            },
            wu_ctrl: CtrlResConf {
                lfo_enabled: true,
                hfo_enabled: true,
                vdda_enabled: true,
                ..CtrlResConf::NONE
            },
            wu_period,
            osc_start: 0x60,
            dac_start: 0x60,
            dac_data: DacData {
                low: 0x00,
                high: 0x00,
            },
            swing_count: 0x00,
            max_sleep,
            ..Self::default()
        };
        params.validate()?;
        Ok(params)
    }

    /// Idle in sleep, woken by an external RF field or by the host.
    ///
    /// Enabling the field detector takes two EnterCtrl bits, not one: without
    /// [`field_detect_aux_enabled`](CtrlResConf::field_detect_aux_enabled)
    /// alongside
    /// [`field_detector_enabled`](CtrlResConf::field_detector_enabled) the
    /// chip enters a state that IRQ_IN, Reset and IDN do not bring it out of.
    /// Setting both together is the reason this constructor exists.
    ///
    /// Reproduces the Idle template of ST's own ST25R95 reference driver,
    /// IRQ_IN included so the host can always cut the wait short.
    pub fn field_detection() -> Self {
        Self {
            wus: WakeUpSource {
                lfo_freq: LFOFreq::KHz32,
                irq_in_low_pulse: true,
                field_detection: true,
                ..WakeUpSource::default()
            },
            enter_ctrl: CtrlResConf {
                field_detect_aux_enabled: true,
                field_detector_enabled: true,
                sleep_state_enabled: true,
                ..CtrlResConf::NONE
            },
            wu_ctrl: CtrlResConf {
                lfo_enabled: true,
                hfo_enabled: true,
                vdda_enabled: true,
                ..CtrlResConf::NONE
            },
            wu_period: 0x20,
            osc_start: 0x60,
            dac_start: 0x60,
            dac_data: DacData {
                low: 0x74,
                high: 0x84,
            },
            swing_count: 0x3F,
            // Nothing here counts sleeps, so `MaxSleep` stays inactive and its
            // datasheet range does not apply.
            max_sleep: 0x00,
            ..Self::default()
        }
    }

    /// Idle in sleep, woken by a tag entering the periodically generated
    /// field, or by the host.
    ///
    /// The chip raises the field and measures it on every trial, so the
    /// reference current and both oscillators stay available during the
    /// wake-up period. `max_sleep` bounds the number of trials.
    ///
    /// The comparator thresholds are left at their defaults on purpose:
    /// [`St25r95::idle`](crate::St25r95::idle) overwrites them with the result
    /// of [`calibrate_tag_detector`](crate::St25r95::calibrate_tag_detector),
    /// and refuses to send the command when that calibration has not run.
    ///
    /// Reproduces the datasheet's "Active to Tag detector mode" example when
    /// called with `(0x20, 0x08)`. Add `timeout: true` to
    /// [`wus`](Self::wus) for a failsafe that reports why the chip woke up
    /// even when no tag arrived.
    ///
    /// Returns [`Error::InvalidU8Parameter`] when `max_sleep` is outside
    /// [`MIN_MAX_SLEEP`]`..=`[`MAX_MAX_SLEEP`].
    pub fn tag_detection(wu_period: u8, max_sleep: u8) -> crate::Result<Self> {
        let params = Self {
            wus: WakeUpSource {
                lfo_freq: LFOFreq::KHz32,
                irq_in_low_pulse: true,
                tag_detection: true,
                ..WakeUpSource::default()
            },
            enter_ctrl: CtrlResConf {
                lfo_enabled: true,
                sleep_state_enabled: true,
                ..CtrlResConf::NONE
            },
            wu_ctrl: CtrlResConf {
                iref_enabled: true,
                lfo_enabled: true,
                hfo_enabled: true,
                vdda_enabled: true,
                sleep_state_enabled: true,
                ..CtrlResConf::NONE
            },
            wu_period,
            max_sleep,
            ..Self::default()
        };
        params.validate()?;
        Ok(params)
    }

    /// The Idle used by each step of the tag detector calibration.
    ///
    /// One trial per call, thresholds driven by the caller rather than by a
    /// previous calibration, and the timeout enabled so a trial that detects
    /// nothing still reports back. These are the bytes ST's reference driver
    /// sends while calibrating.
    pub(crate) fn tag_detector_calibration() -> Self {
        Self {
            wus: WakeUpSource {
                lfo_freq: LFOFreq::KHz32,
                tag_detection: true,
                timeout: true,
                ..WakeUpSource::default()
            },
            enter_ctrl: CtrlResConf {
                dac_comp_high: true,
                lfo_enabled: true,
                sleep_state_enabled: true,
                ..CtrlResConf::NONE
            },
            wu_ctrl: CtrlResConf {
                iref_enabled: true,
                dac_comp_high: true,
                lfo_enabled: true,
                hfo_enabled: true,
                vdda_enabled: true,
                ..CtrlResConf::NONE
            },
            wu_period: 0x00,
            dac_data: DacData {
                low: 0x00,
                high: 0x00,
            },
            max_sleep: MIN_MAX_SLEEP,
            ..Self::default()
        }
    }

    /// Check that the configuration describes a chip that can wake up again.
    ///
    /// The fields of this struct are public, so a caller can always assemble a
    /// combination the ST25R95 does not implement. Some of those are merely
    /// useless; others leave the chip in a state only a power cycle clears,
    /// which on a board without independent NFC rail control means the whole
    /// product. [`St25r95::idle`](crate::St25r95::idle) and
    /// [`St25r95::idle_async`](crate::St25r95::idle_async) call this before
    /// touching the SPI bus, so a rejected configuration never reaches the
    /// chip.
    ///
    /// Rejected here:
    ///
    /// - no wake-up source at all, which is a sleep nothing ends;
    /// - field detection without both of its EnterCtrl bits;
    /// - tag detection without the resources its trials need;
    /// - field and tag detection at once, which would ask the chip to watch for an
    ///   external field and to generate its own;
    /// - hibernate with any wake-up source other than IRQ_IN, since hibernate stops the
    ///   oscillator every other source depends on;
    /// - hibernate and sleep requested from the same control word;
    /// - `MaxSleep` outside the datasheet range while something counts sleeps.
    pub fn validate(&self) -> crate::Result<()> {
        let wus = self.wus;
        let any_source = wus.ss_low_pulse
            || wus.irq_in_low_pulse
            || wus.field_detection
            || wus.tag_detection
            || wus.timeout;
        if !any_source {
            return Err(Error::IdleNoWakeUpSource);
        }

        if wus.field_detection && wus.tag_detection {
            return Err(Error::IdleConflictingWakeUpSources);
        }

        if wus.field_detection
            && !(self.enter_ctrl.field_detect_aux_enabled && self.enter_ctrl.field_detector_enabled)
        {
            return Err(Error::IdleFieldDetectionIncomplete);
        }

        if wus.tag_detection
            && !(self.wu_ctrl.iref_enabled && self.wu_ctrl.lfo_enabled && self.wu_ctrl.hfo_enabled)
        {
            return Err(Error::IdleTagDetectionIncomplete);
        }

        for ctrl in [self.enter_ctrl, self.wu_ctrl, self.leave_ctrl] {
            if ctrl.hibernate_state_enabled && ctrl.sleep_state_enabled {
                return Err(Error::IdleConflictingPowerStates);
            }
        }

        if self.enter_ctrl.hibernate_state_enabled
            && (wus.ss_low_pulse || wus.field_detection || wus.tag_detection || wus.timeout)
        {
            return Err(Error::IdleHibernateNeedsIrqIn);
        }

        // `MaxSleep` bounds how many wake-up periods pass before the chip
        // gives up, so it only means anything when something counts them.
        if (wus.timeout || wus.tag_detection)
            && !(MIN_MAX_SLEEP..=MAX_MAX_SLEEP).contains(&self.max_sleep)
        {
            return Err(Error::InvalidU8Parameter {
                min: MIN_MAX_SLEEP,
                max: MAX_MAX_SLEEP,
                actual: self.max_sleep,
            });
        }

        Ok(())
    }

    pub(crate) fn data(self) -> [u8; 14] {
        let mut data = [0u8; 14];
        data[0] = self.wus.into();
        let enter_ctrl: u16 = self.enter_ctrl.into();
        data[1..3].copy_from_slice(&enter_ctrl.to_le_bytes());
        let wu_ctrl: u16 = self.wu_ctrl.into();
        data[3..5].copy_from_slice(&wu_ctrl.to_le_bytes());
        let leave_ctrl: u16 = self.leave_ctrl.into();
        data[5..7].copy_from_slice(&leave_ctrl.to_le_bytes());
        data[7] = self.wu_period;
        data[8] = self.osc_start;
        data[9] = self.dac_start;
        data[10] = self.dac_data.low;
        data[11] = self.dac_data.high;
        data[12] = self.swing_count;
        data[13] = self.max_sleep;
        data
    }

    pub fn duration_before_timeout(self) -> f32 {
        256.0
            * self.wus.lfo_freq.period_us()
            * (self.wu_period as f32 + 2.0)
            * (self.max_sleep as f32 + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_wakeup_source() {
        // Wake-up by Timeout
        assert_eq!(
            u8::from(WakeUpSource {
                lfo_freq: LFOFreq::KHz32,
                ss_low_pulse: false,
                irq_in_low_pulse: false,
                field_detection: false,
                tag_detection: false,
                timeout: true,
            }),
            0x01
        );
        // Wake-up by tag detect
        assert_eq!(
            u8::from(WakeUpSource {
                lfo_freq: LFOFreq::KHz32,
                ss_low_pulse: false,
                irq_in_low_pulse: false,
                field_detection: false,
                tag_detection: true,
                timeout: false,
            }),
            0x02
        );
        // Wake-up by low pulse on IRQ_IN pin
        assert_eq!(
            u8::from(WakeUpSource {
                lfo_freq: LFOFreq::KHz32,
                ss_low_pulse: false,
                irq_in_low_pulse: true,
                field_detection: false,
                tag_detection: false,
                timeout: false,
            }),
            0x08
        );
    }

    #[test]
    pub fn test_ctrl_res_conf() {
        assert_eq!(
            CtrlResConf {
                field_detect_aux_enabled: false,
                field_detector_enabled: false,
                iref_enabled: false,
                dac_comp_high: false,
                lfo_enabled: false,
                hfo_enabled: false,
                vdda_enabled: false,
                hibernate_state_enabled: true,
                sleep_state_enabled: false,
            },
            CtrlResConf::default() // Hibernate
        );
        assert_eq!(
            u16::from(CtrlResConf {
                field_detect_aux_enabled: false,
                field_detector_enabled: false,
                iref_enabled: false,
                dac_comp_high: false,
                lfo_enabled: false,
                hfo_enabled: false,
                vdda_enabled: false,
                hibernate_state_enabled: true,
                sleep_state_enabled: false,
            }),
            0x0004 // Hibernate
        );
        assert_eq!(
            u16::from(CtrlResConf {
                field_detect_aux_enabled: false,
                field_detector_enabled: false,
                iref_enabled: false,
                dac_comp_high: false,
                lfo_enabled: false,
                hfo_enabled: true,
                vdda_enabled: true,
                hibernate_state_enabled: false,
                sleep_state_enabled: false,
            }),
            0x0018 // default Leave control
        );
    }

    #[test]
    pub fn test_idle_self() {
        // Example of switch from Active mode to Hibernate state
        assert_eq!(
            IdleParams {
                wus: WakeUpSource {
                    lfo_freq: LFOFreq::KHz32,
                    ss_low_pulse: false,
                    irq_in_low_pulse: true,
                    field_detection: false,
                    tag_detection: false,
                    timeout: false,
                },
                wu_period: 0,
                osc_start: 0,
                dac_start: 0,
                dac_data: DacData { low: 0, high: 0 },
                swing_count: 0,
                max_sleep: 0,
                ..Default::default()
            }
            .data(),
            [0x08, 0x04, 0x00, 0x04, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        // Example of switch from Active to WFE mode (wake-up by low pulse on IRQ_IN pin)
        assert_eq!(
            IdleParams {
                wus: WakeUpSource {
                    lfo_freq: LFOFreq::KHz32,
                    ss_low_pulse: false,
                    irq_in_low_pulse: true,
                    field_detection: false,
                    tag_detection: false,
                    timeout: false,
                },
                enter_ctrl: CtrlResConf {
                    field_detect_aux_enabled: false,
                    field_detector_enabled: false,
                    iref_enabled: false,
                    dac_comp_high: false,
                    lfo_enabled: false,
                    hfo_enabled: false,
                    vdda_enabled: false,
                    hibernate_state_enabled: false,
                    sleep_state_enabled: true,
                },
                wu_ctrl: CtrlResConf {
                    field_detect_aux_enabled: false,
                    field_detector_enabled: false,
                    iref_enabled: false,
                    dac_comp_high: false,
                    lfo_enabled: true,
                    hfo_enabled: true,
                    vdda_enabled: true,
                    hibernate_state_enabled: false,
                    sleep_state_enabled: false,
                },
                wu_period: 0,
                dac_start: 0,
                dac_data: DacData { low: 0, high: 0 },
                swing_count: 0,
                max_sleep: 0,
                ..Default::default()
            }
            .data(),
            [0x08, 0x01, 0x00, 0x38, 0x00, 0x18, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        // Example of switch from Active to WFE mode (wake-up by low pulse on SPI_SS pin)
        assert_eq!(
            IdleParams {
                wus: WakeUpSource {
                    lfo_freq: LFOFreq::KHz32,
                    ss_low_pulse: true,
                    irq_in_low_pulse: false,
                    field_detection: false,
                    tag_detection: false,
                    timeout: false,
                },
                enter_ctrl: CtrlResConf {
                    field_detect_aux_enabled: false,
                    field_detector_enabled: false,
                    iref_enabled: false,
                    dac_comp_high: false,
                    lfo_enabled: false,
                    hfo_enabled: false,
                    vdda_enabled: false,
                    hibernate_state_enabled: false,
                    sleep_state_enabled: true,
                },
                wu_ctrl: CtrlResConf {
                    field_detect_aux_enabled: false,
                    field_detector_enabled: false,
                    iref_enabled: false,
                    dac_comp_high: false,
                    lfo_enabled: true,
                    hfo_enabled: true,
                    vdda_enabled: true,
                    hibernate_state_enabled: false,
                    sleep_state_enabled: false,
                },
                wu_period: 0,
                dac_start: 0,
                dac_data: DacData { low: 0, high: 0 },
                swing_count: 0,
                max_sleep: 0,
                ..Default::default()
            }
            .data(),
            [0x10, 0x01, 0x00, 0x38, 0x00, 0x18, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        // Example of wake-up by Timeout (7 seconds)
        assert_eq!(
            IdleParams {
                wus: WakeUpSource {
                    lfo_freq: LFOFreq::KHz32,
                    ss_low_pulse: false,
                    irq_in_low_pulse: false,
                    field_detection: false,
                    tag_detection: false,
                    timeout: true,
                },
                enter_ctrl: CtrlResConf {
                    field_detect_aux_enabled: false,
                    field_detector_enabled: false,
                    iref_enabled: false,
                    dac_comp_high: false,
                    lfo_enabled: true,
                    hfo_enabled: false,
                    vdda_enabled: false,
                    hibernate_state_enabled: false,
                    sleep_state_enabled: true,
                },
                wu_ctrl: CtrlResConf {
                    field_detect_aux_enabled: false,
                    field_detector_enabled: false,
                    iref_enabled: false,
                    dac_comp_high: false,
                    lfo_enabled: true,
                    hfo_enabled: true,
                    vdda_enabled: true,
                    hibernate_state_enabled: false,
                    sleep_state_enabled: false,
                },
                wu_period: 0,
                dac_data: DacData { low: 0, high: 0 },
                swing_count: 0,
                ..Default::default()
            }
            .data(),
            [0x01, 0x21, 0x00, 0x38, 0x00, 0x18, 0x00, 0x00, 0x60, 0x60, 0x00, 0x00, 0x00, 0x08]
        );
        // Example of switch from Active to Tag detector mode (wake-up by tag detection or low
        // pulse on IRQ_IN pin) (32 kHz, inactivity duration = 272 ms, DAC oscillator = 3 ms,
        // Swing = 63 pulses of 13.56 MHz)
        assert_eq!(
            IdleParams {
                wus: WakeUpSource {
                    lfo_freq: LFOFreq::KHz32,
                    ss_low_pulse: false,
                    irq_in_low_pulse: true,
                    field_detection: false,
                    tag_detection: true,
                    timeout: false,
                },
                enter_ctrl: CtrlResConf {
                    field_detect_aux_enabled: false,
                    field_detector_enabled: false,
                    iref_enabled: false,
                    dac_comp_high: false,
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
                    dac_comp_high: false,
                    lfo_enabled: true,
                    hfo_enabled: true,
                    vdda_enabled: true,
                    hibernate_state_enabled: false,
                    sleep_state_enabled: true,
                },
                ..Default::default()
            }
            .data(),
            [0x0A, 0x21, 0x00, 0x39, 0x01, 0x18, 0x00, 0x20, 0x60, 0x60, 0x64, 0x74, 0x3F, 0x08] /* Datasheet gives bytes[3] = 0x79 (with bit 6 set) */
        );
        // Example of a basic Idle command used during the Tag detection Calibration process
        assert_eq!(
            IdleParams {
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
                dac_data: DacData {
                    low: 0x00,
                    high: 0x74
                },
                max_sleep: 0x01,
                ..Default::default()
            }
            .data(),
            [0x03, 0xA1, 0x00, 0xB8, 0x01, 0x18, 0x00, 0x20, 0x60, 0x60, 0x00, 0x74, 0x3F, 0x01] /* Datasheet gives bytes[3] = 0xF8 (with bit 6 set) */
        );
        // Idle template of an earlier RFAL release, kept as a serialization
        // fixture. The 1.7.0 template is the field-detection one asserted by
        // `test_idle_constructors_match_published_commands`.
        assert_eq!(
            IdleParams {
                wus: WakeUpSource {
                    lfo_freq: LFOFreq::KHz32,
                    ss_low_pulse: false,
                    irq_in_low_pulse: true,
                    field_detection: false,
                    tag_detection: true,
                    timeout: false,
                },
                enter_ctrl: CtrlResConf {
                    field_detect_aux_enabled: false,
                    field_detector_enabled: false,
                    iref_enabled: false,
                    dac_comp_high: false,
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
                    dac_comp_high: false,
                    lfo_enabled: true,
                    hfo_enabled: true,
                    vdda_enabled: true,
                    hibernate_state_enabled: false,
                    sleep_state_enabled: false,
                },
                dac_data: DacData {
                    low: 0x74,
                    high: 0x84
                },
                max_sleep: 0x00,
                ..Default::default()
            }
            .data(),
            [0x0A, 0x21, 0x00, 0x38, 0x01, 0x18, 0x00, 0x20, 0x60, 0x60, 0x74, 0x84, 0x3F, 0x00]
        );
        // RFAL Calibrate default value
        // RFAL can only modify wu_period and dac_data
        assert_eq!(
            IdleParams {
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
                    high: 0x00
                },
                max_sleep: 0x01,
                ..Default::default()
            }
            .data(),
            [0x03, 0xA1, 0x00, 0xB8, 0x01, 0x18, 0x00, 0x00, 0x60, 0x60, 0x00, 0x00, 0x3F, 0x01]
        );
    }

    /// Every constructor has to reproduce a published command, byte for byte.
    ///
    /// The expectations are the Idle examples of the ST25R95 datasheet and, for
    /// field detection and calibration, the templates in ST's own reference
    /// driver. A constructor that drifts from them stops being a known-good
    /// configuration, which is the only reason it exists.
    #[test]
    pub fn test_idle_constructors_match_published_commands() {
        // Datasheet: Active mode to Hibernate state.
        assert_eq!(
            IdleParams::hibernate().data(),
            [0x08, 0x04, 0x00, 0x04, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        // Datasheet: Active to WFE mode, wake-up by low pulse on IRQ_IN.
        assert_eq!(
            IdleParams::irq_in().data(),
            [0x08, 0x01, 0x00, 0x38, 0x00, 0x18, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        // Datasheet: Active to WFE mode, wake-up by low pulse on SPI_SS.
        assert_eq!(
            IdleParams::spi_ss().data(),
            [0x10, 0x01, 0x00, 0x38, 0x00, 0x18, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        // Datasheet: wake-up by Timeout.
        assert_eq!(
            IdleParams::timeout(LFOFreq::KHz32, 0x00, 0x08)
                .unwrap()
                .data(),
            [0x01, 0x21, 0x00, 0x38, 0x00, 0x18, 0x00, 0x00, 0x60, 0x60, 0x00, 0x00, 0x00, 0x08]
        );
        // Datasheet: Active to Tag detector mode. Byte 3 is 0x79 there; bit 6
        // of WuCtrl has no name in `CtrlResConf`, so the driver sends 0x39.
        assert_eq!(
            IdleParams::tag_detection(0x20, 0x08).unwrap().data(),
            [0x0A, 0x21, 0x00, 0x39, 0x01, 0x18, 0x00, 0x20, 0x60, 0x60, 0x64, 0x74, 0x3F, 0x08]
        );
        // ST reference driver: the Idle sent while calibrating.
        assert_eq!(
            IdleParams::tag_detector_calibration().data(),
            [0x03, 0xA1, 0x00, 0xB8, 0x01, 0x18, 0x00, 0x00, 0x60, 0x60, 0x00, 0x00, 0x3F, 0x01]
        );
        // ST reference driver: the field-detection Idle, both EnterCtrl bits
        // set (0x42 in the high byte).
        assert_eq!(
            IdleParams::field_detection().data(),
            [0x0C, 0x01, 0x42, 0x38, 0x00, 0x18, 0x00, 0x20, 0x60, 0x60, 0x74, 0x84, 0x3F, 0x00]
        );
    }

    #[test]
    pub fn test_idle_constructors_validate() {
        for params in [
            IdleParams::hibernate(),
            IdleParams::irq_in(),
            IdleParams::spi_ss(),
            IdleParams::timeout(LFOFreq::KHz4, 0x20, MIN_MAX_SLEEP).unwrap(),
            IdleParams::timeout(LFOFreq::KHz32, 0x20, MAX_MAX_SLEEP).unwrap(),
            IdleParams::field_detection(),
            IdleParams::tag_detection(0x20, MIN_MAX_SLEEP).unwrap(),
            IdleParams::tag_detection(0x20, MAX_MAX_SLEEP).unwrap(),
            IdleParams::tag_detector_calibration(),
        ] {
            assert_eq!(params.validate(), Ok(()));
        }

        // The timeout failsafe is safe to add to tag detection.
        let mut tag = IdleParams::tag_detection(0x20, 0x08).unwrap();
        tag.wus.timeout = true;
        assert_eq!(tag.validate(), Ok(()));
    }

    #[test]
    pub fn test_idle_rejects_a_sleep_nothing_ends() {
        // The bare default names no wake-up source.
        assert_eq!(
            IdleParams::default().validate(),
            Err(Error::IdleNoWakeUpSource)
        );
    }

    #[test]
    pub fn test_idle_rejects_incomplete_field_detection() {
        // This is the combination the public documentation used to suggest:
        // the wake-up source set from a default that leaves both EnterCtrl
        // bits clear.
        let params = IdleParams {
            wus: WakeUpSource {
                field_detection: true,
                ..Default::default()
            },
            enter_ctrl: CtrlResConf::NONE,
            ..Default::default()
        };
        assert_eq!(params.validate(), Err(Error::IdleFieldDetectionIncomplete));

        // The auxiliary bit on its own is not enough either way round.
        for enter_ctrl in [
            CtrlResConf {
                field_detector_enabled: true,
                ..CtrlResConf::NONE
            },
            CtrlResConf {
                field_detect_aux_enabled: true,
                ..CtrlResConf::NONE
            },
        ] {
            let params = IdleParams {
                enter_ctrl,
                ..IdleParams::field_detection()
            };
            assert_eq!(params.validate(), Err(Error::IdleFieldDetectionIncomplete));
        }
    }

    #[test]
    pub fn test_idle_rejects_incomplete_tag_detection() {
        for wu_ctrl in [
            CtrlResConf {
                iref_enabled: false,
                ..IdleParams::tag_detection(0x20, 0x08).unwrap().wu_ctrl
            },
            CtrlResConf {
                lfo_enabled: false,
                ..IdleParams::tag_detection(0x20, 0x08).unwrap().wu_ctrl
            },
            CtrlResConf {
                hfo_enabled: false,
                ..IdleParams::tag_detection(0x20, 0x08).unwrap().wu_ctrl
            },
        ] {
            let params = IdleParams {
                wu_ctrl,
                ..IdleParams::tag_detection(0x20, 0x08).unwrap()
            };
            assert_eq!(params.validate(), Err(Error::IdleTagDetectionIncomplete));
        }
    }

    #[test]
    pub fn test_idle_rejects_mixed_configurations() {
        // Watching for someone else's field while generating one.
        let mut params = IdleParams::field_detection();
        params.wus.tag_detection = true;
        assert_eq!(params.validate(), Err(Error::IdleConflictingWakeUpSources));

        // Hibernate stops the oscillator the timer runs on.
        let mut params = IdleParams::hibernate();
        params.wus.timeout = true;
        params.max_sleep = 0x08;
        assert_eq!(params.validate(), Err(Error::IdleHibernateNeedsIrqIn));

        // ... and the one the field detector runs on.
        let params = IdleParams {
            enter_ctrl: CtrlResConf {
                hibernate_state_enabled: true,
                sleep_state_enabled: false,
                ..IdleParams::field_detection().enter_ctrl
            },
            ..IdleParams::field_detection()
        };
        assert_eq!(params.validate(), Err(Error::IdleHibernateNeedsIrqIn));

        // Sleep and hibernate are alternatives, not resources to combine.
        let params = IdleParams {
            enter_ctrl: CtrlResConf {
                hibernate_state_enabled: true,
                sleep_state_enabled: true,
                ..CtrlResConf::NONE
            },
            ..IdleParams::irq_in()
        };
        assert_eq!(params.validate(), Err(Error::IdleConflictingPowerStates));
    }

    #[test]
    pub fn test_idle_rejects_reserved_max_sleep() {
        // The datasheet requires 0x00 < MaxSleep < 0x1F.
        for max_sleep in [0x00, 0x1F, 0xFF] {
            assert_eq!(
                IdleParams::timeout(LFOFreq::KHz32, 0x20, max_sleep),
                Err(Error::InvalidU8Parameter {
                    min: MIN_MAX_SLEEP,
                    max: MAX_MAX_SLEEP,
                    actual: max_sleep,
                })
            );
            assert_eq!(
                IdleParams::tag_detection(0x20, max_sleep),
                Err(Error::InvalidU8Parameter {
                    min: MIN_MAX_SLEEP,
                    max: MAX_MAX_SLEEP,
                    actual: max_sleep,
                })
            );
        }

        // Nothing counts sleeps in these, so the field stays inactive and out
        // of range values are none of the driver's business.
        for params in [IdleParams::hibernate(), IdleParams::field_detection()] {
            let params = IdleParams {
                max_sleep: 0xFF,
                ..params
            };
            assert_eq!(params.validate(), Ok(()));
        }
    }
}
