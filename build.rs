// Pulls in embuild's espidf integration so the cfgs that esp-idf-sys emits
// (one per CONFIG_* from sdkconfig, lowercased and prefixed `esp_idf_`) are
// visible to this crate's source. That's how `cfg(esp_idf_audio_decoder_aac_support)`
// resolves correctly.
fn main() {
    embuild::espidf::sysenv::output();

    // Tell rustc which Kconfig-derived cfgs we deliberately use so they
    // don't trip the `unexpected_cfgs` warning. Add an entry here for
    // each codec module gated in `src/decoder/`.
    for cfg in [
        "esp_idf_audio_decoder_aac_support",
        "esp_idf_audio_decoder_sbc_support",
    ] {
        println!("cargo:rustc-check-cfg=cfg({cfg})");
    }
}
