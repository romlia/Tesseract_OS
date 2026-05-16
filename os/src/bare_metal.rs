use prismatic_acoustics::run_cpal_gradient_loop;
use prismatic_core::{GlobalContext, SensoryEvent, temporal};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;
use tokenizers::Tokenizer;

pub fn run_bare_metal(
    state: Arc<GlobalContext>,
    bus: Arc<dyn prismatic_core::bus::EventBus<SensoryEvent>>,
) {
    tracing::info!("Initializing Bare-Metal DRM/KMS Fallback Framebuffer...");

    // 1. Initialize Headless WebGPU via HolographicManifold.
    // Any DRM init failure (EACCES from a held master, EBUSY, the
    // downstream "no active display mode found" that masks EBUSY,
    // etc.) falls back to headless instead of panicking — bare-metal
    // is best effort, the host may be running a compositor.
    let manifold = match pollster::block_on(
        prismatic_glass::drm_matrix::HolographicManifold::ignite_bare_metal(),
    ) {
        Ok(m) => m,
        Err(e) => {
            let msg = format!("{:?}", e);
            let _ = std::fs::write("WGPU_CRASH.log", msg.clone());
            tracing::warn!(
                "DRM init failed ({}). Bare-metal requires a free TTY \
                 (Ctrl+Alt+F3, display manager stopped). \
                 Falling back to headless for this session.",
                msg.lines().next().unwrap_or("unknown error")
            );
            crate::headless::run_headless(state, bus);
            return;
        }
    };
    let device = manifold.device;
    let queue = manifold.queue;
    let _adapter = manifold.adapter;

    // Display configuration
    let (width, height) = manifold.display_mode.size();
    let width = width as u32;
    let height = height as u32;

    let card = prismatic_glass::drm_matrix::DrmCard(manifold.drm_node.try_clone().unwrap());
    use rust_drm::control::Device as ControlDevice;

    let mut dumb_buffer = card
        .create_dumb_buffer((width, height), rust_drm::buffer::DrmFourcc::Xrgb8888, 32)
        .unwrap();
    let fb = card.add_framebuffer(&dumb_buffer, 24, 32).unwrap();
    card.set_crtc(
        manifold.crtc,
        Some(fb),
        (0, 0),
        &[manifold.connector],
        Some(manifold.display_mode),
    )
    .unwrap();

    let render_target = device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        label: None,
        view_formats: &[],
    });
    let render_view = render_target.create_view(&wgpu::TextureViewDescriptor::default());

    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Readback"),
        size: (width * height * 4) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // 2. Setup Hologram Compute Pipeline
    let hologram_src = include_str!("../../glass/src/hologram.wgsl");
    let hologram_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("SDF Hologram Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(hologram_src)),
    });

    let fb_size = 1920 * 1080 * 4;

    let framebuffer_gpu = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Framebuffer GPU Storage"),
        size: fb_size as u64,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::UNIFORM,
        mapped_at_creation: false,
    });

    let heat_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Hallucination Heat Buffer"),
        size: (prismatic_core::LILITH_CONFIG.hidden_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let engine_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Engine Uniform Buffer"),
        size: 32,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let hologram_bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Hologram Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let glyph_buffer_gpu = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Glyph Buffer GPU Storage"),
        size: fb_size as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let hologram_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Hologram Bind Group"),
        layout: &hologram_bg_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: heat_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: engine_uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: framebuffer_gpu.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: glyph_buffer_gpu.as_entire_binding(),
            },
        ],
    });

    let hologram_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Hologram Pipeline Layout"),
        bind_group_layouts: &[Some(&hologram_bg_layout)],
        immediate_size: 0,
    });

    let hologram_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Hologram Compute Pipeline"),
        layout: Some(&hologram_pipeline_layout),
        module: &hologram_shader,
        entry_point: Some("cs_main"),
        compilation_options: Default::default(),
        cache: None,
    });

    // Presentation Pipeline
    let present_shader_src = "
    @group(0) @binding(0) var<storage, read> compute_framebuffer: array<u32>;
    struct VertexOutput {
        @builtin(position) clip_position: vec4<f32>,
        @location(0) uv: vec2<f32>,
    };
    @vertex fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
        var out: VertexOutput;
        let x = f32((in_vertex_index << 1u) & 2u);
        let y = f32(in_vertex_index & 2u);
        out.clip_position = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
        out.uv = vec2<f32>(x, y);
        return out;
    }
    @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
        let x = u32(in.uv.x * 1920.0);
        let y = u32((1.0 - in.uv.y) * 1080.0);
        let idx = y * 1920u + x;
        let pixel = compute_framebuffer[idx];
        let r = f32((pixel >> 16u) & 255u) / 255.0;
        let g = f32((pixel >> 8u) & 255u) / 255.0;
        let b = f32(pixel & 255u) / 255.0;
        return vec4<f32>(r, g, b, 1.0);
    }
    ";
    let present_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Present Shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(present_shader_src)),
    });

    let present_bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Present BG Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let present_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Present BG"),
        layout: &present_bg_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: framebuffer_gpu.as_entire_binding(),
        }],
    });

    let present_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Present Pipeline Layout"),
        bind_group_layouts: &[Some(&present_bg_layout)],
        immediate_size: 0,
    });

    let present_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Present Pipeline"),
        layout: Some(&present_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &present_shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &present_shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    // 3. Initialize Background Sensory Engines
    let tx_audio = bus.clone();
    let state_audio = state.clone();
    std::thread::spawn(move || {
        run_cpal_gradient_loop(tx_audio, state_audio);
    });

    let tokenizer = Tokenizer::from_file("weights/tokenizer.json").unwrap();
    crate::kestrel::spawn_optic_nerve(bus.clone(), tokenizer.clone());
    crate::mesh::spawn_nebula_shadow_node(state.clone());

    let state_tess = state.clone();
    let rx_tess = bus.clone();
    std::thread::spawn(move || {
        temporal::run_continuous_loop(rx_tess, state_tess);
    });

    tracing::info!("=== V45 Prismatic OS Bare-Metal Compute Loop Ready ===");

    let mut html_parser = crate::html_parser::Html2Parser::new();
    let mut pretext = crate::pretext_layout::PretextLayoutEngine::new(1920, 1080);

    let mut zero_trust = crate::zero_trust::ZeroTrustLedger::new();
    let mut needs_glyph_sync = false;
    let mut idle_lerp: f32 = 1.0;
    let boot_time = Instant::now();

    loop {
        if prismatic_core::SHUTDOWN.load(Ordering::Relaxed) {
            tracing::info!("Bare-Metal main loop shutting down...");
            break;
        }

        // Fixed 60FPS UI delta time
        zero_trust.tick_ebbinghaus_decay(16.6);

        let mut drained = Vec::new();
        while let Some(token) = state.vocal_chord.pop() {
            drained.push(token);
        }
        if !drained.is_empty()
            && let Ok(text) = tokenizer.decode(&drained, false)
        {
            print!("{}", text);
            use std::io::Write;
            std::io::stdout().flush().unwrap();

            let blocks = html_parser.parse(&text);
            if !blocks.is_empty() {
                pretext.layout_blocks(&blocks);
                needs_glyph_sync = true;
            }
        }

        // Sync glyphs to GPU
        if needs_glyph_sync {
            queue.write_buffer(
                &glyph_buffer_gpu,
                0,
                bytemuck::cast_slice(&pretext.glyph_buffer),
            );
            needs_glyph_sync = false;
        }

        // Sync heat
        let heat = f32::from_bits(
            state
                .gpu_thermal_celsius
                .load(std::sync::atomic::Ordering::Relaxed),
        );
        let heat_arr = [heat, heat, heat];
        queue.write_buffer(&heat_buffer, 0, bytemuck::cast_slice(&heat_arr));

        // Idle logic
        let target_idle = if pretext.is_empty { 1.0 } else { 0.0 };
        let lerp_speed = if target_idle > 0.5 { 0.01 } else { 0.1 };
        idle_lerp = idle_lerp * (1.0 - lerp_speed) + target_idle * lerp_speed;

        let time = boot_time.elapsed().as_secs_f32();

        // Sync uniforms
        if let Ok(cam) = state.camera_pos.try_lock() {
            let audio_hz = f32::from_bits(
                state
                    .audio_oscillator_hz
                    .load(std::sync::atomic::Ordering::Relaxed),
            );
            let uniforms: [f32; 8] = [cam[0], cam[1], cam[2], 0.0, audio_hz, idle_lerp, time, 0.0];
            queue.write_buffer(&engine_uniform_buffer, 0, bytemuck::cast_slice(&uniforms));
        }

        // 1. Compute Pass
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            cpass.set_pipeline(&hologram_pipeline);
            cpass.set_bind_group(0, &hologram_bg, &[]);
            cpass.dispatch_workgroups(1920 / 16, 1080 / 16, 1);
        }

        // 2. Presentation Render Pass (Headless)
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &render_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rpass.set_pipeline(&present_pipeline);
            rpass.set_bind_group(0, &present_bg, &[]);
            rpass.draw(0..3, 0..1);
        }

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &render_target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let submission_index = queue.submit(Some(encoder.finish()));

        let buffer_slice = readback_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission_index),
                timeout: None,
            })
            .unwrap();

        if let Ok(Ok(())) = rx.recv() {
            let data = buffer_slice.get_mapped_range();
            let mut mapping = card.map_dumb_buffer(&mut dumb_buffer).unwrap();
            mapping.as_mut().copy_from_slice(&data);
            drop(data);
            readback_buffer.unmap();

            let _ = card.page_flip(
                manifold.crtc,
                fb,
                rust_drm::control::PageFlipFlags::EVENT,
                None,
            );
            if let Ok(events) = card.receive_events() {
                for _event in events {
                    // Block until VBlank Event triggers
                }
            }
        }
    }

    tracing::info!("Bare-Metal DRM/KMS Framebuffer shutdown complete. Session released.");
}
