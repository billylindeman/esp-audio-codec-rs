//! SBC decoder constructor. Compiled only when
//! `CONFIG_AUDIO_DECODER_SBC_SUPPORT=y` is in the project's sdkconfig.

use esp_idf_sys::{
    esp_sbc_dec_cfg_t, esp_sbc_mode_t, esp_sbc_mode_t_ESP_SBC_MODE_MSBC,
    esp_sbc_mode_t_ESP_SBC_MODE_STD,
};

use super::Decoder;
use crate::types::{AudioType, Error};

/// SBC operating mode. For A2DP sinks always use `Std`; `Msbc` is the
/// 16 kHz wideband variant used for HFP voice.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SbcMode {
    Std,
    Msbc,
}

impl SbcMode {
    fn as_raw(self) -> esp_sbc_mode_t {
        match self {
            Self::Std => esp_sbc_mode_t_ESP_SBC_MODE_STD,
            Self::Msbc => esp_sbc_mode_t_ESP_SBC_MODE_MSBC,
        }
    }
}

/// SBC decoder configuration. Mirrors `esp_sbc_dec_cfg_t`.
///
/// `Default` matches the C `ESP_SBC_DEC_CONFIG_DEFAULT()` macro: standard
/// SBC, stereo, PLC enabled.
#[derive(Debug, Copy, Clone)]
pub struct SbcConfig {
    pub mode: SbcMode,
    /// 1 (mono) or 2 (stereo). Ignored when `mode == Msbc`. See
    /// `esp_sbc_dec_cfg_t::ch_num` doc for the channel-mismatch rules.
    pub channels: u8,
    /// Enable packet-loss concealment. Only effective when `mode == Msbc`
    /// (PLC requires the caller to set `FrameRecovery::Plc` on the
    /// affected input frame).
    pub enable_plc: bool,
}

impl Default for SbcConfig {
    fn default() -> Self {
        Self {
            mode: SbcMode::Std,
            channels: 2,
            enable_plc: true,
        }
    }
}

impl Decoder {
    /// Open an SBC decoder. Pass `SbcConfig::default()` for A2DP stereo
    /// playback; supply `SbcMode::Msbc` for HFP wideband voice.
    pub fn sbc(config: SbcConfig) -> Result<Self, Error> {
        let mut raw = esp_sbc_dec_cfg_t {
            sbc_mode: config.mode.as_raw(),
            ch_num: config.channels,
            _bitfield_align_1: [],
            _bitfield_1: esp_sbc_dec_cfg_t::new_bitfield_1(config.enable_plc as u8),
            ..Default::default()
        };
        // Safety: `raw` is a valid `esp_sbc_dec_cfg_t` and stays alive
        // for the duration of `open_with` (which copies what it needs).
        unsafe {
            Decoder::open_with(
                AudioType::Sbc,
                &mut raw as *mut _ as *mut core::ffi::c_void,
                core::mem::size_of::<esp_sbc_dec_cfg_t>() as u32,
            )
        }
    }
}
