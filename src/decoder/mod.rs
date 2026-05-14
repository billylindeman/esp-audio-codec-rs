//! Generic audio decoder, mirroring `decoder/esp_audio_dec.h`.
//!
//! A [`Decoder`] is obtained from a codec-specific constructor (e.g.
//! [`Decoder::aac`]) and processes encoded bytes into PCM. Per-codec
//! constructors live in submodules and are individually cfg-gated on
//! the corresponding `CONFIG_AUDIO_DECODER_*_SUPPORT` Kconfig.

use core::ptr;

use esp_idf_sys::{
    esp_audio_dec_close, esp_audio_dec_get_info, esp_audio_dec_handle_t, esp_audio_dec_in_raw_t,
    esp_audio_dec_info_t, esp_audio_dec_open, esp_audio_dec_out_frame_t, esp_audio_dec_process,
    esp_audio_dec_recovery_t_ESP_AUDIO_DEC_RECOVERY_NONE,
    esp_audio_dec_recovery_t_ESP_AUDIO_DEC_RECOVERY_PLC, esp_audio_dec_register_default,
    esp_audio_dec_reset, esp_audio_dec_unregister_default,
};

use crate::types::{AudioInfo, AudioType, Error};

#[cfg(esp_idf_audio_decoder_aac_support)]
pub mod aac;

/// One-shot registration of the codecs that were enabled at Kconfig time.
/// Holds the registration alive for the program's lifetime; dropping
/// unregisters via `esp_audio_dec_unregister_default`.
///
/// Most apps will call `DefaultDecoders::register()` once at startup and
/// store the returned handle in a `static`/`OnceCell`. The first call
/// performs the registration; subsequent ones return a fresh handle and
/// the underlying C call is idempotent (`ESP_AUDIO_ERR_ALREADY_EXIST` is
/// surfaced as an error).
pub struct DefaultDecoders {
    _private: (),
}

impl DefaultDecoders {
    pub fn register() -> Result<Self, Error> {
        Error::check(unsafe { esp_audio_dec_register_default() })?;
        Ok(Self { _private: () })
    }
}

impl Drop for DefaultDecoders {
    fn drop(&mut self) {
        unsafe { esp_audio_dec_unregister_default() };
    }
}

/// Hint about how the current decoded frame was produced.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FrameRecovery {
    Normal,
    /// Frame was synthesised by the codec's packet-loss-concealment path.
    Plc,
}

impl FrameRecovery {
    fn as_raw(self) -> u32 {
        match self {
            Self::Normal => esp_audio_dec_recovery_t_ESP_AUDIO_DEC_RECOVERY_NONE,
            Self::Plc => esp_audio_dec_recovery_t_ESP_AUDIO_DEC_RECOVERY_PLC,
        }
    }
}

/// Result of one [`Decoder::process`] call.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct DecodeOutput {
    /// Number of input bytes consumed by the decoder.
    pub consumed: usize,
    /// Number of PCM bytes written into the output buffer.
    pub decoded: usize,
    /// When `Err(Error::BuffNotEnough)` is returned from `process`, the
    /// minimum output buffer size needed for the next attempt.
    pub needed: usize,
}

/// Safe wrapper around an `esp_audio_dec_handle_t`.
pub struct Decoder {
    handle: esp_audio_dec_handle_t,
}

impl Decoder {
    /// Open a decoder for a given codec with no codec-specific options.
    /// For codecs that need configuration (e.g. AAC without ADTS), use the
    /// codec-specific constructor in the matching submodule instead.
    pub fn new(audio_type: AudioType) -> Result<Self, Error> {
        // Safety: passing a null config pointer with zero size is always
        // valid (per esp_audio_dec_open).
        unsafe { Self::open_with(audio_type, ptr::null_mut(), 0) }
    }

    /// # Safety
    ///
    /// `cfg` must point to a struct matching the codec's expected
    /// configuration type for `audio_type`, or be null. `cfg_sz` must be
    /// the byte length of that struct or zero. The pointed-to data must
    /// remain valid for the duration of this call; the decoder copies
    /// what it needs internally.
    pub(crate) unsafe fn open_with(
        audio_type: AudioType,
        cfg: *mut core::ffi::c_void,
        cfg_sz: u32,
    ) -> Result<Self, Error> {
        let mut config = esp_idf_sys::esp_audio_dec_cfg_t {
            type_: audio_type.as_raw(),
            cfg,
            cfg_sz,
        };
        let mut handle: esp_audio_dec_handle_t = ptr::null_mut();
        Error::check(esp_audio_dec_open(&mut config, &mut handle))?;
        Ok(Self { handle })
    }

    /// Decode bytes from `input` into `output`, returning how many bytes
    /// of each were consumed/produced.
    ///
    /// Callers typically call this in a loop, advancing the input slice
    /// by `consumed` each iteration, until the input is exhausted. On
    /// `Err(Error::BuffNotEnough)`, reallocate `output` to at least
    /// `needed` bytes and retry the same input.
    pub fn process(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        recovery: FrameRecovery,
    ) -> Result<DecodeOutput, Error> {
        let mut raw = esp_audio_dec_in_raw_t {
            buffer: input.as_ptr() as *mut u8,
            len: input.len() as u32,
            consumed: 0,
            frame_recover: recovery.as_raw(),
        };
        let mut frame = esp_audio_dec_out_frame_t {
            buffer: output.as_mut_ptr(),
            len: output.len() as u32,
            needed_size: 0,
            decoded_size: 0,
        };
        let result = unsafe { esp_audio_dec_process(self.handle, &mut raw, &mut frame) };
        let out = DecodeOutput {
            consumed: raw.consumed as usize,
            decoded: frame.decoded_size as usize,
            needed: frame.needed_size as usize,
        };
        Error::check(result)?;
        Ok(out)
    }

    /// Fetch stream metadata (sample rate, channels, bitrate, ...). Only
    /// returns useful data after at least one successful `process` call
    /// that produced output.
    pub fn info(&self) -> Result<AudioInfo, Error> {
        let mut info = esp_audio_dec_info_t {
            sample_rate: 0,
            bits_per_sample: 0,
            channel: 0,
            bitrate: 0,
            frame_size: 0,
        };
        Error::check(unsafe { esp_audio_dec_get_info(self.handle, &mut info) })?;
        Ok(AudioInfo {
            sample_rate: info.sample_rate,
            bits_per_sample: info.bits_per_sample,
            channels: info.channel,
            bitrate: info.bitrate,
            frame_size: info.frame_size,
        })
    }

    /// Reset internal state and any cached input/output buffers so the
    /// decoder can be re-used (e.g. after a seek).
    pub fn reset(&mut self) -> Result<(), Error> {
        Error::check(unsafe { esp_audio_dec_reset(self.handle) })
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe { esp_audio_dec_close(self.handle) };
    }
}

// Safety: the decoder handle is opaque and the C API uses internal
// synchronisation for its global registries. The per-handle state is not
// thread-safe, so we only mark Send (move-across-threads) and not Sync.
unsafe impl Send for Decoder {}
