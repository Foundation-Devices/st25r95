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
    /// ```rust
    /// # struct MyGpio;
    /// # impl MyGpio {
    /// #     fn set_irq_in_high(&mut self) {}
    /// #     fn set_irq_in_low(&mut self) {}
    /// #     fn delay_us(&mut self, _us: u32) {}
    /// # }
    /// # impl st25r95::St25r95Gpio for MyGpio {
    /// # fn wait_irq_out_falling_edge(&mut self, _timeout: u32) -> Result<(), ()> {
    /// #     Ok(())
    /// # }
    /// fn irq_in_pulse_low(&mut self) {
    ///     // make sure we start with a high state
    ///     self.set_irq_in_high();
    ///     self.delay_us(10);
    ///     self.set_irq_in_low();
    ///     // Wait at least 1μs (use timer or delay)
    ///     self.delay_us(10);
    ///     self.set_irq_in_high();
    ///     self.delay_us(11_000);
    /// }
    /// # }
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
    /// - `timeout`: Maximum time to wait in milliseconds. The driver derives it from the
    ///   timeout configured on the chip for the operation in flight, so an implementation
    ///   must honour the full duration rather than capping it; returning early makes the
    ///   driver report `Error::ResponseInProgress` for an operation the chip is still
    ///   legitimately serving.
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
    /// ```rust
    /// # struct MyGpio;
    /// # impl MyGpio {
    /// #     fn now_ms(&self) -> u32 { 1 }
    /// #     fn irq_out_is_low(&self) -> bool { true }
    /// # }
    /// # impl st25r95::St25r95Gpio for MyGpio {
    /// # fn irq_in_pulse_low(&mut self) {}
    /// fn wait_irq_out_falling_edge(&mut self, timeout: u32) -> Result<(), ()> {
    ///     let start = self.now_ms();
    ///     while self.now_ms() - start < timeout {
    ///         if self.irq_out_is_low() {
    ///             return Ok(());
    ///         }
    ///     }
    ///     Err(())
    /// }
    /// # }
    /// ```
    ///
    /// **Interrupt approach** (efficient):
    /// ```rust
    /// # struct MyGpio;
    /// # impl MyGpio {
    /// #     fn configure_falling_edge_interrupt(&mut self) {}
    /// #     fn wait_for_interrupt(&mut self, _timeout: u32) -> Result<(), ()> { Ok(()) }
    /// # }
    /// # impl st25r95::St25r95Gpio for MyGpio {
    /// # fn irq_in_pulse_low(&mut self) {}
    /// fn wait_irq_out_falling_edge(&mut self, timeout: u32) -> Result<(), ()> {
    ///     // Configure external interrupt on falling edge
    ///     self.configure_falling_edge_interrupt();
    ///
    ///     // Wait for interrupt with timeout
    ///     self.wait_for_interrupt(timeout)
    /// }
    /// # }
    /// ```
    #[allow(clippy::result_unit_err)]
    fn wait_irq_out_falling_edge(&mut self, timeout: u32) -> Result<(), ()>;
}
