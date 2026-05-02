#![allow(
    dead_code,
    unused_variables,
    unused_imports,
    unused_assignments,
    unused_must_use
)]
//! J.A.R.V.I.S. V45: Prismatic Singularity
//!
//! The native Wayland hardware architecture for the Middle-Out Tesseract.

pub mod crypto;
pub mod font_8x8;
pub mod html_parser;
pub mod io_membrane;
pub mod kestrel;
pub mod mesh;
pub mod pretext_layout;
pub mod sysprep_debug;
pub mod zero_trust;

pub mod bare_metal;
pub mod headless;
pub mod watchdog;

use prismatic_core::{
    BackpressurePolicy, GlobalContext, LockFreeEventBus, PIDController, SHUTDOWN, SensoryEvent,
    temporal,
};
use std::sync::Arc;
use std::sync::atomic::Ordering;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--verify-sysprep") {
        sysprep_debug::run_purity_audit();
        return;
    }

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    kestrel::initialize_kestrel_hook();

    tracing::info!("=== J.A.R.V.I.S. V36 Prismatic OS Boot Sequence ===");

    tracing::info!("Initializing Hardware PID Controller...");
    let pid_cfg = PIDController::calibrate_on_boot();
    tracing::info!(
        "PID Configuration Locked: P={:.3}, I={:.3}, D={:.3}",
        pid_cfg.p_gain,
        pid_cfg.i_gain,
        pid_cfg.d_gain
    );

    let state = Arc::new(GlobalContext::new(
        prismatic_core::LILITH_CONFIG.hidden_size,
    ));

    GlobalContext::spawn_diagnostic_socket(state.clone());

    let bus: Arc<dyn prismatic_core::bus::EventBus<SensoryEvent>> =
        Arc::new(LockFreeEventBus::new(BackpressurePolicy::DropOldest));

    watchdog::spawn_watchdog(state.clone(), bus.clone());

    // Graceful Termination Hook
    let tx_ctrlc = bus.clone();
    ctrlc::set_handler(move || {
        tracing::warn!("Termination Signal Received. Broadcasting OS shutdown...");
        SHUTDOWN.store(true, Ordering::Relaxed);
        let _ = tx_ctrlc.push(SensoryEvent::Terminate);
    })
    .expect("Error setting Ctrl-C handler");

    let is_headless =
        std::env::var("WAYLAND_DISPLAY").is_err() && std::env::var("DISPLAY").is_err();
    if is_headless {
        headless::run_headless(state, bus);
    } else {
        bare_metal::run_bare_metal(state, bus);
    }
}
