//! Small, monochrome GPU accents. All inspection content remains ordinary Rune UI.
use crate::State;
use rune::El;
use rune::compose::{Compositor, DeviceKey, Frame};
use rune::render::{CLIP_WGSL, EffectSite};

const MARK: u32 = 0x4a455600;
const FIELD: u32 = MARK + 1;
const FLOW: u32 = MARK + 2;
const BURST_SECONDS: f64 = 1.8;

pub fn mark(dark: bool) -> El {
    rune::effect(MARK)
        .effect_data([0.0, f32::from(dark), 0.0, 0.0])
        .key("ascii-mark")
        .a11y_hidden()
}

pub fn field(dark: bool) -> El {
    rune::effect(FIELD)
        .effect_data([1.0, f32::from(dark), 0.0, 0.0])
        .key("ascii-field")
        .a11y_hidden()
}

pub fn header_flow(dark: bool) -> El {
    rune::effect(FLOW)
        .effect_data([2.0, f32::from(dark), 0.0, 0.0])
        .key("ascii-header-flow")
        .a11y_hidden()
}

#[derive(Default)]
struct Burst {
    previous: Option<(Option<u64>, u8, u64)>,
    started: f64,
}

impl Burst {
    fn update(&mut self, event: (Option<u64>, u8, u64), now: f64, reduced: bool) -> (f32, bool) {
        if self.previous != Some(event) {
            self.previous = Some(event);
            self.started = now;
        }
        let elapsed = (now - self.started).max(0.0);
        if reduced || elapsed >= BURST_SECONDS {
            (0.0, false)
        } else {
            ((elapsed / BURST_SECONDS) as f32, true)
        }
    }
}

#[derive(Default)]
pub struct AsciiCompositor {
    device: Option<DeviceKey>,
    gpu: Option<GpuEffects>,
    burst: Burst,
    moving: bool,
}

struct Slot {
    buffer: wgpu::Buffer,
    group: wgpu::BindGroup,
}

struct GpuEffects {
    layout: wgpu::BindGroupLayout,
    shader: wgpu::ShaderModule,
    pipelines: Vec<(wgpu::TextureFormat, wgpu::RenderPipeline)>,
    slots: Vec<Slot>,
    used: usize,
}

impl GpuEffects {
    fn new(device: &wgpu::Device) -> Self {
        Self {
            layout: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ASCII uniforms"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            }),
            shader: device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Jev ASCII signal"),
                source: wgpu::ShaderSource::Wgsl(
                    format!("{CLIP_WGSL}\n{}", include_str!("ascii_fx.wgsl")).into(),
                ),
            }),
            pipelines: Vec::new(),
            slots: Vec::new(),
            used: 0,
        }
    }

    fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pass: &mut wgpu::RenderPass<'_>,
        site: &EffectSite<'_>,
        phase: f32,
    ) {
        if site.mark != MARK && site.mark != FIELD && site.mark != FLOW {
            return;
        }
        if !self
            .pipelines
            .iter()
            .any(|(format, _)| *format == site.format)
        {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ASCII pipeline layout"),
                bind_group_layouts: &[Some(&self.layout)],
                immediate_size: 0,
            });
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("ASCII signal"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: site.format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
            self.pipelines.push((site.format, pipeline));
        }
        // Every draw occurrence needs its own buffer: an exiting and entering
        // slot can share a mark, and queue writes precede the entire submission.
        if self.used == self.slots.len() {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ASCII slot"),
                size: 96,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ASCII slot"),
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });
            self.slots.push(Slot { buffer, group });
        }
        let slot = &self.slots[self.used];
        self.used += 1;
        let i = site.instance;
        let mut data = [0.0f32; 24];
        data[0..4].copy_from_slice(&i.rect);
        data[4..8].copy_from_slice(&i.clip);
        for (dst, src) in data[8..12].iter_mut().zip(i.fade) {
            *dst = f32::from_bits(src);
        }
        data[12..16].copy_from_slice(&[
            site.origin[0],
            site.origin[1],
            site.size[0] as f32,
            site.size[1] as f32,
        ]);
        data[16..20].copy_from_slice(&[phase, site.scale, site.data[0], site.data[1]]);
        data[20] = f32::from(site.srgb_blend);
        queue.write_buffer(&slot.buffer, 0, bytemuck::cast_slice(&data));
        pass.set_pipeline(
            &self
                .pipelines
                .iter()
                .find(|(f, _)| *f == site.format)
                .unwrap()
                .1,
        );
        pass.set_bind_group(0, &slot.group, &[]);
        pass.draw(0..4, 0..1);
    }
}

impl Compositor<State> for AsciiCompositor {
    fn render(&mut self, f: &mut Frame<'_, State>) {
        if self.device != Some(f.device_key()) {
            self.gpu = Some(GpuEffects::new(f.device));
            self.device = Some(f.device_key());
        }
        let s = &f.rt.state;
        let (phase, moving) = self.burst.update(
            (s.selected, s.tab, s.snapshot.total_requests),
            f.rt.t,
            f.rt.system_reduced_motion(),
        );
        self.moving = moving;
        let gpu = self.gpu.as_mut().unwrap();
        gpu.used = 0;
        let clear = f.rt.clear_color();
        f.ui.prepare(
            f.device,
            f.queue,
            &mut f.rt.text.atlas,
            &f.rt.instances,
            f.size,
        );
        f.ui.draw_with(f.encoder, f.target, clear, &mut |pass, site| {
            gpu.draw(f.device, f.queue, pass, site, phase);
        });
    }

    fn animating(&self) -> bool {
        self.moving
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_flow_uses_available_width_without_overlapping_controls() {
        let mut previous_width = 0.0;
        for width in [1100.0, 1440.0, 1900.0] {
            let mut state = State::default();
            crate::demo::populate(&mut state);
            let mut rt = rune::Runtime::new(
                state,
                crate::view,
                (width, 700.0),
                rune::Env {
                    system_dark: true,
                    scale: 1.0,
                },
            );
            rt.frame(0.0);
            rt.frame(0.3);
            rt.frame(0.3);
            let (flow, _) = rt.presented("ascii-header-flow").unwrap();
            assert!(flow[2] > 250.0 && flow[2] > previous_width);
            previous_width = flow[2];
            for key in ["open-session", "export-menu", "help", "theme-toggle"] {
                let (control, _) = rt.presented(key).unwrap();
                assert!(flow[0] + flow[2] <= control[0]);
                assert!(control[0] + control[2] <= width);
            }
        }
    }

    #[test]
    fn activity_settles_and_reduced_motion_never_requests_frames() {
        let mut burst = Burst::default();
        let first = (Some(1), 0, 14);
        assert!(burst.update(first, 10.0, false).1);
        assert!(burst.update(first, 10.5, false).1);
        assert_eq!(burst.update(first, 12.0, false), (0.0, false));
        assert!(burst.update((Some(2), 0, 14), 13.0, false).1);
        assert_eq!(burst.update((Some(3), 0, 14), 14.0, true), (0.0, false));
        assert_eq!(burst.update(first, 20.0, true), (0.0, false));
    }
}
