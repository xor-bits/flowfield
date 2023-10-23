use std::{
    borrow::Cow,
    fs,
    io::{BufRead, BufReader},
    mem::size_of,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::Instant,
};

use anyhow::{anyhow, Result};
use glam::{Mat4, Vec2};
use rand::Rng;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{settings::GlobalSettings, RuntimeSettings};

use self::surface::{Surface, SurfaceBuilder};

use bytemuck::{Pod, Zeroable};

//

pub mod surface;

//

pub struct Graphics {
    device: Arc<Device>,
    queue: Queue,
    surface: Surface,

    boot: Instant,
    value: f32,

    #[allow(unused)]
    limits: Limits,

    last_flags: u32,
    size: (u32, u32),

    pub cursor: Vec2,

    points: Buffer,
    points_len: u32,

    blit_sampler: Sampler,
    blit_bind_group_layout: BindGroupLayout,
    blit_bind_group: BindGroup,
    blit_pipeline: RenderPipeline,

    shadow_bind_group_layout: BindGroupLayout,
    shadow_bind_group: BindGroup,
    shadow_pipeline: ComputePipeline,
    update_bind_group_layout: BindGroupLayout,
    update_bind_group: BindGroup,
    update_pipeline: ComputePipeline,
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct DrawPush {
    mvp: Mat4,
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct UpdatePush {
    cursor: Vec2,
    time: f32,
    flags: u32,
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct ShadowPush {
    flags: u32,
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct BlitPush {
    flags: u32,
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Vertex {
    // col: Vec4,
    // pos: Vec2,
    // _pad: Vec2,
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Instance {
    pos: Vec2,
    vel: Vec2,
}

//

impl Graphics {
    pub async fn init(settings: &GlobalSettings, window: Arc<Window>) -> Result<Self> {
        let s = &settings.graphics;

        let instance = Arc::new(wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: s.allowed_backends.to_backends(),
            ..<_>::default()
        }));

        #[cfg(not(target_family = "wasm"))]
        {
            let inst = instance.clone();
            thread::spawn(move || {
                inst.poll_all(true);
            });
        }

        let PhysicalSize { width, height } = window.inner_size();
        let surface_builder = SurfaceBuilder::new(instance.clone(), window)?;

        let gpu = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: s.gpu_preference.to_power_preference(),
                force_fallback_adapter: s.force_software_rendering,
                compatible_surface: Some(&surface_builder.surface),
            })
            .await
            .ok_or_else(|| anyhow!("Could not find a suitable GPU"))?;

        /* let features = Features::POLYGON_MODE_LINE | Features::PUSH_CONSTANTS;
        let limits = Limits {
            max_texture_dimension_2d: 128,
            max_push_constant_size: core::mem::size_of::<Push>() as u32,
            ..Limits::downlevel_defaults()
        }; */
        let features = gpu.features();
        let limits = gpu.limits();

        let (device, queue) = gpu
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    features,
                    limits: limits.clone(),
                },
                None,
            )
            .await?;
        let device = Arc::new(device);

        let surface = surface_builder.build(s, &gpu, device.clone());

        let module =
            Self::load_shader_module("./asset/shader.wgsl").expect("failed to read the shader");
        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(Cow::from(module)),
        });

        let update_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::ReadWrite,
                            format: TextureFormat::R32Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let update_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&update_bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..size_of::<UpdatePush>() as u32,
            }],
        });

        let update_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("update pipeline"),
            layout: Some(&update_layout),
            module: &module,
            entry_point: "cs_main_update",
        });

        let shadow_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: TextureFormat::R32Float,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                }],
            });

        let shadow_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&shadow_bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..size_of::<ShadowPush>() as u32,
            }],
        });

        let shadow_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("shadow pipeline"),
            layout: Some(&shadow_layout),
            module: &module,
            entry_point: "cs_main_shadow",
        });

        let mut rng = rand::thread_rng();
        let points_len = 100_000;
        let points: Vec<_> = (0..points_len)
            .map(|_| Instance {
                pos: Vec2::new(rng.gen(), rng.gen()) * 4.0 - 2.0,
                vel: Vec2::new(rng.gen(), rng.gen()) * 0.001 - 0.0005,
            })
            .collect();
        let points = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&points[..]),
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
        });

        /* let update_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &update_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &points,
                    offset: 0,
                    size: None,
                }),
            }],
        }); */

        let module =
            Self::load_shader_module("./asset/blit.wgsl").expect("failed to read the shader");
        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(Cow::from(module)),
        });

        let blit_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let blit_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            // bind_group_layouts: &[],
            bind_group_layouts: &[&blit_bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..std::mem::size_of::<BlitPush>() as u32,
            }],
        });

        let blit_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("blit pipeline"),
            layout: Some(&blit_layout),
            vertex: VertexState {
                module: &module,
                entry_point: "vs_main",
                buffers: &[],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: <_>::default(),
            fragment: Some(FragmentState {
                module: &module,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: surface.format(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        let blit_sampler = device.create_sampler(&SamplerDescriptor {
            label: None,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            // mag_filter: FilterMode::Linear,
            // min_filter: FilterMode::Linear,
            // mipmap_filter: FilterMode::Linear,
            compare: None,
            anisotropy_clamp: 1,
            border_color: Some(SamplerBorderColor::OpaqueBlack),

            ..<_>::default()
        });

        let (blit_bind_group, shadow_bind_group, update_bind_group) = Self::create_bind_groups(
            &device,
            &limits,
            &blit_sampler,
            &points,
            &blit_bind_group_layout,
            &shadow_bind_group_layout,
            &update_bind_group_layout,
            (width, height),
        );

        Ok(Self {
            device,
            queue,
            surface,

            boot: Instant::now(),
            value: 0.0,

            limits,

            last_flags: 0,
            size: (width, height),

            cursor: Vec2::new(-100.0, -100.0),

            points,
            points_len,

            blit_sampler,
            blit_bind_group_layout,
            blit_bind_group,
            blit_pipeline,

            shadow_bind_group_layout,
            shadow_bind_group,
            shadow_pipeline,
            update_bind_group_layout,
            update_bind_group,
            update_pipeline,
        })
    }

    fn create_bind_groups(
        device: &Device,
        limits: &Limits,
        sampler: &Sampler,
        points: &Buffer,
        blit_bind_layout: &BindGroupLayout,
        shadow_bind_layout: &BindGroupLayout,
        update_bind_layout: &BindGroupLayout,
        (mut width, mut height): (u32, u32),
    ) -> (BindGroup, BindGroup, BindGroup) {
        // width /= 2;
        // height /= 2;
        width = width.min(limits.max_texture_dimension_2d);
        height = height.min(limits.max_texture_dimension_2d);

        let target = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R32Float,
            usage: TextureUsages::STORAGE_BINDING
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let target_view = target.create_view(&TextureViewDescriptor { ..<_>::default() });

        let blit_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("blit bind group"),
            layout: blit_bind_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&target_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        });

        let shadow_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("shadow bind group"),
            layout: shadow_bind_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&target_view),
            }],
        });

        let update_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("update bind group"),
            layout: update_bind_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&target_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: points,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        (blit_bind_group, shadow_bind_group, update_bind_group)
        // (blit_bind_group, draw_bind_group)
    }

    fn load_shader_module(path: &str) -> anyhow::Result<String> {
        let mut src = String::new();
        Self::load_shader_module_into(&mut src, path.as_ref())?;
        Ok(src)
    }

    fn load_shader_module_into(into: &mut String, path: &Path) -> anyhow::Result<()> {
        let path = PathBuf::from(path);
        let file = fs::OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open(&path)?;

        let parent = path.parent().ok_or_else(|| anyhow!("what"))?;

        for line in BufReader::new(file).lines() {
            let line = line?;

            if line.starts_with("//!include") {
                let mut split = line.split('"');
                let _include = split.next().unwrap();

                while let Some(path) = split.next() {
                    Self::load_shader_module_into(into, &parent.join(path))?;
                    let _separator = split
                        .next()
                        .ok_or_else(|| anyhow!("unexpected end of line"))?;
                }
            } else {
                into.push_str(&line);
                into.push('\n');
            }
        }

        Ok(())
    }

    pub fn scrolled(&mut self, delta: (f32, f32)) {
        self.value += delta.0 + delta.1;
        tracing::debug!("value: {}", self.value);
    }

    pub fn resized(&mut self, size: (u32, u32)) {
        self.surface.configure(Some(size));

        (
            self.blit_bind_group,
            self.shadow_bind_group,
            self.update_bind_group,
        ) = Self::create_bind_groups(
            &self.device,
            &self.limits,
            &self.blit_sampler,
            &self.points,
            &self.blit_bind_group_layout,
            &self.shadow_bind_group_layout,
            &self.update_bind_group_layout,
            size,
        );
        self.size = size;

        let (width, height) = size;
        /* self.draw_target = self.device.create_texture_with_data(
            &self.queue,
            &TextureDescriptor {
                label: None,
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: self.surface.format(),
                usage: TextureUsages::COPY_SRC | TextureUsages::COPY_DST,
                view_formats: &[self.surface.format()],
            },
            &(0..width * height * 4).map(|_| 0u8).collect::<Vec<_>>(),
        ); */
    }

    pub fn frame(&mut self, settings: &RuntimeSettings) {
        /* if self.last_flags != settings.f {
            self.last_flags = settings.f;

            self.resized(self.size);
        } */
        if settings.f & 1 << 11 != 0 {
            // self.resized(self.size);
        }
        // println!("flags: {:b}", settings.f);

        let texture = self
            .surface
            .acquire()
            .expect("Failed to acquire the next frame");

        let texture_view = texture
            .texture
            .create_view(&TextureViewDescriptor { ..<_>::default() });

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { ..<_>::default() });

        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("shadow pass"),
        });

        pass.set_pipeline(&self.shadow_pipeline);

        let push = ShadowPush { flags: settings.f };

        pass.set_push_constants(0, bytemuck::cast_slice(std::slice::from_ref(&push)));
        pass.set_bind_group(0, &self.shadow_bind_group, &[]);
        pass.dispatch_workgroups(self.size.0 / 16 + 1, self.size.1 / 16 + 1, 1);

        drop(pass);

        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("update pass"),
        });

        pass.set_pipeline(&self.update_pipeline);

        let push = UpdatePush {
            cursor: self.cursor,
            time: self.boot.elapsed().as_secs_f32(),
            flags: settings.f,
        };

        pass.set_push_constants(0, bytemuck::cast_slice(std::slice::from_ref(&push)));
        pass.set_bind_group(0, &self.update_bind_group, &[]);
        pass.dispatch_workgroups(self.points_len / 512 + 1, 1, 1);

        drop(pass);

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("blit pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &texture_view,
                resolve_target: None,
                /* ops: Operations {
                    load: LoadOp::Load, // no clear
                    store: true,
                }, */
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }),
                    store: true,
                },
            })],
            ..<_>::default()
        });

        pass.set_pipeline(&self.blit_pipeline);
        pass.set_push_constants(
            ShaderStages::VERTEX,
            0,
            bytemuck::cast_slice(std::slice::from_ref(&BlitPush { flags: 0 })),
        );
        pass.set_bind_group(0, &self.blit_bind_group, &[]);
        pass.draw(0..4, 0..1);

        drop(pass);

        self.queue.submit([encoder.finish()]);

        texture.present();
        self.surface.window.set_visible(true);
    }
}
