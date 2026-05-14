//! Shared error / audio-type definitions mirroring `esp_audio_types.h`.

use esp_idf_sys::{
    esp_audio_err_t, esp_audio_err_t_ESP_AUDIO_ERR_ALREADY_EXIST,
    esp_audio_err_t_ESP_AUDIO_ERR_BUFF_NOT_ENOUGH, esp_audio_err_t_ESP_AUDIO_ERR_CONTINUE,
    esp_audio_err_t_ESP_AUDIO_ERR_DATA_LACK, esp_audio_err_t_ESP_AUDIO_ERR_FAIL,
    esp_audio_err_t_ESP_AUDIO_ERR_HEADER_PARSE,
    esp_audio_err_t_ESP_AUDIO_ERR_INVALID_PARAMETER, esp_audio_err_t_ESP_AUDIO_ERR_MEM_LACK,
    esp_audio_err_t_ESP_AUDIO_ERR_NOT_FOUND, esp_audio_err_t_ESP_AUDIO_ERR_NOT_SUPPORT,
    esp_audio_err_t_ESP_AUDIO_ERR_OK, esp_audio_type_t,
};

/// `esp_audio_err_t` mapped to a `Result`-friendly Rust enum. `Ok` is
/// represented by `Result::Ok`; this enum only covers the error path
/// plus the "continue" sentinel.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(i32)]
pub enum Error {
    /// `ESP_AUDIO_ERR_CONTINUE` — operation should be retried with more input.
    Continue = esp_audio_err_t_ESP_AUDIO_ERR_CONTINUE,
    /// Generic failure.
    Fail = esp_audio_err_t_ESP_AUDIO_ERR_FAIL,
    /// Out of memory.
    MemLack = esp_audio_err_t_ESP_AUDIO_ERR_MEM_LACK,
    /// More input data is required to make progress.
    DataLack = esp_audio_err_t_ESP_AUDIO_ERR_DATA_LACK,
    /// Failure while parsing a stream header.
    HeaderParse = esp_audio_err_t_ESP_AUDIO_ERR_HEADER_PARSE,
    /// One of the supplied parameters was invalid.
    InvalidParameter = esp_audio_err_t_ESP_AUDIO_ERR_INVALID_PARAMETER,
    /// Resource already exists / already registered.
    AlreadyExist = esp_audio_err_t_ESP_AUDIO_ERR_ALREADY_EXIST,
    /// Operation not supported (e.g. codec not registered).
    NotSupport = esp_audio_err_t_ESP_AUDIO_ERR_NOT_SUPPORT,
    /// Output buffer too small; reallocate to `needed_size` and retry.
    BuffNotEnough = esp_audio_err_t_ESP_AUDIO_ERR_BUFF_NOT_ENOUGH,
    /// Requested item not found.
    NotFound = esp_audio_err_t_ESP_AUDIO_ERR_NOT_FOUND,
}

impl Error {
    /// Convert a raw `esp_audio_err_t` into a `Result`. `OK` becomes `Ok(())`.
    pub fn check(code: esp_audio_err_t) -> Result<(), Error> {
        if code == esp_audio_err_t_ESP_AUDIO_ERR_OK {
            Ok(())
        } else {
            Err(Self::from_raw(code))
        }
    }

    pub(crate) fn from_raw(code: esp_audio_err_t) -> Self {
        match code {
            esp_audio_err_t_ESP_AUDIO_ERR_CONTINUE => Self::Continue,
            esp_audio_err_t_ESP_AUDIO_ERR_MEM_LACK => Self::MemLack,
            esp_audio_err_t_ESP_AUDIO_ERR_DATA_LACK => Self::DataLack,
            esp_audio_err_t_ESP_AUDIO_ERR_HEADER_PARSE => Self::HeaderParse,
            esp_audio_err_t_ESP_AUDIO_ERR_INVALID_PARAMETER => Self::InvalidParameter,
            esp_audio_err_t_ESP_AUDIO_ERR_ALREADY_EXIST => Self::AlreadyExist,
            esp_audio_err_t_ESP_AUDIO_ERR_NOT_SUPPORT => Self::NotSupport,
            esp_audio_err_t_ESP_AUDIO_ERR_BUFF_NOT_ENOUGH => Self::BuffNotEnough,
            esp_audio_err_t_ESP_AUDIO_ERR_NOT_FOUND => Self::NotFound,
            _ => Self::Fail,
        }
    }
}

/// `esp_audio_type_t` — codec identifier (FourCC). Listed variants cover
/// the codecs declared in `esp_audio_types.h`; we expose only the wide
/// enumeration so callers can match on it directly.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u32)]
pub enum AudioType {
    Unsupported = 0,
    AmrNb = fourcc(b'A', b'M', b'R', b'N'),
    AmrWb = fourcc(b'A', b'M', b'R', b'W'),
    Aac = fourcc(b'A', b'A', b'C', b' '),
    G711A = fourcc(b'A', b'L', b'A', b'W'),
    G711U = fourcc(b'U', b'L', b'A', b'W'),
    Opus = fourcc(b'O', b'P', b'U', b'S'),
    Adpcm = fourcc(b'A', b'D', b'P', b'C'),
    Pcm = fourcc(b'P', b'C', b'M', b' '),
    Flac = fourcc(b'F', b'L', b'A', b'C'),
    Vorbis = fourcc(b'V', b'O', b'B', b'S'),
    Mp3 = fourcc(b'M', b'P', b'3', b' '),
    Alac = fourcc(b'A', b'L', b'A', b'C'),
    Sbc = fourcc(b'S', b'B', b'C', b' '),
    Lc3 = fourcc(b'L', b'C', b'3', b'0'),
    G722 = fourcc(b'G', b'7', b'2', b'2'),
}

impl AudioType {
    pub(crate) fn as_raw(self) -> esp_audio_type_t {
        self as esp_audio_type_t
    }
}

const fn fourcc(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (a as u32) | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
}

/// `esp_audio_dec_info_t` — metadata reported by the decoder once it has
/// parsed enough input to know the stream parameters.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AudioInfo {
    pub sample_rate: u32,
    pub bits_per_sample: u8,
    pub channels: u8,
    pub bitrate: u32,
    pub frame_size: u32,
}
