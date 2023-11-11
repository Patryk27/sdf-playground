use pixels::wgpu;
use sdf_playground_common::Params;
use std::path::PathBuf;
use std::{fs, mem};

#[derive(Debug)]
pub struct Renderer {
    path: PathBuf,
    texture_view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    params_buffer: wgpu::Buffer,
}

impl Renderer {
    pub fn new(
        pixels: &pixels::Pixels,
        width: u32,
        height: u32,
        path: PathBuf,
    ) -> Self {
        let device = pixels.device();
        let shader = fs::read(&path).unwrap();

        let module = device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: Some("renderer_shader"),
                source: wgpu::util::make_spirv(&shader),
            },
        );

        let texture_descriptor = wgpu::TextureDescriptor {
            label: Some("renderer_texture_descriptor"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: pixels.render_texture_format(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };

        let texture_view = device
            .create_texture(&texture_descriptor)
            .create_view(&Default::default());

        let params_buffer =
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("renderer_params_buffer"),
                size: mem::size_of::<Params>()
                    as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("renderer_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("renderer_bind_group"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer
                        .as_entire_binding(),
                }],
            },
        );

        let pipeline_layout = device
            .create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("renderer_pipeline_layout"),
                    bind_group_layouts: &[
                        &bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                },
            );

        let pipeline = device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("renderer_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: "main_vs",
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample:
                    wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: "main_fs",
                    targets: &[Some(
                        wgpu::ColorTargetState {
                            format: pixels
                                .render_texture_format(),
                            blend: Some(
                                wgpu::BlendState::REPLACE,
                            ),
                            write_mask:
                                wgpu::ColorWrites::ALL,
                        },
                    )],
                }),
                multiview: None,
            },
        );

        Self {
            path,
            texture_view,
            bind_group,
            pipeline,
            params_buffer,
        }
    }

    pub fn texture_view(&self) -> &wgpu::TextureView {
        &self.texture_view
    }

    pub fn resize(
        &mut self,
        pixels: &pixels::Pixels,
        width: u32,
        height: u32,
    ) {
        *self = Self::new(
            pixels,
            width,
            height,
            mem::take(&mut self.path),
        );
    }

    pub fn update(
        &self,
        queue: &wgpu::Queue,
        params: &Params,
    ) {
        queue.write_buffer(
            &self.params_buffer,
            0,
            bytemuck::bytes_of(params),
        );
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("renderer_render_pass"),
                color_attachments: &[Some(
                    wgpu::RenderPassColorAttachment {
                        view: target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(
                                wgpu::Color::BLACK,
                            ),
                            store: true,
                        },
                    },
                )],
                depth_stencil_attachment: None,
            },
        );

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
