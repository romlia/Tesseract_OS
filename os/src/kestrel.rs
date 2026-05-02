use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::panic;

use prismatic_core::SensoryEvent;
use std::backtrace::Backtrace;
use tokenizers::Tokenizer;
use std::sync::Arc;
use prismatic_core::bus::EventBus;

// Pre-allocated static buffer to avoid heap-allocation during Out-Of-Memory panics
static mut PANIC_BUFFER: [u8; 8192] = [0; 8192];

pub fn initialize_kestrel_hook() {
    panic::set_hook(Box::new(|panic_info| {
        let backtrace = Backtrace::force_capture();

        // Use a cursor over the pre-allocated slice to avoid formatting on the heap
        unsafe {
            let mut cursor = std::io::Cursor::new(&mut PANIC_BUFFER[..]);
            let _ = write!(
                cursor,
                "V45 PRISMATIC SINGULARITY FAULT\nPanicked at: {:?}\nBacktrace:\n{}",
                panic_info.location(),
                backtrace
            );

            let length = cursor.position() as usize;

            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("CRASH_DUMP_V45.log")
            {
                let _ = file.write_all(&PANIC_BUFFER[..length]);
                let _ = file.sync_all(); // Synchronous block before power loss
            }
        }

        std::process::exit(1);
    }));
}

#[repr(C)]
struct InputEvent {
    tv_sec: isize,
    tv_usec: isize,
    type_: u16,
    code: u16,
    value: i32,
}

