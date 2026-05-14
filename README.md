# esp-audio-codec

Safe Rust bindings for Espressif's [`esp_audio_codec`](https://github.com/espressif/esp-adf-libs/tree/master/esp_audio_codec)
component — the audio encoder/decoder engine that ships in esp-adf-libs and is
distributed standalone via the Component Registry as `espressif/esp_audio_codec`.

This crate wraps the generic engine (`esp_audio_dec.h` / `esp_audio_enc.h`)
plus a growing list of per-codec constructors. Pair it with `esp-idf-svc`
to feed encoded audio out of A2DP / HTTP / SD card / etc. into PCM you can
hand to I2S.

## Status

Pre-release. Currently wrapped:

| Side    | Codec | Constructor               | Cfg gate                            |
| ------- | ----- | ------------------------- | ----------------------------------- |
| Decoder | AAC   | `Decoder::aac(AacConfig)` | `esp_idf_audio_decoder_aac_support` |
| Decoder | SBC   | `Decoder::sbc(SbcConfig)` | `esp_idf_audio_decoder_sbc_support` |

The generic `Decoder::new(AudioType)` works for any codec the engine knows
about — the codec-specific constructors only exist for codecs whose C API
takes a non-trivial configuration struct.

Encoder side and the higher-level `simple_dec` (auto-detecting container +
codec) wrapper are planned but not yet implemented.

## Adding the crate to a project

In your app's `Cargo.toml`:

```toml
[dependencies]
esp-audio-codec = { git = "https://github.com/billylindeman/esp-audio-codec" }
# or, while developing locally:
# esp-audio-codec = { path = "../esp-audio-codec" }
```

In your app's `sdkconfig.defaults` (or the layered fragment you point
`ESP_IDF_SDKCONFIG_DEFAULTS` at), enable the codecs you want:

```
CONFIG_AUDIO_DECODER_AAC_SUPPORT=y
CONFIG_AUDIO_DECODER_SBC_SUPPORT=y
# ... other codecs as needed
```

Nothing else. The `espressif/esp_audio_codec` component itself is pulled
from the Component Registry automatically via this crate's
`[[package.metadata.esp-idf-sys.extra_components]]` entry.

## How codec gating works

Each codec module in `src/decoder/` is gated on the Kconfig-derived cfg
that `esp-idf-sys` emits — there are no cargo features for codec selection.
Set `CONFIG_<CODEC>_SUPPORT=y` in your sdkconfig and the corresponding
`Decoder::<codec>(...)` constructor becomes available; leave it off and
the constructor isn't compiled at all (so you can't accidentally call a
codec whose registration is disabled).

This design keeps a single source of truth: the C component's Kconfig.

## Example

```rust
use esp_audio_codec::{
    decoder::{aac::AacConfig, sbc::SbcConfig, DecodeOutput, Decoder, DefaultDecoders, FrameRecovery},
    AudioInfo, Error,
};

// Call once at startup. Hold the returned guard for program lifetime; on
// drop it calls `esp_audio_dec_unregister_default`.
let _decoders = DefaultDecoders::register()?;

// AAC sink path
let mut aac = Decoder::aac(AacConfig::default())?;

// SBC sink path (e.g. A2DP fallback)
let mut sbc = Decoder::sbc(SbcConfig::default())?;

// Loop over an input frame:
let mut pcm = [0u8; 4096];
let mut in_buf: &[u8] = encoded_aac_frame;
while !in_buf.is_empty() {
    match aac.process(in_buf, &mut pcm, FrameRecovery::Normal) {
        Ok(DecodeOutput { consumed, decoded, .. }) => {
            i2s_write(&pcm[..decoded]);
            in_buf = &in_buf[consumed..];
        }
        Err(Error::BuffNotEnough) => {
            // realloc `pcm` to >= out.needed and retry
            break;
        }
        Err(e) => {
            log::warn!("aac decode: {e:?}");
            break;
        }
    }
}

// Stream parameters become available after the first successful decode:
let info: AudioInfo = aac.info()?;
log::info!("aac stream: {} Hz, {} ch, {} bps", info.sample_rate, info.channels, info.bitrate);
```

## Requirements

- ESP-IDF v6.0+ — earlier versions don't have the cfg-propagation
  embuild needs.
- The `esp` Rust toolchain (install via [`espup`](https://github.com/esp-rs/espup)).
- Xtensa or RISC-V ESP target as appropriate; A2DP sink callers are
  limited to ESP32 by the underlying Bluedroid stack.

## Building standalone

For local hacking (`cargo check`, `cargo clippy`):

```bash
ESP_IDF_VERSION=master \
ESP_IDF_SDKCONFIG_DEFAULTS=path/to/sdkconfig.defaults \
MCU=esp32 \
cargo +esp check --target xtensa-esp32-espidf
```

`build-std` is configured in `.cargo/config.toml`, so no `-Z` flag needed.

## License

Dual-licensed under Apache-2.0 or MIT, at your option.

Note: the underlying `esp_audio_codec` C library is distributed under
Espressif's Modified MIT License, which restricts use to Espressif
hardware. This crate is a wrapper only — the runtime distribution
constraint is on the C library itself, not on this Rust code.
