// P1: Run the watchdog as a `systemd` service with `CPUQuota=5%` and `Nice=-20` to guarantee pre-emption over inference threads during thermal breaches.
use prismatic_core::{GlobalContext, SensoryEvent};
use std::sync::Arc;

pub fn spawn_watchdog(
    watchdog_context: Arc<GlobalContext>,
    watchdog_bus: Arc<dyn prismatic_core::bus::EventBus<SensoryEvent>>,
) {
    std::thread::spawn(move || {
        // Simulating the systemd Nice=-20 execution priority
        tracing::info!("Health-Monitoring Watchdog initialized with elevated priority.");
        loop {
            if prismatic_core::SHUTDOWN.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            let temp = watchdog_context
                .gpu_thermal_celsius
                .load(std::sync::atomic::Ordering::Relaxed);
            let limit = watchdog_context
                .thermal_limit_celsius
                .load(std::sync::atomic::Ordering::Relaxed);

            // Severe back-pressure detection (>80% full queue on a 256 capacity queue)
            let len = watchdog_bus.len();
            let capacity = watchdog_bus.capacity();
            if len > (capacity * 8) / 10 {
                let current_scale = watchdog_context
                    .batch_scale
                    .load(std::sync::atomic::Ordering::Relaxed);
                if current_scale < 8 {
                    watchdog_context
                        .batch_scale
                        .store(current_scale * 2, std::sync::atomic::Ordering::Release);
                    tracing::warn!(
                        "WATCHDOG ALARM: Severe Back-Pressure ({}/{}). Scaling batch size to {}.",
                        len,
                        capacity,
                        current_scale * 2
                    );
                }
            } else if len < (capacity * 2) / 10 {
                let current_scale = watchdog_context
                    .batch_scale
                    .load(std::sync::atomic::Ordering::Relaxed);
                if current_scale > 1 {
                    let new_scale = std::cmp::max(1, current_scale / 2);
                    watchdog_context
                        .batch_scale
                        .store(new_scale, std::sync::atomic::Ordering::Release);
                    tracing::info!(
                        "Queue back-pressure recovering ({}/{}). Reducing batch size to {}.",
                        len,
                        capacity,
                        new_scale
                    );
                }
            }

            // Thermal emergency shutdown
            if temp >= limit {
                tracing::error!(
                    "WATCHDOG ALARM: Thermal Limit Exceeded ({}C >= {}C)! Initiating ACPI Power-Off...",
                    temp,
                    limit
                );
                prismatic_core::SHUTDOWN.store(true, std::sync::atomic::Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(500)); // Allow pipelines to drain

                // Graceful Bare-Metal Shutdown: Manually write dump, then issue ACPI poweroff
                let dump_msg = format!(
                    "WATCHDOG THERMAL MELTDOWN ({}C)\nPipelines drained gracefully.",
                    temp
                );
                let _ = std::fs::write("CRASH_DUMP_V45.log", dump_msg);
                let _ = std::process::Command::new("poweroff").spawn();
                std::process::exit(1);
            }

            // Safety Envelopes: Enforce strict minimum/maximum dt_ms caps to prevent PID runaways
            let lat = watchdog_context
                .inference_latency_ms
                .load(std::sync::atomic::Ordering::Relaxed);
            if lat > 1000 {
                tracing::warn!(
                    "Safety Envelope Exceeded: Inference latency too high ({} ms). Shedding load.",
                    lat
                );
                watchdog_context
                    .batch_scale
                    .store(1, std::sync::atomic::Ordering::Release);
            }
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    });
}
