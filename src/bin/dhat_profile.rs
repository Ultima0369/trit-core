// dhat heap profiling binary for Trit-Core.
//
// Verifies the zero-allocation claim on the hot path and measures
// allocation patterns on the cold path and end-to-end pipeline.
//
// Usage:
//   cargo build --release --bin dhat-profile --features dhat-profile
//   cargo run --release --bin dhat-profile --features dhat-profile
//   dhat-heap.json is written to the current directory.
//
// Analyze with:
//   dhat-viewer dhat-heap.json    (if installed)
//   Or upload to https://nnethercote.github.io/dh_view/dh_view.html

use std::error::Error;
use trit_core::core::frame::Frame;
use trit_core::core::phase::Phase;
use trit_core::core::value::TritValue;
use trit_core::core::word::TritWord;
use trit_core::core::TernaryAlgebra;
use trit_core::meta::{Domain, ResolutionPolicy, SafeFallback};

fn main() -> Result<(), Box<dyn Error>> {
    println!("Trit-Core dhat Heap Profiling");
    println!("==============================");

    // ── Sanity check: verify dhat is capturing allocations ──
    println!("\n[0/6] Sanity check: explicit Vec allocation...");
    let _sanity_profiler = dhat::Profiler::new_heap();
    {
        let v: Vec<u8> = vec![0u8; 1024];
        std::hint::black_box(&v);
    }
    drop(_sanity_profiler);

    // ── Hot path: TAND same-frame (should be zero-allocation) ──
    println!("\n[1/5] Profiling hot path: TAND same-frame x 100,000...");
    let _hot_profiler = dhat::Profiler::new_heap();
    {
        let a = TritWord::new(TritValue::True, Phase::new(0.8)?, Frame::Science);
        let b = TritWord::new(TritValue::False, Phase::new(0.2)?, Frame::Science);
        for _ in 0..100_000 {
            let _ = TernaryAlgebra::t_and_hot(&a, &b);
        }
    }
    drop(_hot_profiler);

    // ── Hot path: TOR same-frame ──
    println!("[2/5] Profiling hot path: TOR same-frame x 100,000...");
    let _tor_profiler = dhat::Profiler::new_heap();
    {
        let a = TritWord::new(TritValue::True, Phase::new(0.7)?, Frame::Science);
        let b = TritWord::new(TritValue::Hold, Phase::new(0.5)?, Frame::Science);
        for _ in 0..100_000 {
            let _ = TernaryAlgebra::t_or_hot(&a, &b);
        }
    }
    drop(_tor_profiler);

    // ── Hot path: TNOT ──
    println!("[3/5] Profiling hot path: TNOT x 100,000...");
    let _tnot_profiler = dhat::Profiler::new_heap();
    {
        let a = TritWord::new(TritValue::True, Phase::new(0.9)?, Frame::Science);
        for _ in 0..100_000 {
            let _ = TernaryAlgebra::t_not(&a);
        }
    }
    drop(_tnot_profiler);

    // ── Cold path: TAND cross-frame (should allocate MetaInterrupt) ──
    println!("[4/5] Profiling cold path: TAND cross-frame x 10,000...");
    let _cold_profiler = dhat::Profiler::new_heap();
    {
        let a = TritWord::new(TritValue::True, Phase::new(0.8)?, Frame::Science);
        let b = TritWord::new(TritValue::False, Phase::new(0.2)?, Frame::Individual);
        let mut interrupt_count = 0u64;
        for _ in 0..10_000 {
            let (result, interrupt) = TernaryAlgebra::t_and(&a, &b);
            if interrupt.is_some() {
                interrupt_count += 1;
            }
            std::hint::black_box(result);
        }
        // Force the count to be used so the interrupt path isn't optimized away
        println!("  Interrupts generated: {}", interrupt_count);
    }
    drop(_cold_profiler);

    // ── End-to-end pipeline: MedicalEthics ──
    println!("[5/5] Profiling end-to-end pipeline: MedicalEthics x 1,000...");
    let _pipeline_profiler = dhat::Profiler::new_heap();
    {
        let science = TritWord::new(TritValue::True, Phase::new(0.85)?, Frame::Science);
        let individual = TritWord::new(TritValue::False, Phase::new(0.20)?, Frame::Individual);

        for _ in 0..1_000 {
            let (result, interrupt) = TernaryAlgebra::t_and(&science, &individual);
            let policy = ResolutionPolicy::new(Domain::MedicalEthics);
            let arbitration = policy.arbitrate(&[science, individual])?;
            let sf = SafeFallback::new();
            let (final_result, _) = sf.guard(
                &Domain::MedicalEthics,
                &result,
                if interrupt.is_some() { 1 } else { 0 },
            );
            // Prevent optimization: use the result
            std::hint::black_box(final_result);
            std::hint::black_box(arbitration);
        }
    }
    drop(_pipeline_profiler);

    println!("\nProfiling complete.");
    println!("dhat-heap.json written to current directory.");
    println!("View with: dhat-viewer dhat-heap.json");
    println!("Or upload to: https://nnethercote.github.io/dh_view/dh_view.html");
    Ok(())
}
