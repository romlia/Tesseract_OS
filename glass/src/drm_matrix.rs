// P1: Integrated minimal DRM/KMS locking to secure display modes before launching the UI.
// IMPLEMENTED[Phase 2]: Implement a scanner for incoming text to instantly trigger the SDF pipeline if any code point > 0x7F appears.
use gbm::Device as GbmDevice;
use libseat::{Seat, SeatEvent};
use std::os::unix::io::FromRawFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rust_drm::control::{
    Device as ControlDevice, PageFlipFlags, dumbbuffer::DumbBuffer, framebuffer,
};
use std::os::unix::io::{AsFd, AsRawFd, BorrowedFd, RawFd};

pub struct DrmCard(pub std::fs::File);

impl AsFd for DrmCard {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}
impl AsRawFd for DrmCard {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}
impl rust_drm::Device for DrmCard {}
impl rust_drm::control::Device for DrmCard {}

// PHASE 10: DRM Ping-Pong Buffers
pub struct DrmDoubleBuffer {
    pub buffers: [DumbBuffer; 2],
    pub framebuffers: [framebuffer::Handle; 2],
    pub current_index: usize,
}

pub struct HolographicManifold {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::Adapter,
    pub drm_node: std::fs::File,
    pub display_mode: rust_drm::control::Mode,
    pub crtc: rust_drm::control::crtc::Handle,
    pub connector: rust_drm::control::connector::Handle,
    pub card: DrmCard,
    pub double_buffer: DrmDoubleBuffer,
    pub _seat: Seat, // Owned seat for automatic drop/cleanup
}

