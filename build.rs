// Pulls in embuild's espidf integration so the cfgs that esp-idf-sys emits
// (one per CONFIG_* from sdkconfig, lowercased and prefixed `esp_idf_`) are
// visible to this crate's source. That's how `cfg(esp_idf_audio_decoder_aac_support)`
// resolves correctly.
fn main() {
    embuild::espidf::sysenv::output();
}
