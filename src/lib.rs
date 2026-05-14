//! Safe Rust bindings for Espressif's `esp_audio_codec` component (esp-adf-libs).
//!
//! # Layout
//!
//! - [`Error`] / [`AudioType`] / [`AudioInfo`] — common types mirroring
//!   `esp_audio_err_t` / `esp_audio_type_t` / `esp_audio_dec_info_t`.
//! - [`decoder`] — generic decoder engine + per-codec constructors.
//! - encoder support and the simple-decoder wrapper are planned but not yet
//!   implemented.
//!
//! # Codec gating
//!
//! Per-codec bindings are gated on the Kconfig-derived cfgs that esp-idf-sys
//! emits, e.g. `cfg(esp_idf_audio_decoder_aac_support)` from
//! `CONFIG_AUDIO_DECODER_AAC_SUPPORT=y`. To enable a codec, set the
//! corresponding `CONFIG_*` line in your project's `sdkconfig.defaults`; no
//! cargo features needed.
//!
//! # Component sourcing
//!
//! The crate's `Cargo.toml` declares `espressif/esp_audio_codec` as an
//! `extra_components` entry for esp-idf-sys, so the underlying C library
//! gets fetched from the Component Registry automatically during the
//! downstream consumer's build.

#![no_std]
#![allow(non_upper_case_globals)]

pub mod decoder;
pub mod types;

pub use types::{AudioInfo, AudioType, Error};
