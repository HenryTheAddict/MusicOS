//! GPU feedback and original-preset warp/composite passes.
use crate::{
    gpu::Gpu,
    model::{Audio, Model, Vertex},
    shaders,
};
use anyhow::Result;
use wgpu::util::DeviceExt;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const VERTEX: &str = r#"struct Input{@location(0) p:vec2<f32>,@location(1) uv:vec2<f32>,@location(2) original:vec2<f32>,@location(3) color:vec4<f32>};struct Out{@builtin(position) p:vec4<f32>,@location(0) uv:vec2<f32>,@location(1) original:vec2<f32>,@location(2) color:vec4<f32>};@vertex fn main(v:Input)->Out{return Out(vec4(v.p,0.,1.),v.uv,v.original,v.color);}"#;
const BLUR: &str = r#"
struct Out{@builtin(position)p:vec4<f32>,@location(0)uv:vec2<f32>};
@vertex fn vs(@builtin(vertex_index)i:u32)->Out{let p=array<vec2<f32>,3>(vec2(-1.,-1.),vec2(3.,-1.),vec2(-1.,3.));return Out(vec4(p[i],0.,1.),p[i]*vec2(0.5,-0.5)+0.5);}
@group(0)@binding(0)var source:texture_2d<f32>;@group(0)@binding(1)var smp:sampler;
struct Parameters{axis:vec4<f32>,edge:vec4<f32>};@group(0)@binding(2)var<uniform> parameters:Parameters;
@fragment fn fs(v:Out)->@location(0)vec4<f32>{let d=parameters.axis.xy;var c=vec3(0.);
 if d.x!=0.{let weights=array<f32,4>(7.8,6.4,3.1,1.0);let distances=array<f32,4>(0.974358974,2.90625,4.774193548,6.6);for(var i=0;i<4;i++){c+=(textureSample(source,smp,v.uv+d*distances[i]).rgb+textureSample(source,smp,v.uv-d*distances[i]).rgb)*weights[i];}c=c*(0.5/18.3)*parameters.axis.z+parameters.axis.w;}
 else{let weights=array<f32,2>(14.2,4.1);let distances=array<f32,2>(0.9014084507,2.487804878);for(var i=0;i<2;i++){c+=(textureSample(source,smp,v.uv+d*distances[i]).rgb+textureSample(source,smp,v.uv-d*distances[i]).rgb)*weights[i];}c*=0.5/18.3;let t=sqrt(min(min(v.uv.x,v.uv.y),1.-max(v.uv.x,v.uv.y)));c*=1.-parameters.edge.x+parameters.edge.x*clamp(t*5.,0.,1.);}
 return vec4(c,1.);
}"#;
const PRESENT: &str = r#"
struct Out{@builtin(position)p:vec4<f32>,@location(0)uv:vec2<f32>};
@vertex fn vs(@builtin(vertex_index)i:u32)->Out{let p=array<vec2<f32>,3>(vec2(-1.,-1.),vec2(3.,-1.),vec2(-1.,3.));return Out(vec4(p[i],0.,1.),p[i]*vec2(0.5,-0.5)+0.5);}
@group(0)@binding(0)var a:texture_2d<f32>;
@group(0)@binding(1)var b:texture_2d<f32>;
@group(0)@binding(2)var smp:sampler;
struct Values{controls:vec4<f32>,shadow:vec4<f32>,mid:vec4<f32>,highlight:vec4<f32>};
@group(0)@binding(3)var<uniform> values:Values;
@fragment fn fs(v:Out)->@location(0)vec4<f32>{
 var color=clamp(mix(textureSample(a,smp,v.uv).rgb,textureSample(b,smp,v.uv).rgb,values.controls.x)*values.controls.y,vec3(0.),vec3(1.));
 if values.controls.z>0.{
  let weights=vec3(0.2126,0.7152,0.0722);
  let light=dot(color,weights);
  var tone=mix(values.shadow.rgb,values.mid.rgb,smoothstep(0.,0.5,light));
  tone=mix(tone,values.highlight.rgb,smoothstep(0.5,0.85,light));
  // Preserve a preset's different color regions, rather than mapping every
  // equally bright pixel to the same purple/brown band.
  let low=min(color.r,min(color.g,color.b));
  let chromatic=color-vec3(low);
  let hueWeights=chromatic/max(chromatic.r+chromatic.g+chromatic.b,0.001);
  let hueTone=values.shadow.rgb*hueWeights.r+values.mid.rgb*hueWeights.g+values.highlight.rgb*hueWeights.b;
  tone=mix(tone,hueTone,smoothstep(0.025,0.2,max(chromatic.r,max(chromatic.g,chromatic.b))));
  // Fit chroma around the original luminance instead of clipping channels.
  // This preserves bright details, black levels and SDR output.
  let delta=tone-vec3(dot(tone,weights));
  var amount=1.;
  for(var channel=0;channel<3;channel++){
   if delta[channel]>0.0001 {amount=min(amount,(1.-light)/delta[channel]);}
   if delta[channel]<-0.0001 {amount=min(amount,-light/delta[channel]);}
  }
  color=clamp(vec3(light)+delta*max(0.,amount),vec3(0.),vec3(1.));
 }
 return vec4(color,1.);
}"#;