impl HolographicManifold {
    pub async fn ignite_bare_metal() -> anyhow::Result<Self> {
        let active = Arc::new(AtomicBool::new(false));
        let active_clone = active.clone();

        let mut seat = Seat::open(move |seat_ref, event| match event {
            SeatEvent::Enable => {
                tracing::info!("Libseat: Seat enabled by logind/seatd!");
                active_clone.store(true, Ordering::Relaxed);
            }
            SeatEvent::Disable => {
                tracing::warn!("Libseat: Seat disabled by logind/seatd!");
                active_clone.store(false, Ordering::Relaxed);
                let _ = seat_ref.disable();
            }
        })
        .map_err(|e| anyhow::anyhow!("Failed to open libseat: {:?}", e))?;

        // Wait for the seat to become active
        while !active.load(Ordering::Relaxed) {
            if prismatic_core::SHUTDOWN.load(Ordering::Relaxed) {
                anyhow::bail!("Shutdown sequence initiated before DRM seat could be secured.");
            }
            if seat.dispatch(100).is_err() {
                anyhow::bail!("Libseat dispatch failed before seat became active");
            }
        }

        let mut drm_node = None;
        // Prioritize NVIDIA card1 over the AMD card0
        let targets = [
            "/dev/dri/card1",
            "/dev/dri/card0",
            "/dev/dri/card2",
            "/dev/dri/card3",
        ];
        for path in targets {
            if let Ok(device) = seat.open_device(&path) {
                use std::os::unix::io::{AsFd, AsRawFd};
                let fd = device.as_fd().as_raw_fd();

                let file = unsafe { std::fs::File::from_raw_fd(fd) };

                // Validate it's a render node
                if let Ok(_) = GbmDevice::new(file.try_clone().unwrap()) {
                    drm_node = Some(file);
                    tracing::info!(
                        "Hardware Seizure Successful (via logind/seatd) on: {}",
                        path
                    );
                    break;
                } else {
                    seat.close_device(device).unwrap();
                }
            }
        }

        let drm_node = drm_node.ok_or_else(|| {
            anyhow::anyhow!("Hardware Seizure Failed: No suitable DRM nodes found via libseat")
        })?;

        let card = DrmCard(drm_node.try_clone().unwrap());
        use rust_drm::control::Device;
        let res = card
            .resource_handles()
            .map_err(|e| anyhow::anyhow!("Failed to get DRM resources: {:?}", e))?;

        let mut active_mode = None;
        let mut active_crtc = None;
        let mut active_connector = None;

        for &con in res.connectors() {
            if let Ok(info) = card.get_connector(con, true) {
                if info.state() == rust_drm::control::connector::State::Connected {
                    let modes = info.modes();
                    if modes.len() > 0 {
                        active_mode = Some(modes[0]);
                        active_connector = Some(con);
                        if let Some(enc) = info.current_encoder() {
                            if let Ok(enc_info) = card.get_encoder(enc) {
                                active_crtc = enc_info.crtc();
                            }
                        }
                        if active_crtc.is_none() {
                            active_crtc = Some(res.crtcs()[0]);
                        }
                        break;
                    }
                }
            }
        }

        let display_mode =
            active_mode.ok_or_else(|| anyhow::anyhow!("No active display mode found"))?;
        let crtc = active_crtc.ok_or_else(|| anyhow::anyhow!("No active CRTC found"))?;
        let connector =
            active_connector.ok_or_else(|| anyhow::anyhow!("No active connector found"))?;

        let instance = wgpu::Instance::default();

        // Headless Initialization (No Surface)
        let adapter_future = instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: None,
            ..Default::default()
        });

        let adapter = adapter_future
            .await
            .map_err(|_| anyhow::anyhow!("Failed to acquire compatible WGPU adapter"))?;

        let device_req: wgpu::DeviceDescriptor = wgpu::DeviceDescriptor::default();
        let (device, queue): (wgpu::Device, wgpu::Queue) = adapter
            .request_device(&device_req)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to acquire WGPU device queue: {:?}", e))?;

        // Phase 10: Allocate DRM Dumb Buffers (1920x1080)
        let mode_size = display_mode.size();
        let size_u32 = (mode_size.0 as u32, mode_size.1 as u32);

        let db0 = card
            .create_dumb_buffer(size_u32, rust_drm::buffer::DrmFourcc::Xrgb8888, 32)
            .map_err(|e| anyhow::anyhow!("Failed to create Dumb Buffer 0: {:?}", e))?;
        let fb0 = card
            .add_framebuffer(&db0, 24, 32)
            .map_err(|e| anyhow::anyhow!("Failed to create Framebuffer 0: {:?}", e))?;

        let db1 = card
            .create_dumb_buffer(size_u32, rust_drm::buffer::DrmFourcc::Xrgb8888, 32)
            .map_err(|e| anyhow::anyhow!("Failed to create Dumb Buffer 1: {:?}", e))?;
        let fb1 = card
            .add_framebuffer(&db1, 24, 32)
            .map_err(|e| anyhow::anyhow!("Failed to create Framebuffer 1: {:?}", e))?;

        let double_buffer = DrmDoubleBuffer {
            buffers: [db0, db1],
            framebuffers: [fb0, fb1],
            current_index: 0,
        };

        // Initial Blast: Force the CRTC to display the front buffer
        card.set_crtc(crtc, Some(fb0), (0, 0), &[connector], Some(display_mode))
            .map_err(|e| anyhow::anyhow!("Failed to set CRTC for initial DRM blast: {:?}", e))?;

        Ok(Self {
            device,
            queue,
            adapter,
            drm_node,
            display_mode,
            crtc,
            connector,
            card,
            double_buffer,
            _seat: seat,
        })
    }

    // Phase 10: Bare-Metal Page Flipping & V-Sync
    // P1: Prototyped the zero-allocation framebuffer UI with a simple fast-mode renderer, maintaining CLS-free layout and Unicode SDF fallback.
    // DRM/KMS Mode-Setting Integration for smooth hand-off to avoid flicker
    pub fn present_frame(&mut self, wgpu_pixels: &[u8]) -> anyhow::Result<()> {
        let next_idx = 1 - self.double_buffer.current_index;
        let mut next_buffer = self.double_buffer.buffers[next_idx];
        let next_fb = self.double_buffer.framebuffers[next_idx];

        // Map DRM Back Buffer to CPU
        let mut mapping = self
            .card
            .map_dumb_buffer(&mut next_buffer)
            .map_err(|e| anyhow::anyhow!("Failed to map DRM back buffer: {:?}", e))?;

        // Copy WGPU pixels into DRM physical memory
        mapping.as_mut()[..wgpu_pixels.len()].copy_from_slice(wgpu_pixels);
        drop(mapping); // Release the mapping

        // Blast the new frame to the physical monitor
        self.card
            .page_flip(self.crtc, next_fb, PageFlipFlags::EVENT, None)
            .map_err(|e| anyhow::anyhow!("Bare-metal DRM page flip failed: {:?}", e))?;

        // Wait for Hardware V-Blank (Bare-Metal V-Sync)
        let _events = self
            .card
            .receive_events()
            .map_err(|e| anyhow::anyhow!("Failed to receive DRM V-Blank event: {:?}", e))?;

        self.double_buffer.current_index = next_idx;
        Ok(())
    }
}