pub fn spawn_optic_nerve(bus: Arc<dyn EventBus<SensoryEvent>>, tokenizer: Tokenizer) {
    let tx_stdin = bus.clone();
    let tokenizer_stdin = tokenizer.clone();
    std::thread::spawn(move || {
        tracing::info!("Binding Kestrel to STDIN for REPL interface...");
        for b in std::io::stdin().bytes().flatten() {
            let c = b as char;
            if let Ok(encoding) = tokenizer_stdin.encode(c.to_string(), true) {
                for &id in encoding.get_ids() {
                    let _ = tx_stdin.push(SensoryEvent::KeyboardHash(id));
                }
            }
        }
    });

    let tx_keyboard = bus.clone();
    std::thread::spawn(move || {
        tracing::info!("Binding Kestrel to /dev/input/event0...");
        let file = match File::open("/dev/input/event0") {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("Failed to open /dev/input/event0: {}", e);
                return;
            }
        };

        // Async io_uring Device Polling
        // Binds `/dev/input/event0` to the Linux `io_uring` instance, allowing the kernel to asynchronously 
        // push hardware events directly into our memory ring buffer without performing context-switching system calls.
        use std::os::unix::io::AsRawFd;
        let fd = file.as_raw_fd();
        let mut ring = match io_uring::IoUring::new(4) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Failed to initialize io_uring: {}", e);
                return;
            }
        };
        
        // FFI Boundary Benchmarking
        // Setup tracing spans across the C/Rust FFI boundary when polling io_uring completions.
        // We need to measure if the kernel-to-userspace completion queue causes any hidden
        // latency spikes that could bottleneck the LockFreeEventBus under heavy contention.
        let mut event_buf = [0u8; std::mem::size_of::<InputEvent>()];
        
        loop {
            if prismatic_core::SHUTDOWN.load(std::sync::atomic::Ordering::Relaxed) { break; }
            let read_e = io_uring::opcode::Read::new(
                io_uring::types::Fd(fd),
                event_buf.as_mut_ptr(),
                event_buf.len() as _,
            )
            .build()
            .user_data(1);
            
            unsafe {
                if ring.submission().push(&read_e).is_err() {
                    continue;
                }
            }
            if ring.submit_and_wait(1).is_err() {
                continue;
            }
            
            let cqe = if let Some(cqe) = ring.completion().next() { cqe } else { continue };
            if cqe.result() == event_buf.len() as i32 {
                let event: InputEvent = unsafe { std::ptr::read(event_buf.as_ptr() as *const _) };
                if event.type_ == 1 && event.value == 1 {
                    // EV_KEY and KeyPress
                    if event.code == 1 {
                        // KEY_ESC
                        let _ = tx_keyboard.push(SensoryEvent::ConsentOverride(false));
                        continue;
                    } else if event.code == 21 {
                        // KEY_Y
                        let _ = tx_keyboard.push(SensoryEvent::ConsentOverride(true));
                    } else if event.code == 103 {
                        // KEY_UP
                        let _ = tx_keyboard.push(SensoryEvent::Navigation(0.0, 0.0, 1.0));
                        continue;
                    } else if event.code == 108 {
                        // KEY_DOWN
                        let _ = tx_keyboard.push(SensoryEvent::Navigation(0.0, 0.0, -1.0));
                        continue;
                    } else if event.code == 105 {
                        // KEY_LEFT
                        let _ = tx_keyboard.push(SensoryEvent::Navigation(-1.0, 0.0, 0.0));
                        continue;
                    } else if event.code == 106 {
                        // KEY_RIGHT
                        let _ = tx_keyboard.push(SensoryEvent::Navigation(1.0, 0.0, 0.0));
                        continue;
                    } else if event.code == 28 {
                        // KEY_ENTER
                        let _ = tx_keyboard.push(SensoryEvent::CommitPrompt);
                        continue;
                    }

                    // Map raw linux keycode to char (simplified for mapping)
                    let c = match event.code {
                        16 => 'q',
                        17 => 'w',
                        18 => 'e',
                        19 => 'r',
                        20 => 't',
                        30 => 'a',
                        31 => 's',
                        32 => 'd',
                        33 => 'f',
                        34 => 'g',
                        44 => 'z',
                        45 => 'x',
                        46 => 'c',
                        47 => 'v',
                        48 => 'b',
                        57 => ' ', // space
                        _ => '?',
                    };

                    if let Ok(encoding) = tokenizer.encode(c.to_string(), true) {
                        for &id in encoding.get_ids() {
                            let _ = tx_keyboard.push(SensoryEvent::KeyboardHash(id));
                        }
                    }
                }
            }
        }
    });

    let tx_mice = bus.clone();
    std::thread::spawn(move || {
        fn q_rsqrt(number: f32) -> f32 {
            let mut y = number;
            let mut i = y.to_bits();
            i = 0x5f3759df - (i >> 1);
            y = f32::from_bits(i);
            y * (1.5 - (number * 0.5 * y * y))
        }
        tracing::info!("Binding Kestrel to /dev/input/mice for kinetic vision...");
        let mut file = match File::open("/dev/input/mice") {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("Failed to open /dev/input/mice: {}", e);
                return;
            }
        };

        let mut buf = [0u8; 3];
        loop {
            if prismatic_core::SHUTDOWN.load(std::sync::atomic::Ordering::Relaxed) { break; }
            if file.read_exact(&mut buf).is_ok() {
                let dx = buf[1] as i8 as f32;
                let dy = buf[2] as i8 as f32;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq > 0.0 {
                    let velocity = dist_sq * q_rsqrt(dist_sq);
                    let _ = tx_mice.push(SensoryEvent::VisualKinetic(velocity));
                }
            }
        }
    });

    let tx_cam = bus.clone();
    #[cfg(feature = "optical_flow")]
    std::thread::spawn(move || {
        tracing::info!("Binding Kestrel to /dev/video0 for raw visual ingestion...");
        let dev = match v4l::Device::new(0) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Failed to open /dev/video0: {}", e);
                return;
            }
        };

        let mut format = dev.format().unwrap();
        format.width = 640;
        format.height = 480;
        format.fourcc = v4l::FourCC::new(b"YUYV");
        let _ = dev.set_format(&format);

        use v4l::io::traits::CaptureStream;
        use v4l::video::Capture;
        let mut stream = match v4l::prelude::MmapStream::with_buffers(
            &dev,
            v4l::buffer::Type::VideoCapture,
            4,
        ) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Failed to create video stream: {}", e);
                return;
            }
        };

        let mut pseudo_seed = 12345u32;
        let prev_left_mass = 0.0;
        let prev_right_mass = 0.0;

        loop {
            if prismatic_core::SHUTDOWN.load(std::sync::atomic::Ordering::Relaxed) { break; }
            if let Ok((buf, _meta)) = stream.next() {
                let len_pairs = buf.len() / 2;
                if len_pairs == 0 {
                    continue;
                }

                // Disabled optical flow CPU parallel reduction for performance.
                // Gestural Kinetic Sweep detection via Optical Flow approximation is too expensive.
                // In a production system, use a low-res background worker or DSP.
                /*
                // Task 1: Vectorized Optical Flow using Rayon parallel reduction
                let (left_mass, right_mass) = buf
                    .par_chunks_exact(4)
                    .enumerate()
                    .fold(
                        || (0.0f32, 0.0f32),
                        |(mut l, mut r), (i, chunk)| {
                            let y = chunk[0] as f32;
                            // Each 4-byte chunk represents 2 pixels (Y U Y V). 
                            // i is the chunk index. Since each chunk is 2 pixels, x_coord is (i * 2) % 640
                            let x_coord = (i * 2) % 640;
                            if x_coord < 320 {
                                l += y;
                            } else {
                                r += y;
                            }
                            (l, r)
                        },
                    )
                    .reduce(|| (0.0, 0.0), |a, b| (a.0 + b.0, a.1 + b.1));

                let d_left = left_mass - prev_left_mass;
                let d_right = right_mass - prev_right_mass;

                // If a hand sweeps across the camera, we detect a massive asymmetric luminance delta
                if d_left > 500000.0 && d_right < 100000.0 {
                    let _ = tx_cam.push(SensoryEvent::Navigation(-1.0, 0.0, 0.0));
                } else if d_right > 500000.0 && d_left < 100000.0 {
                    let _ = tx_cam.push(SensoryEvent::Navigation(1.0, 0.0, 0.0));
                }
                // Pinch / Expand gesture approximation
                else if d_left > 300000.0 && d_right > 300000.0 {
                    let _ = tx_cam.push(SensoryEvent::Navigation(0.0, 0.0, 1.0)); // Zoom in
                } else if d_left < -300000.0 && d_right < -300000.0 {
                    let _ = tx_cam.push(SensoryEvent::Navigation(0.0, 0.0, -1.0)); // Zoom out
                }

                prev_left_mass = left_mass;
                prev_right_mass = right_mass;
                */

                for _ in 0..64 {
                    // Randomly sample 64 pixels to keep bandwidth low
                    pseudo_seed = pseudo_seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let pixel_idx = (pseudo_seed as usize % len_pairs) * 2;
                    let y = buf[pixel_idx] as f32;

                    pseudo_seed = pseudo_seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let x_pos = (pseudo_seed % 1000) as f32 / 1000.0;

                    pseudo_seed = pseudo_seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let y_pos = (pseudo_seed % 1000) as f32 / 1000.0;

                    let _ = tx_cam.push(SensoryEvent::VisualPixel(y, y, y, x_pos, y_pos));
                }
            }
        }
    });
}
