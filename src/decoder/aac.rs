//! AAC decoder constructor. Compiled only when
//! `CONFIG_AUDIO_DECODER_AAC_SUPPORT=y` is in the project's sdkconfig.

use esp_idf_sys::esp_aac_dec_cfg_t;

use super::Decoder;
use crate::types::{AudioType, Error};

/// AAC decoder configuration. The defaults handle ADTS-wrapped AAC at
/// 48 kHz mono / 16-bit, which is what `ESP_AAC_DEC_CONFIG_DEFAULT()`
/// produces in the C header.
#[derive(Debug, Copy, Clone)]
pub struct AacConfig {
    /// Sample rate of the AAC stream. Only relevant when
    /// `no_adts_header == true`.
    pub sample_rate: u32,
    /// Channel count (1 or 2). Only relevant when
    /// `no_adts_header == true`.
    pub channels: u8,
    /// Bits per sample (typically 16). Only relevant when
    /// `no_adts_header == true`.
    pub bits_per_sample: u8,
    /// Set to `true` if the input bytes are raw AAC frames without ADTS
    /// headers (e.g. AAC inside an MP4 / LATM container). When `false`,
    /// the decoder reads the ADTS header and the `sample_rate` /
    /// `channels` / `bits_per_sample` fields are ignored.
    pub no_adts_header: bool,
    /// Enable HE-AAC (SBR / PS extensions).
    pub aac_plus_enable: bool,
}

impl Default for AacConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            channels: 1,
            bits_per_sample: 16,
            no_adts_header: false,
            aac_plus_enable: false,
        }
    }
}

impl Decoder {
    /// Open an AAC decoder. Pass `AacConfig::default()` for the common
    /// case of ADTS-framed AAC; supply an explicit config for non-ADTS
    /// or HE-AAC.
    pub fn aac(config: AacConfig) -> Result<Self, Error> {
        let mut raw = esp_aac_dec_cfg_t {
            sample_rate: config.sample_rate as i32,
            channel: config.channels,
            bits_per_sample: config.bits_per_sample,
            no_adts_header: config.no_adts_header,
            aac_plus_enable: config.aac_plus_enable,
        };
        // Safety: `raw` is a valid `esp_aac_dec_cfg_t` and stays alive for
        // the duration of `open_with` (which copies what it needs).
        unsafe {
            Decoder::open_with(
                AudioType::Aac,
                &mut raw as *mut _ as *mut core::ffi::c_void,
                core::mem::size_of::<esp_aac_dec_cfg_t>() as u32,
            )
        }
    }
}
