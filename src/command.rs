// SPDX-FileCopyrightText: 2023 Foundation Devices, Inc. <hello@foundationdevices.com>
// SPDX-License-Identifier: GPL-3.0-or-later

#[derive(Debug, Copy, Clone)]
pub(crate) enum Command {
    Idn = 0x01,
    ProtocolSelect = 0x02,
    PollField = 0x03,
    SendRecv = 0x04,
    Listen = 0x05,
    Send = 0x06,
    Idle = 0x07,
    // RdReg = 0x08,
    WrReg = 0x09,
    // SubFreqRes = 0x0A,
    // ACFilter = 0x0B,
    Echo = 0x55,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct WaitForField {
    pub apparance: bool,
    pub presc: u8,
    pub timer: u8,
}

impl WaitForField {
    pub fn us(self) -> f32 {
        (((self.presc) as f32 + 1f32) * ((self.timer as f32) + 1f32)) / 13.56f32
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub enum LFOFreq {
    #[default]
    KHz32 = 0b00,
    KHz16 = 0b01,
    KHz8 = 0b10,
    KHz4 = 0b11,
}

impl LFOFreq {
    /// tL = 1/fLFO
    pub fn period_us(self) -> f32 {
        match self {
            LFOFreq::KHz32 => 31.25,
            LFOFreq::KHz16 => 62.5,
            LFOFreq::KHz8 => 125.0,
            LFOFreq::KHz4 => 250.0,
        }
    }

    /// tREF = 256*tL ms (where tL = 1/fLFO)
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

/// Specifies authorized wake-up sources and the LFO frequency
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct WakeUpSource {
    pub lfo_freq: LFOFreq,
    pub ss_low_pulse: bool,
    pub irq_in_low_pulse: bool,
    pub field_detection: bool,
    pub tag_detection: bool,
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
    pub field_detector_enabled: bool,
    pub iref_enabled: bool, /* TODO: Must to be set to 1 in WUCtrLl for tag detection
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

impl From<CtrlResConf> for u16 {
    fn from(ctrl: CtrlResConf) -> Self {
        (ctrl.field_detector_enabled as u16) << 9
            | (ctrl.iref_enabled as u16) << 8
            | (ctrl.dac_comp_high as u16) << 7
            | (ctrl.lfo_enabled as u16) << 5
            | (ctrl.hfo_enabled as u16) << 4
            | (ctrl.vdda_enabled as u16) << 3
            | (ctrl.hibernate_state_enabled as u16) << 2
            | ctrl.sleep_state_enabled as u16
    }
}

#[derive(Debug, Copy, Clone)]
pub struct DacData {
    /// Lower compare value for tag detection.
    /// This value must be set to 0x00 during tag detection calibration.
    pub low: u8,
    /// Higher compare value for tag detection.
    /// This is a variable used during tag detection calibration.
    pub high: u8,
}

#[derive(Debug, Copy, Clone)]
pub struct IdleParams {
    /// Specifies authorized wake-up sources and the LFO frequency
    pub wus: WakeUpSource,
    /// Settings to enter WFE mode
    pub enter_ctrl: CtrlResConf,
    /// Settings to wake-up from WFE mode
    pub wu_ctrl: CtrlResConf,
    /// Settings to leave WFE mode
    pub leave_ctrl: CtrlResConf,
    /// Period of time between two tag detection bursts.
    /// Also used to specify the duration before Timeout.
    pub wu_period: u8,
    /// Defines the wait time for HFO to stabilize
    pub osc_start: u8,
    /// Defines the wait time for DAC to stabilize
    pub dac_start: u8,
    /// Compare values for tag detection
    pub dac_data: DacData,
    /// Number of swings HF during tag detection
    pub swing_count: u8,
    /// Max. number of tag detection trials before Timeout.
    /// This value must be set to 0x01 during tag detection calibration.
    /// Also used to specify duration before Timeout.
    pub max_sleep: u8,
}

impl Default for IdleParams {
    fn default() -> Self {
        Self {
            wus: WakeUpSource::default(),
            enter_ctrl: CtrlResConf::default(),
            wu_ctrl: CtrlResConf::default(),
            leave_ctrl: CtrlResConf {
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

impl IdleParams {
    // TODO: impl a Builder that check max_sleep range

    pub fn to_bytes(self) -> [u8; 14] {
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
            .to_bytes(),
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
            .to_bytes(),
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
            .to_bytes(),
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
            .to_bytes(),
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
            .to_bytes(),
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
            .to_bytes(),
            [0x03, 0xA1, 0x00, 0xB8, 0x01, 0x18, 0x00, 0x20, 0x60, 0x60, 0x00, 0x74, 0x3F, 0x01] /* Datasheet gives bytes[3] = 0xF8 (with bit 6 set) */
        );
        // RFAL Idle default value
        // RFAL can only modify wu_period and dac_data
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
            .to_bytes(),
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
            .to_bytes(),
            [0x03, 0xA1, 0x00, 0xB8, 0x01, 0x18, 0x00, 0x00, 0x60, 0x60, 0x00, 0x00, 0x3F, 0x01]
        );
    }
}
