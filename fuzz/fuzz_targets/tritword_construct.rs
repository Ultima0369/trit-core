#![no_main]

use libfuzzer_sys::fuzz_target;
use trit_core::core::{Frame, Phase, TritValue, TritWord};

fuzz_target!(|data: &[u8]| {
    if data.len() < 3 {
        return;
    }
    // Interpret first byte as value selector, second as frame selector,
    // remaining bytes as phase seed.
    let value = match data[0] % 4 {
        0 => TritValue::True,
        1 => TritValue::Hold,
        2 => TritValue::False,
        _ => TritValue::Unknown,
    };
    let frame = match data[1] % 6 {
        0 => Frame::Science,
        1 => Frame::Individual,
        2 => Frame::Consensus,
        3 => Frame::Absolute,
        4 => Frame::Meta,
        _ => Frame::FirstPerson,
    };
    // Map bytes 2.. to a phase in [0.0, 1.0].
    let phase_raw = if data.len() > 3 {
        let mut acc: u64 = 0;
        for &b in &data[2..] {
            acc = acc.wrapping_mul(256).wrapping_add(b as u64);
        }
        (acc % 1001) as f64 / 1000.0
    } else {
        0.5
    };

    // Both constructors must not panic.
    let _ = TritWord::new(value, Phase::new_clamped(phase_raw), frame);
    let _ = Phase::new(phase_raw);
});
