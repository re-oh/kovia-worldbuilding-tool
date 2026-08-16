use iced::mouse;
use iced::widget::shader::{self, Viewport};
use iced::{Element, Fill, Rectangle, Theme};
use iced_wgpu::wgpu;
use kovia_protocol::{MapInput, PointerButton};

use crate::Message;

pub struct MapViewport {
    texture_view: wgpu::TextureView,
    physical_size: [u32; 2],
    scale_factor: f32,
}

impl MapViewport {
    pub fn new(
        texture_view: wgpu::TextureView,
        physical_size: [u32; 2],
        scale_factor: f32,
    ) -> Self {
        Self {
            texture_view,
            physical_size,
            scale_factor,
        }
    }

    pub fn replace(
        &mut self,
        texture_view: wgpu::TextureView,
        physical_size: [u32; 2],
        scale_factor: f32,
    ) {
        self.texture_view = texture_view;
        self.physical_size = physical_size;
        self.scale_factor = scale_factor;
    }

    pub fn view(&self) -> Element<'_, Message, Theme, iced_wgpu::Renderer> {
        shader::Shader::new(self).width(Fill).height(Fill).into()
    }
}

impl shader::Program<Message> for &MapViewport {
    type State = ();
    type Primitive = ViewportPrimitive;

    fn update(
        &self,
        _state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<shader::Action<Message>> {
        let local = cursor
            .position_in(bounds)
            .map(|position| logical_to_physical(position, self.scale_factor));

        match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => local.map(|position| {
                shader::Action::publish(Message::MapInput(MapInput::PointerMoved {
                    physical_position: position,
                }))
            }),
            iced::Event::Mouse(mouse::Event::ButtonPressed(button)) => {
                let button = pointer_button(*button)?;
                local.map(|position| {
                    shader::Action::publish(Message::MapInput(MapInput::PointerDown {
                        physical_position: position,
                        button,
                    }))
                    .and_capture()
                })
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(button)) => {
                let button = pointer_button(*button)?;
                local.map(|position| {
                    shader::Action::publish(Message::MapInput(MapInput::PointerUp {
                        physical_position: position,
                        button,
                    }))
                    .and_capture()
                })
            }
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) if local.is_some() => {
                let physical_delta = match delta {
                    mouse::ScrollDelta::Lines { x, y } => [x * 40.0, y * 40.0],
                    mouse::ScrollDelta::Pixels { x, y } => {
                        [x * self.scale_factor, y * self.scale_factor]
                    }
                };
                Some(
                    shader::Action::publish(Message::MapInput(MapInput::Scroll { physical_delta }))
                        .and_capture(),
                )
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        ViewportPrimitive {
            texture_view: self.texture_view.clone(),
            physical_size: self.physical_size,
        }
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

fn logical_to_physical(position: iced::Point, scale_factor: f32) -> [f32; 2] {
    [position.x * scale_factor, position.y * scale_factor]
}

fn pointer_button(button: mouse::Button) -> Option<PointerButton> {
    match button {
        mouse::Button::Left => Some(PointerButton::Left),
        mouse::Button::Right => Some(PointerButton::Right),
        mouse::Button::Middle => Some(PointerButton::Middle),
        _ => None,
    }
}

#[derive(Debug)]
pub struct ViewportPrimitive {
    texture_view: wgpu::TextureView,
    physical_size: [u32; 2],
}

impl shader::Primitive for ViewportPrimitive {
    type Pipeline = ViewportPipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _bounds: &Rectangle,
        _viewport: &Viewport,
    ) {
        pipeline.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Kovia Atlas viewport bind group"),
            layout: &pipeline.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                },
            ],
        }));
        pipeline.last_source_size = self.physical_size;
    }

    fn draw(&self, pipeline: &Self::Pipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        let Some(bind_group) = &pipeline.bind_group else {
            return true;
        };
        render_pass.set_pipeline(&pipeline.pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);
        true
    }
}

pub struct ViewportPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    bind_group: Option<wgpu::BindGroup>,
    last_source_size: [u32; 2],
}

impl shader::Pipeline for ViewportPipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Kovia Atlas viewport bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Kovia Atlas viewport shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("viewport.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Kovia Atlas viewport pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Kovia Atlas viewport pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            bind_group_layout,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Kovia Atlas viewport sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            bind_group: None,
            last_source_size: [0, 0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_coordinates_convert_once_to_physical_pixels() {
        assert_eq!(
            logical_to_physical(iced::Point::new(12.5, 20.0), 2.0),
            [25.0, 40.0]
        );
    }
}
