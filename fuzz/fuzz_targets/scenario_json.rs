#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz JSON scenario input parsing.
    // We don't assert success — we assert that parsing never panics.
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_json::from_str::<trit_core::sandbox::ScenarioInput>(s);
    }
});
