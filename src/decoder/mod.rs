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
    esp_audio_err_t_ESP_AUDIO_ERR_BUFF_NOT_ENOUGH, esp_audio_err_t_ESP_AUDIO_ERR_CONTINUE,
    esp_audio_err_t_ESP_AUDIO_ERR_DATA_LACK, esp_audio_err_t_ESP_AUDIO_ERR_OK,
};

use crate::types::{AudioInfo, AudioType, Error};

#[cfg(esp_idf_audio_decoder_aac_support)]
pub mod aac;
#[cfg(esp_idf_audio_decoder_sbc_support)]
pub mod sbc;

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

/// Result of one [`Decoder::process`] call. Mirrors every status the
/// underlying C `esp_audio_dec_process` can return and preserves the
/// `consumed` / `needed` values the decoder writes regardless of status —
/// crucial because all the non-OK statuses below are signals for how to
/// drive the next call, not fatal errors.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DecoderResult {
    /// Decoder made progress: `consumed` input bytes were used and
    /// `decoded` PCM bytes were written to the output buffer. Either
    /// may be zero on a given call (e.g. decoder absorbed input but
    /// didn't emit a frame yet, or emitted a frame from internal state
    /// without consuming new input). Advance input by `consumed`,
    /// write the PCM, call again.
    Ok { consumed: usize, decoded: usize },
    /// Decoder needs more input. `consumed` may still be > 0 (header
    /// bytes were absorbed). Advance input by `consumed`, append more
    /// bytes from the producer, call again.
    DataLack { consumed: usize },
    /// Output buffer too small for the next frame. Grow output to at
    /// least `needed` bytes and retry with the *same* input — do NOT
    /// advance by `consumed` here. Matches Espressif's `test_sbc.c`.
    BuffNotEnough { consumed: usize, needed: usize },
    /// Decoder asks to be called again with the same input. Advance by
    /// `consumed`, call again.
    Continue { consumed: usize },
    /// Hard error from the codec. Decoder state may be poisoned;
    /// consider [`Decoder::reset`] before continuing.
    Failed(Error),
}

/// Safe wrapper around an `esp_audio_dec_handle_t`.
pub struct Decoder {
    handle: esp_audio_dec_handle_t,
    /// Cached for the SBC consumed-bytes workaround in [`Decoder::process`].
    /// See the comment there for details.
    audio_type: AudioType,
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
        Ok(Self { handle, audio_type })
    }

    /// Decode bytes from `input` into `output`, returning a [`DecoderResult`]
    /// that captures both the decoder's status and the `consumed` /
    /// `needed` values it wrote. See the variants of [`DecoderResult`] for
    /// the action each one demands from the caller.
    pub fn process(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        recovery: FrameRecovery,
    ) -> DecoderResult {
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
        // Upstream bug in `esp_sbc_dec_decode` (esp-adf-libs): its success
        // path propagates OI's post-call `*frameBytes` (= REMAINING bytes)
        // directly into `raw->consumed` without translating to the
        // bytes-CONSUMED semantic the rest of the `esp_audio_dec` API
        // (AAC, MP3, Opus, FLAC, …) honors. Confirmed via disassembly of
        // `esp_sbc_dec.c.obj` cross-referenced with Android's
        // `embdrv/sbc/decoder/srce/decoder-sbc.c:322-325`. The AAC wrapper
        // does the equivalent translation; only SBC is off.
        //
        // We invert here at the FFI boundary so every consumer of this
        // binding sees the documented `esp_audio_dec` contract regardless
        // of codec. Remove this fixup once the upstream wrapper is fixed.
        //
        // Tracking: https://github.com/espressif/esp-adf-libs/issues/75
        let consumed = if matches!(self.audio_type, AudioType::Sbc) {
            (input.len() as u32).saturating_sub(raw.consumed) as usize
        } else {
            raw.consumed as usize
        };
        let decoded = frame.decoded_size as usize;
        let needed = frame.needed_size as usize;
        if result == esp_audio_err_t_ESP_AUDIO_ERR_OK {
            DecoderResult::Ok { consumed, decoded }
        } else if result == esp_audio_err_t_ESP_AUDIO_ERR_DATA_LACK {
            DecoderResult::DataLack { consumed }
        } else if result == esp_audio_err_t_ESP_AUDIO_ERR_BUFF_NOT_ENOUGH {
            DecoderResult::BuffNotEnough { consumed, needed }
        } else if result == esp_audio_err_t_ESP_AUDIO_ERR_CONTINUE {
            DecoderResult::Continue { consumed }
        } else {
            DecoderResult::Failed(Error::from_raw(result))
        }
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
