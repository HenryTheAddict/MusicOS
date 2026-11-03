//! Compile independent curated EEL paths to a native compute dispatch.
//! The CPU interpreter remains the validation reference.
use crate::{gpu::Gpu, model::Model};
use anyhow::{Context, Result};
const HELPERS: &str = r#"
fn finite_value(v:f32)->f32{if abs(v)<=3.402823e38{return v;}return 0.;}
fn coordinate(v:f32)->f32{if abs(v)<=3.402823e38{return v;}return 0.5;}
fn divide(a:f32,b:f32)->f32{if b==0.{return 0.;}return a/b;}
fn remainder(a:f32,b:f32)->f32{let d=floor(b);if d==0.{return 0.;}return floor(a)-trunc(floor(a)/d)*d;}
fn power(a:f32,b:f32)->f32{if a<0.{if b!=trunc(b){return 0.;}let magnitude=pow(-a,b);if i32(abs(b))%2==1{return -magnitude;}return magnitude;}return pow(a,b);}
@group(0)@binding(0)var<storage,read> globals:array<f32>;
@group(0)@binding(1)var<storage,read_write> vertices:array<f32>;
"#;
pub fn source(model: &Model) -> Result<String> {
    let (program, mut used) = model.pixel.gpu_statements(&model.symbols)?;
    let names = [
        "zoom",
        "zoomexp",
        "rot",
        "warp",
        "cx",
        "cy",
        "dx",
        "dy",
        "sx",
        "sy",
        "x",
        "y",
        "rad",
        "ang",
        "time",
        "warpanimspeed",
        "warpscale",
        "aspectx",
        "aspecty",
    ];
    let slots = names.map(|n| model.symbols.find(n).unwrap());
    used.extend(slots);
    let mut shader = HELPERS.to_owned();
    shader+="@compute @workgroup_size(64) fn main(@builtin(global_invocation_id)thread:vec3<u32>){let id=thread.x;if id>=3185u{return;}\n";
    for slot in used {
        shader += &format!("var v{slot}=globals[{slot}u];\n");
    }
    let v = |i: usize| format!("v{}", slots[i]);
    shader+=&format!("let px=f32(id%65u)/32.-1.;let py=f32(id/65u)/24.-1.;let ax=1.0/{};let ay=1.0/{};let radius=length(vec2(px*ax,py*ay));{}=px*0.5*ax+0.5;{}=-py*0.5*ay+0.5;{}=radius;{}=0.;if px!=0.||py!=0.{{{}=atan2(py*ay,px*ax);}}\n",v(17),v(18),v(10),v(11),v(12),v(13),v(13));
    shader += &program;
    shader += &format!(
        r#"
 let animation={time}*{speed};let inv_scale=divide(1.,{scale});
 let f0=11.68+4.*cos(animation*1.413+10.);let f1=8.77+3.*cos(animation*1.113+7.);let f2=10.54+3.*cos(animation*1.233+3.);let f3=11.49+4.*cos(animation*0.933+5.);
 let zoom=power({zoom},power({zoomexp},radius*2.-1.));
 var u=px*0.5*ax/zoom+0.5;var vv=-py*0.5*ay/zoom+0.5;
 u=(u-{cx})/{sx}+{cx};vv=(vv-{cy})/{sy}+{cy};let warp={warp}*0.0035;
 if warp!=0.{{u+=warp*sin(animation*0.333+inv_scale*(px*f0-py*f3));vv+=warp*cos(animation*0.375-inv_scale*(px*f2+py*f1));u+=warp*cos(animation*0.753-inv_scale*(px*f1-py*f2));vv+=warp*sin(animation*0.825+inv_scale*(px*f0+py*f3));}}
 let a=u-{cx};let b=vv-{cy};let rotation={rot};u=(a*cos(rotation)-b*sin(rotation)+{cx}-{dx}-0.5)/ax+0.5;vv=(a*sin(rotation)+b*cos(rotation)+{cy}-{dy}-0.5)/ay+0.5;
 let at=id*10u;vertices[at]=px;vertices[at+1u]=-py;vertices[at+2u]=coordinate(u);vertices[at+3u]=coordinate(vv);vertices[at+4u]=px*0.5+0.5;vertices[at+5u]=-py*0.5+0.5;vertices[at+6u]=1.;vertices[at+7u]=1.;vertices[at+8u]=1.;vertices[at+9u]=1.;
 }}"#,
        time = v(14),
        speed = v(15),
        scale = v(16),
        zoom = v(0),
        zoomexp = v(1),
        cx = v(4),
        cy = v(5),
        sx = v(8),
        sy = v(9),
        warp = v(3),
        rot = v(2),
        dx = v(6),
        dy = v(7)
    );
    Ok(shader)
}
pub struct Pixel {
    pipeline: wgpu::ComputePipeline,
    bind: wgpu::BindGroup,
    globals: wgpu::Buffer,
}
impl Pixel {
    pub fn new(gpu: &Gpu, model: &Model, vertices: &wgpu::Buffer) -> Result<Self> {
        let code = source(model)?;
        let module = naga::front::wgsl::parse_str(&code)
            .map_err(|e| anyhow::anyhow!("{}", e.emit_to_string(&code)))?;
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .context("Invalid native pixel equation")?;
        let device = &gpu.device;
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Native pixel inputs"),
            size: (model.values.slots.len() * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
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
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Native pixel equations"),
            source: wgpu::ShaderSource::Wgsl(code.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Native mesh deformation"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: vertices.as_entire_binding(),
                },
            ],
        });
        Ok(Self {
            pipeline,
            bind,
            globals,
        })
    }
    pub fn run(&self, gpu: &Gpu, encoder: &mut wgpu::CommandEncoder, model: &Model) {
        self.profiled(gpu, encoder, model, None);
    }
    pub fn profiled(
        &self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        model: &Model,
        timestamps: Option<wgpu::ComputePassTimestampWrites<'_>>,
    ) {
        let data = model
            .values
            .slots
            .iter()
            .map(|v| *v as f32)
            .collect::<Vec<_>>();
        gpu.queue
            .write_buffer(&self.globals, 0, bytemuck::cast_slice(&data));
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Native pixel equations"),
            timestamp_writes: timestamps,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.dispatch_workgroups(50, 1, 1);
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn every_curated_pixel_program_is_parallel_and_valid() {
        for preset in crate::catalog::catalog() {
            let m = crate::model::Model::load(&preset.id, 42).unwrap();
            let source = super::source(&m).unwrap_or_else(|e| panic!("{}: {e}", preset.id));
            naga::front::wgsl::parse_str(&source)
                .unwrap_or_else(|e| panic!("{}: {}", preset.id, e.emit_to_string(&source)));
        }
    }
}
