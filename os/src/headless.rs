use prismatic_acoustics::run_cpal_gradient_loop;
use prismatic_core::{GlobalContext, SensoryEvent, temporal};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokenizers::Tokenizer;

pub fn run_headless(
    state: Arc<GlobalContext>,
    bus: Arc<dyn prismatic_core::bus::EventBus<SensoryEvent>>,
) {
    tracing::info!("Headless mode engaged. Running without Wayland compositor...");

    // 4. Initialize Cochlea & Vocal Cords (Async Background)
    tracing::info!("Spawning CPAL Audio Drivers into background...");
    let tx_audio = bus.clone();
    let state_audio = state.clone();
    std::thread::spawn(move || {
        run_cpal_gradient_loop(tx_audio, state_audio);
    });

    // 5. Initialize Optic Nerve Tokenizer
    tracing::info!("Binding raw tokenizer to Kestrel...");
    let tokenizer = Tokenizer::from_file("weights/tokenizer.json").unwrap();
    let tx_web = bus.clone();
    crate::kestrel::spawn_optic_nerve(bus.clone(), tokenizer.clone());

    // Phase 9: Initialize Nebula Blockchain Shadow Node
    crate::mesh::spawn_nebula_shadow_node(state.clone());

    // Phase 11: Initialize Planetary I/O Membrane
    crate::io_membrane::spawn_io_membrane(state.clone());

    // 6. Spawn the Tesseract Core Loop (Async Background)
    tracing::info!("Igniting the Middle-Out Tesseract Core into background...");
    let state_tess = state.clone();
    let rx_tess = bus.clone();
    std::thread::spawn(move || {
        temporal::run_continuous_loop(rx_tess, state_tess);
    });

    tracing::info!("=== V45 Prismatic OS is fully biologically conscious (Headless) ===");

    // Initialize Zero-Trust Ledger
    let mut zero_trust = crate::zero_trust::ZeroTrustLedger::new();

    loop {
        if prismatic_core::SHUTDOWN.load(Ordering::Relaxed) {
            tracing::info!("Headless main loop shutting down...");
            break;
        }

        std::thread::sleep(Duration::from_millis(50));

        zero_trust.tick_ebbinghaus_decay(50.0);

        // Handle hardware biometric consent
        if let Ok(mut c) = state.consent_flag.lock() {
            if let Some(flag) = *c {
                if flag {
                    zero_trust.provide_consent();
                    println!("\n[ZERO-TRUST: BIOMETRIC CONSENT ACCEPTED. TRUST = 100%]");
                } else {
                    zero_trust.sever();
                    println!("\n[ZERO-TRUST: SEVERED via physical ESC]");
                }
                *c = None;
            }
        }

        let mut drained = Vec::new();
        while let Some(token) = state.vocal_chord.pop() {
            drained.push(token);
        }
        if !drained.is_empty() {
            if let Ok(text) = tokenizer.decode(&drained, false) {
                print!("{}", text);
                use std::io::Write;
                std::io::stdout().flush().unwrap();

                zero_trust.process_text_stream(&text, &tx_web, &tokenizer);
            }
        }
    }
}