pub struct Image {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampled: wgpu::TextureView,
    mip_views: Vec<wgpu::TextureView>,
    pub width: u32,
    pub height: u32,
}
impl Image {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Visualizer library surface"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: width.max(height).max(1).ilog2() + 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            mip_level_count: Some(1),
            ..Default::default()
        });
        let sampled = texture.create_view(&Default::default());
        let mip_views = (1..texture.mip_level_count())
            .map(|level| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    base_mip_level: level,
                    mip_level_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        Self {
            texture,
            view,
            sampled,
            mip_views,
            width: width.max(1),
            height: height.max(1),
        }
    }
}
fn pipeline(
    device: &wgpu::Device,
    label: &str,
    layout: &wgpu::PipelineLayout,
    vertex: &wgpu::ShaderModule,
    vertex_entry: &str,
    buffers: &[wgpu::VertexBufferLayout<'_>],
    fragment: &wgpu::ShaderModule,
    fragment_entry: &str,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: vertex,
            entry_point: Some(vertex_entry),
            buffers,
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: fragment,
            entry_point: Some(fragment_entry),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview: None,
        cache: None,
    })
}
fn uniform(device: &wgpu::Device, label: &str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
fn texture_binding(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}
fn sampler_binding(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}
fn uniform_binding(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
fn pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
    label: &'a str,
) -> wgpu::RenderPass<'a> {
    pass_with_timestamps(encoder, view, label, None)
}
fn pass_with_timestamps<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
    label: &'a str,
    timestamps: Option<wgpu::RenderPassTimestampWrites<'a>>,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: timestamps,
        occlusion_query_set: None,
    })
}
pub struct Layer {
    pub model: Model,
    feedback: [Image; 2],
    output: Image,
    blur: [Image; 3],
    blur_temp: [Image; 3],
    noise: [Image; 2],
    samplers: [wgpu::Sampler; 4],
    uniform: wgpu::Buffer,
    vertex: wgpu::Buffer,
    comp_vertex: wgpu::Buffer,
    index: wgpu::Buffer,
    layout: wgpu::BindGroupLayout,
    warp: wgpu::RenderPipeline,
    comp: wgpu::RenderPipeline,
    blur_layout: wgpu::BindGroupLayout,
    blur_pipeline: wgpu::RenderPipeline,
    at: usize,
    rand_preset: [f32; 4],
    rand_start: [f32; 4],
    random: crate::random::Random,
    comp_uniform: wgpu::Buffer,
    blur_levels: usize,
    mipmaps: Mipmaps,
    pixel: crate::pixel::Pixel,
    pub sprite: crate::sprites::SpriteRenderer,
}
impl Layer {
    pub fn new(gpu: &Gpu, id: &str, width: u32, height: u32, seed: u64) -> Result<Self> {
        Self::with_reference_state(gpu, id, width, height, seed, None)
    }
    /// Validation can align the random stream after the reference's unused blank
    /// preset/transition constructors. It does not replace any rendered equations.
    pub fn with_reference_state(
        gpu: &Gpu,
        id: &str,
        width: u32,
        height: u32,
        seed: u64,
        reference_state: Option<[u32; 4]>,
    ) -> Result<Self> {
        let mut random = crate::random::Random::new(seed);
        let lq = crate::noise::texture(256, 1, || random.next());
        // Preserve the upstream noise stream without allocating unused textures.
        for _ in 0..(32 * 32 * 4 + 256 * 256 * 4) {
            random.next();
        }
        let hq = crate::noise::texture(256, 8, || random.next());
        for _ in 0..(2 * 32 * 32 * 32 * 4) {
            random.next();
        }
        if let Some(state) = reference_state {
            random = crate::random::Random::from_state(state);
        }
        let rand_start = std::array::from_fn(|_| random.next() as f32);
        let rand_preset = std::array::from_fn(|_| random.next() as f32);
        let model = Model::with_random(id, random.clone(), width, height)?;
        let source = crate::catalog::source(id)?;
        let device = &gpu.device;
        let mut entries = vec![uniform_binding(0)];
        entries.extend((1..=6).map(texture_binding));
        entries.extend((7..=11).map(sampler_binding));
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Original preset inputs"),
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Preset mesh"),
            source: wgpu::ShaderSource::Wgsl(VERTEX.into()),
        });
        let attributes =
            wgpu::vertex_attr_array![0=>Float32x2,1=>Float32x2,2=>Float32x2,3=>Float32x4];
        let buffers = [wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &attributes,
        }];
        let make = |kind: &str| -> Result<wgpu::RenderPipeline> {
            let code = shaders::compile(source[kind].as_str().unwrap(), kind == "comp")?;
            let fragment = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(kind),
                source: wgpu::ShaderSource::Wgsl(code.into()),
            });
            Ok(pipeline(
                device,
                kind,
                &pipeline_layout,
                &vertex_shader,
                "main",
                &buffers,
                &fragment,
                "main",
                FORMAT,
            ))
        };
        let warp = make("warp")?;
        let comp = make("comp")?;
        let vertex = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Preset warp mesh"),
            size: (model.vertices.len() * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let pixel = crate::pixel::Pixel::new(gpu, &model, &vertex)?;
        let comp_vertex = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Preset composite mesh"),
            size: vertex.size(),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&model.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let samplers = std::array::from_fn(|i| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: if i % 2 == 0 {
                    wgpu::AddressMode::Repeat
                } else {
                    wgpu::AddressMode::ClampToEdge
                },
                address_mode_v: if i % 2 == 0 {
                    wgpu::AddressMode::Repeat
                } else {
                    wgpu::AddressMode::ClampToEdge
                },
                mag_filter: if i < 2 {
                    wgpu::FilterMode::Linear
                } else {
                    wgpu::FilterMode::Nearest
                },
                min_filter: if i < 2 {
                    wgpu::FilterMode::Linear
                } else {
                    wgpu::FilterMode::Nearest
                },
                mipmap_filter: if i < 2 {
                    wgpu::FilterMode::Linear
                } else {
                    wgpu::FilterMode::Nearest
                },
                ..Default::default()
            })
        });
        let mipmaps = Mipmaps::new(device);
        let noise = std::array::from_fn(|_| Image::new(device, 256, 256));
        for (i, tex) in noise.iter().enumerate() {
            let bytes = if i == 0 { &lq } else { &hq };
            gpu.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &tex.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(tex.width * 4),
                    rows_per_image: Some(tex.height),
                },
                tex.texture.size(),
            );
        }
        let mut upload = gpu.device.create_command_encoder(&Default::default());
        for image in &noise {
            mipmaps.generate(gpu, &mut upload, image);
        }
        gpu.queue.submit([upload.finish()]);
        let blur_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[texture_binding(0), sampler_binding(1), uniform_binding(2)],
        });
        let bl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&blur_layout],
            push_constant_ranges: &[],
        });
        let bs = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MilkDrop blur"),
            source: wgpu::ShaderSource::Wgsl(BLUR.into()),
        });
        let blur_pipeline = pipeline(device, "Blur", &bl, &bs, "vs", &[], &bs, "fs", FORMAT);
        Ok(Self {
            feedback: std::array::from_fn(|_| Image::new(device, width, height)),
            output: Image::new(device, width, height),
            blur: std::array::from_fn(|i| {
                let (w, h) = blur_size(width, height, [0.25, 0.125, 0.0625][i]);
                Image::new(device, w, h)
            }),
            blur_temp: std::array::from_fn(|i| {
                let (w, h) = blur_size(width, height, [0.5, 0.125, 0.0625][i]);
                Image::new(device, w, h)
            }),
            noise,
            samplers,
            uniform: uniform(device, "Preset globals", 1024),
            vertex,
            comp_vertex,
            index,
            layout,
            warp,
            comp,
            blur_layout,
            blur_pipeline,
            at: 0,
            mipmaps,
            pixel,
            rand_preset,
            rand_start,
            random,
            comp_uniform: uniform(device, "Composite globals", 1024),
            blur_levels: (1..=3)
                .rev()
                .find(|i| {
                    source["warp"]
                        .as_str()
                        .unwrap()
                        .contains(&format!("sampler_blur{i}"))
                        || source["comp"]
                            .as_str()
                            .unwrap()
                            .contains(&format!("sampler_blur{i}"))
                })
                .unwrap_or(0),
            sprite: crate::sprites::SpriteRenderer::new(device),
            model,
        })
    }
    fn bindings(&self, device: &wgpu::Device, main: &Image, comp: bool) -> wgpu::BindGroup {
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: if comp {
                self.comp_uniform.as_entire_binding()
            } else {
                self.uniform.as_entire_binding()
            },
        }];
        for (i, view) in [
            &main.sampled,
            &self.blur[0].sampled,
            &self.blur[1].sampled,
            &self.blur[2].sampled,
            &self.noise[0].sampled,
            &self.noise[1].sampled,
        ]
        .iter()
        .enumerate()
        {
            entries.push(wgpu::BindGroupEntry {
                binding: i as u32 + 1,
                resource: wgpu::BindingResource::TextureView(view),
            });
        }
        for (i, sampler) in self.samplers.iter().enumerate() {
            entries.push(wgpu::BindGroupEntry {
                binding: i as u32 + 7,
                resource: wgpu::BindingResource::Sampler(sampler),
            });
        }
        entries.push(wgpu::BindGroupEntry {
            binding: 11,
            resource: wgpu::BindingResource::Sampler(
                &self.samplers[if self.model.get("wrap") != 0. { 0 } else { 1 }],
            ),
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &entries,
        })
    }
    fn update_uniforms(&mut self, gpu: &Gpu) -> [[f32; 4]; 64] {
        let mut data = [[0_f32; 4]; 64];
        for (i, name) in shaders::SCALARS.iter().enumerate() {
            let key = match *name {
                "gammaAdj" => "gammaadj",
                "echo_orientation" => "echo_orient",
                "fShader" => "fshader",
                _ => name,
            };
            data[i / 4][i % 4] = self.model.get(key) as f32;
        }
        let w = self.output.width as f32;
        let h = self.output.height as f32;
        let ax = (w / h).min(1.);
        let ay = (h / w).min(1.);
        data[8] = [ax, ay, 1. / ax, 1. / ay];
        data[9] = [w, h, 1. / w, 1. / h];
        data[10] = [w, h, 0., 0.];
        data[11] = std::array::from_fn(|_| self.random.next() as f32);
        data[12] = self.rand_preset;
        for i in 0..4 {
            let frequencies = if i % 2 == 0 {
                [0.005, 0.008, 0.013, 0.022]
            } else {
                [0.3, 1.3, 5., 20.]
            };
            data[13 + i] = std::array::from_fn(|j| {
                0.5 + 0.5
                    * if i < 2 {
                        (self.model.time * frequencies[j]).cos() as f32
                    } else {
                        (self.model.time * frequencies[j]).sin() as f32
                    }
            });
        }
        for i in 17..23 {
            let size = if i == 20 || i == 21 || i == 22 {
                32.
            } else {
                256.
            };
            data[i] = [size, size, 1. / size, 1. / size];
        }
        for i in 0..32 {
            data[24 + i / 4][i % 4] = self.model.get(&format!("q{}", i + 1)) as f32;
        }
        for i in 0..3 {
            let min = self.model.get(&format!("b{}n", i + 1));
            let mut max = self.model.get(&format!("b{}x", i + 1));
            if max - min < 0.1 {
                max = min + 0.1;
            }
            let n = 16 + i * 2;
            data[n / 4][n % 4] = min as f32;
            data[(n + 1) / 4][(n + 1) % 4] = max as f32;
            data[(22 + i) / 4][(22 + i) % 4] = (max - min) as f32;
            data[(25 + i) / 4][(25 + i) % 4] = min as f32;
        }
        gpu.queue
            .write_buffer(&self.uniform, 0, bytemuck::cast_slice(&data));
        data
    }
    pub fn render(&mut self, gpu: &Gpu, dt: f64, audio: &Audio) -> Result<()> {
        self.render_profiled(gpu, dt, audio, None)
    }
    pub fn render_profiled(
        &mut self,
        gpu: &Gpu,
        dt: f64,
        audio: &Audio,
        queries: Option<(&wgpu::QuerySet, u32, u32)>,
    ) -> Result<()> {
        self.model
            .advance_frame(dt, self.output.width, self.output.height, audio);
        self.model.validate_frame()?;
        let mut globals = self.update_uniforms(gpu);
        let mut comp_vertices = self.model.vertices.clone();
        let mut colors = [[0_f32; 3]; 4];
        for i in 0..4 {
            for c in 0..3 {
                colors[i][c] = 0.6
                    + 0.3
                        * (self.model.time * 30. * [0.0143, 0.0107, 0.0129][c]
                            + [3., 1., 6.][c]
                            + i as f64 * [21., 13., 9.][c]
                            + self.rand_start[[3, 1, 2][c]] as f64)
                            .sin() as f32;
            }
            let m = colors[i].iter().copied().fold(0_f32, f32::max);
            for c in &mut colors[i] {
                *c = 0.5 + 0.5 * (*c / m);
            }
        }
        for v in &mut comp_vertices {
            v.uv = v.original;
            let x = v.original[0];
            let y = 1. - v.original[1];
            for c in 0..3 {
                v.color[c] = colors[0][c] * x * y
                    + colors[1][c] * (1. - x) * y
                    + colors[2][c] * x * (1. - y)
                    + colors[3][c] * (1. - x) * (1. - y);
            }
        }
        gpu.queue
            .write_buffer(&self.comp_vertex, 0, bytemuck::cast_slice(&comp_vertices));
        let next = 1 - self.at;
        let bind = self.bindings(&gpu.device, &self.feedback[self.at], false);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        self.pixel.profiled(
            gpu,
            &mut encoder,
            &self.model,
            queries.map(|(q, start, _)| wgpu::ComputePassTimestampWrites {
                query_set: q,
                beginning_of_pass_write_index: Some(start),
                end_of_pass_write_index: None,
            }),
        );
        self.mipmaps
            .generate(gpu, &mut encoder, &self.feedback[self.at]);
        {
            let mut p = pass(&mut encoder, &self.feedback[next].view, "Original warp");
            p.set_pipeline(&self.warp);
            p.set_bind_group(0, &bind, &[]);
            p.set_vertex_buffer(0, self.vertex.slice(..));
            p.set_index_buffer(self.index.slice(..), wgpu::IndexFormat::Uint32);
            p.draw_indexed(0..self.model.indices.len() as u32, 0, 0..1);
        }
        for i in 0..self.blur_levels {
            let input = if i == 0 {
                &self.feedback[next]
            } else {
                &self.blur[i - 1]
            };
            for (horizontal, source, target) in [
                (true, input, &self.blur_temp[i]),
                (false, &self.blur_temp[i], &self.blur[i]),
            ] {
                let d = if horizontal {
                    let min = self.model.get(&format!("b{}n", i + 1));
                    let max = self.model.get(&format!("b{}x", i + 1)).max(min + 0.1);
                    let prior = if i == 0 {
                        (0., 1.)
                    } else {
                        (
                            self.model.get(&format!("b{}n", i)),
                            self.model.get(&format!("b{}x", i)),
                        )
                    };
                    [
                        (1. / source.width as f32),
                        0.,
                        ((prior.1 - prior.0) / (max - min)) as f32,
                        ((prior.0 - min) / (max - min)) as f32,
                        0.,
                        0.,
                        0.,
                        0.,
                    ]
                } else {
                    [
                        0.,
                        1. / source.height as f32,
                        1.,
                        0.,
                        if i == 0 {
                            self.model.get("b1ed") as f32
                        } else {
                            0.
                        },
                        0.,
                        0.,
                        0.,
                    ]
                };
                let u = gpu
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: bytemuck::cast_slice(&d),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });
                let b = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.blur_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&source.sampled),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.samplers[1]),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: u.as_entire_binding(),
                        },
                    ],
                });
                let mut p = pass(&mut encoder, &target.view, "Feedback blur");
                p.set_pipeline(&self.blur_pipeline);
                p.set_bind_group(0, &b, &[]);
                p.draw(0..3, 0..1);
                drop(p);
                self.mipmaps.generate(gpu, &mut encoder, target);
            }
        }
        self.sprite.render(
            gpu,
            &mut encoder,
            &self.feedback[next].view,
            &self.feedback[self.at].sampled,
            &self.vertex,
            &mut self.model,
            audio,
            self.output.width,
            self.output.height,
        );
        // Custom shape/wave equations share the stream and run before composite.
        globals[11] = std::array::from_fn(|_| self.random.next() as f32);
        gpu.queue
            .write_buffer(&self.comp_uniform, 0, bytemuck::cast_slice(&globals));
        let bind = self.bindings(&gpu.device, &self.feedback[next], true);
        {
            let mut p = pass_with_timestamps(
                &mut encoder,
                &self.output.view,
                "Original composite",
                queries.map(|(q, _, end)| wgpu::RenderPassTimestampWrites {
                    query_set: q,
                    beginning_of_pass_write_index: None,
                    end_of_pass_write_index: Some(end),
                }),
            );
            p.set_pipeline(&self.comp);
            p.set_bind_group(0, &bind, &[]);
            p.set_vertex_buffer(0, self.comp_vertex.slice(..));
            p.set_index_buffer(self.index.slice(..), wgpu::IndexFormat::Uint32);
            p.draw_indexed(0..self.model.indices.len() as u32, 0, 0..1);
        }
        gpu.queue.submit([encoder.finish()]);
        self.at = next;
        Ok(())
    }
    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) {
        if self.output.width == width && self.output.height == height {
            return;
        }
        let next = std::array::from_fn(|_| Image::new(&gpu.device, width, height));
        let presenter = Presenter::new(&gpu.device, FORMAT);
        for i in 0..2 {
            presenter.draw(
                gpu,
                &next[i].view,
                &self.feedback[i],
                &self.feedback[i],
                0.,
                1.,
            );
        }
        self.feedback = next;
        self.output = Image::new(&gpu.device, width, height);
        self.blur = std::array::from_fn(|i| {
            let (w, h) = blur_size(width, height, [0.25, 0.125, 0.0625][i]);
            Image::new(&gpu.device, w, h)
        });
        self.blur_temp = std::array::from_fn(|i| {
            let (w, h) = blur_size(width, height, [0.5, 0.125, 0.0625][i]);
            Image::new(&gpu.device, w, h)
        });
    }
    pub fn mesh_buffer(&self) -> &wgpu::Buffer {
        &self.vertex
    }
    pub fn output(&self) -> &Image {
        &self.output
    }
}
pub struct Presenter {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    values: wgpu::Buffer,
}
impl Presenter {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                texture_binding(0),
                texture_binding(1),
                sampler_binding(2),
                uniform_binding(3),
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Visualizer SDR output"),
            source: wgpu::ShaderSource::Wgsl(PRESENT.into()),
        });
        let pipeline = pipeline(
            device,
            "Visualizer output",
            &pl,
            &shader,
            "vs",
            &[],
            &shader,
            "fs",
            format,
        );
        Self {
            layout,
            pipeline,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            values: uniform(device, "Visualizer blend and album colors", 64),
        }
    }
    pub fn draw(
        &self,
        gpu: &Gpu,
        target: &wgpu::TextureView,
        a: &Image,
        b: &Image,
        blend: f32,
        beat_gain: f32,
    ) {
        self.draw_colored(gpu, target, a, b, blend, beat_gain, None);
    }
    /// Final-output recoloring never enters feedback textures or preset equations.
    pub fn draw_colored(
        &self,
        gpu: &Gpu,
        target: &wgpu::TextureView,
        a: &Image,
        b: &Image,
        blend: f32,
        beat_gain: f32,
        palette: Option<[[f32; 3]; 3]>,
    ) {
        let palette = palette.filter(|colors| {
            colors
                .iter()
                .flatten()
                .all(|v| v.is_finite() && (0. ..=1.).contains(v))
        });
        let mut values = [[0_f32; 4]; 4];
        values[0] = [
            blend.clamp(0., 1.),
            beat_gain.clamp(1., 1.06),
            if palette.is_some() { 1. } else { 0. },
            0.,
        ];
        if let Some(colors) = palette {
            for (target, color) in values[1..].iter_mut().zip(colors) {
                target[..3].copy_from_slice(&color);
            }
        }
        gpu.queue
            .write_buffer(&self.values, 0, bytemuck::cast_slice(&values));
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&a.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&b.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.values.as_entire_binding(),
                },
            ],
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        {
            let mut p = pass(&mut encoder, target, "Visualizer presentation");
            p.set_pipeline(&self.pipeline);
            p.set_bind_group(0, &bind, &[]);
            p.draw(0..3, 0..1);
        }
        gpu.queue.submit([encoder.finish()]);
    }
}

