// SPDX-FileCopyrightText: 2025 Foundation Devices, Inc. <hello@foundation.xyz>
// SPDX-License-Identifier: GPL-3.0-or-later

/// Trait for GPIO interface with the ST25R95 NFC transceiver
///
/// This trait abstracts the GPIO signals required for proper operation of the
/// ST25R95 chip. The ST25R95 uses two GPIO signals for interrupt-based
/// communication and wake-up control.
///
/// ## GPIO Signals
///
/// - **IRQ_IN**: Wake-up/Control signal from host to ST25R95
/// - **IRQ_OUT**: Interrupt signal from ST25R95 to host
///
/// ## Hardware Connection
///
/// The GPIO connections should be made as follows:
/// - Host MCU GPIO Output pin → ST25R95 IRQ_IN pin
/// - Host MCU GPIO Input pin ← ST25R95 IRQ_OUT pin
///
/// This enables bidirectional communication for interrupt handling and wake-up control.
///
/// ## Signal Characteristics
///
/// Both signals should be configured as:
/// - **Push-pull output** for driving
/// - **Input with pull-up** for receiving
/// - **3.3V logic levels**
/// - **Low-speed GPIO** (no special timing requirements)
///
/// ## Implementation Notes
///
/// - IRQ_IN pulses should be at least 1μs wide
/// - IRQ_OUT is active-low (falling edge indicates interrupt)
/// - Both pins should be properly initialized before use
/// - Consider debouncing for noisy environments
pub trait St25r95Gpio {
    /// Generate a low pulse on the IRQ_IN pin
    ///
    /// This method creates a brief low pulse on the IRQ_IN pin to wake up
    /// or control the ST25R95. The pulse should be at least 1μs wide
    /// but typically 10-100μs is used for reliability.
    ///
    /// ## Usage Context
    ///
    /// This signal is used in several scenarios:
    /// - **Startup**: During initial chip startup sequence
    /// - **After reset**: To bring the chip out of reset state
    /// - **Wake-up**: To wake the chip from low-power modes
    /// - **Command cancel**: To cancel ongoing operations in some cases
    ///
    /// ## Implementation Requirements
    ///
    /// The pulse should follow this timing:
    /// ```text
    /// HIGH ─────┐      ┌─────────
    ///           │      │
    /// LOW       └──────┘
    ///           ↑←τmin→↑
    ///           τmin ≥ 1μs
    /// ```
    ///
    /// ## Example Implementation
    ///
    /// ```rust,ignore
    /// fn irq_in_pulse_low(&mut self) {
    ///     // make sure we start with a high state
    ///     self.irq_in.set_high().unwrap();
    ///     delay.delay_us(10);
    ///     self.irq_in.set_low().unwrap();
    ///     // Wait at least 1μs (use timer or delay)
    ///     delay.delay_us(10);
    ///     self.irq_in.set_high().unwrap();
    ///     delay.delay_ms(11);
    /// }
    /// ```
    fn irq_in_pulse_low(&mut self);

    /// Wait for a falling edge on the IRQ_OUT pin with timeout
    ///
    /// This method blocks until the IRQ_OUT pin transitions from high to low
    /// (falling edge) or the timeout expires. The IRQ_OUT signal from the
    /// ST25R95 indicates that the chip has completed an operation or needs
    /// attention.
    ///
    /// ## Parameters
    /// - `timeout`: Maximum time to wait in milliseconds
    ///
    /// ## Returns
    /// - `Ok(())`: Falling edge detected within timeout period
    /// - `Err(())`: Timeout occurred, no falling edge detected
    ///
    /// ## When IRQ_OUT is Activated
    ///
    /// The ST25R95 activates IRQ_OUT (falling edge) for:
    /// - **Command completion**: After processing SPI commands
    /// - **Data ready**: When response data is available for reading
    /// - **Field detection**: When an external RF field is detected
    /// - **Tag detection**: When a tag enters the RF field
    /// - **Wake-up events**: Various low-power wake-up conditions
    /// - **Error conditions**: When hardware errors occur
    ///
    /// ## Implementation Strategies
    ///
    /// **Polling approach** (simple):
    /// ```rust,ignore
    /// fn wait_irq_out_falling_edge(&mut self, timeout: u32) -> Result<(), ()> {
    ///     let start = get_current_time();
    ///     while get_current_time() - start < timeout {
    ///         if self.irq_out.is_low().unwrap() {
    ///             return Ok(());
    ///         }
    ///     }
    ///     Err(())
    /// }
    /// ```
    ///
    /// **Interrupt approach** (efficient):
    /// ```rust,ignore
    /// fn wait_irq_out_falling_edge(&mut self, timeout: u32) -> Result<(), ()> {
    ///     // Configure external interrupt on falling edge
    ///     self.irq_out.configure_interrupt(FallingEdge);
    ///     
    ///     // Wait for interrupt with timeout
    ///     self.wait_for_interrupt(timeout)
    /// }
    /// ```
    fn wait_irq_out_falling_edge(&mut self, timeout: u32) -> Result<(), ()>;
}