fn blur_size(width: u32, height: u32, ratio: f64) -> (u32, u32) {
    (
        (((width as f64 * ratio).max(16.) + 3.) / 16.).floor() as u32 * 16,
        (((height as f64 * ratio).max(16.) + 3.) / 4.).floor() as u32 * 4,
    )
}

const MIPMAP: &str = r#"struct Out{@builtin(position)p:vec4<f32>,@location(0)uv:vec2<f32>};@vertex fn vs(@builtin(vertex_index)i:u32)->Out{let p=array<vec2<f32>,3>(vec2(-1.,-1.),vec2(3.,-1.),vec2(-1.,3.));return Out(vec4(p[i],0.,1.),p[i]*vec2(0.5,-0.5)+0.5);}@group(0)@binding(0)var source:texture_2d<f32>;@group(0)@binding(1)var smp:sampler;@fragment fn fs(v:Out)->@location(0)vec4<f32>{return textureSample(source,smp,v.uv);}"#;
struct Mipmaps {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}
impl Mipmaps {
    fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Feedback mipmaps"),
            entries: &[texture_binding(0), sampler_binding(1)],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Feedback downsample"),
            source: wgpu::ShaderSource::Wgsl(MIPMAP.into()),
        });
        Self {
            pipeline: pipeline(
                device,
                "Feedback mipmap",
                &pl,
                &shader,
                "vs",
                &[],
                &shader,
                "fs",
                FORMAT,
            ),
            layout,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
        }
    }
    fn generate(&self, gpu: &Gpu, encoder: &mut wgpu::CommandEncoder, image: &Image) {
        for (i, target) in image.mip_views.iter().enumerate() {
            let source = if i == 0 {
                &image.view
            } else {
                &image.mip_views[i - 1]
            };
            let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            let mut p = pass(encoder, target, "Feedback mipmap");
            p.set_pipeline(&self.pipeline);
            p.set_bind_group(0, &bind, &[]);
            p.draw(0..3, 0..1);
        }
    }
}
