//! Preset custom shapes, custom waveforms, basic waveform modes and borders.
use crate::{
    eel::{Symbols, Values},
    gpu::Gpu,
    model::{Audio, Component, Model},
};
use wgpu::util::DeviceExt;
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Vertex {
    p: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    textured: f32,
}
struct Batch {
    vertices: Vec<Vertex>,
    additive: bool,
}
fn vertex(p: [f64; 2], uv: [f64; 2], color: [f64; 4], textured: bool) -> Vertex {
    Vertex {
        p: p.map(|x| if x.is_finite() { x as f32 } else { 0. }),
        uv: uv.map(|x| x as f32),
        color: color.map(|x| if x.is_finite() { x as f32 } else { 0. }),
        textured: if textured { 1. } else { 0. },
    }
}
fn line(batch: &mut Batch, a: Vertex, b: Vertex, width: f64, w: u32, h: u32) {
    let dx = (b.p[0] - a.p[0]) as f64 * w as f64;
    let dy = (b.p[1] - a.p[1]) as f64 * h as f64;
    let length = dx.hypot(dy).max(1e-6);
    let offset = [
        (-dy / length * width / w as f64) as f32,
        (dx / length * width / h as f64) as f32,
    ];
    let mut aa = a;
    let mut ab = a;
    let mut ba = b;
    let mut bb = b;
    for i in 0..2 {
        aa.p[i] += offset[i];
        ab.p[i] -= offset[i];
        ba.p[i] += offset[i];
        bb.p[i] -= offset[i];
    }
    batch.vertices.extend_from_slice(&[aa, ab, ba, ba, ab, bb]);
}
fn dot(batch: &mut Batch, a: Vertex, width: f64, w: u32, h: u32) {
    let mut corners = [a; 4];
    for (i, p) in [[-1., -1.], [1., -1.], [-1., 1.], [1., 1.]]
        .iter()
        .enumerate()
    {
        corners[i].p[0] += p[0] * width as f32 / w as f32;
        corners[i].p[1] += p[1] * width as f32 / h as f32;
    }
    batch.vertices.extend_from_slice(&[
        corners[0], corners[1], corners[2], corners[2], corners[1], corners[3],
    ]);
}
fn globals(c: &mut Component, s: &Symbols, main: &Values, qs: &[usize; 32]) {
    if !c.initialized {
        crate::model::initialize_component(c, s, main);
    }
    for (i, v) in &c.base {
        c.values.slots[*i] = *v;
    }
    for name in [
        "time", "frame", "fps", "bass", "mid", "treb", "bass_att", "mid_att", "treb_att", "vol",
        "vol_att", "aspectx", "aspecty", "pixelsx", "pixelsy", "meshx", "meshy",
    ] {
        c.values.set(s, name, main.get(s, name));
    }
    for &slot in qs {
        c.values.slots[slot] = main.slots[slot];
    }
    for &(slot, value) in &c.t_initial {
        c.values.slots[slot] = value;
    }
}
fn geometry(model: &mut Model, audio: &Audio, w: u32, h: u32) -> Vec<Batch> {
    let mut batches = Vec::new();
    let s = &model.symbols;
    let main = &model.values;
    let q_slots = std::array::from_fn(|i| s.find(&format!("q{}", i + 1)).unwrap());
    let ay = (h as f64 / w as f64).min(1.);
    let ax = (w as f64 / h as f64).min(1.);
    motion_vectors(model, w, h, &mut batches);
    for c in &mut model.shapes {
        globals(c, s, main, &q_slots);
        let instances = c.values.get(s, "num_inst").clamp(1., 1024.) as usize;
        let mut batch = Batch {
            vertices: Vec::with_capacity(instances * 300),
            additive: c.values.get(s, "additive") != 0.,
        };
        for instance in 0..instances {
            for (i, v) in &c.base {
                c.values.slots[*i] = *v;
            }
            for &slot in &q_slots {
                c.values.slots[slot] = main.slots[slot];
            }
            for &(slot, value) in &c.t_initial {
                c.values.slots[slot] = value;
            }
            c.values.set(s, "instance", instance as f64);
            c.frame.run(&mut c.values);
            let g = |name: &str| c.values.get(s, name);
            batch.additive = g("additive") != 0.;
            let sides = g("sides").clamp(3., 100.) as usize;
            let center = [g("x") * 2. - 1., 1. - g("y") * 2.];
            let color = [g("r"), g("g"), g("b"), g("a")];
            let edge = [g("r2"), g("g2"), g("b2"), g("a2")];
            let border = [g("border_r"), g("border_g"), g("border_b"), g("border_a")];
            let textured = g("textured").abs() >= 1.;
            let rad = g("rad");
            let angle = g("ang");
            let tex_angle = g("tex_ang");
            let tex_zoom = g("tex_zoom").max(1e-6);
            let at = |i: usize| {
                let t = i as f64 / sides as f64 * std::f64::consts::TAU;
                vertex(
                    [
                        center[0] + rad * (t + angle + std::f64::consts::FRAC_PI_4).cos() * ay,
                        center[1] + rad * (t + angle + std::f64::consts::FRAC_PI_4).sin(),
                    ],
                    [
                        0.5 + 0.5 * (t + tex_angle + std::f64::consts::FRAC_PI_4).cos() / tex_zoom
                            * ay,
                        0.5 + 0.5 * (t + tex_angle + std::f64::consts::FRAC_PI_4).sin() / tex_zoom,
                    ],
                    edge,
                    textured,
                )
            };
            for i in 0..sides {
                let a = at(i);
                let b = at(i + 1);
                batch.vertices.extend_from_slice(&[
                    vertex(center, [0.5, 0.5], color, textured),
                    a,
                    b,
                ]);
                if border[3] > 0. {
                    let mut a = a;
                    let mut b = b;
                    a.color = border.map(|v| v as f32);
                    b.color = a.color;
                    a.textured = 0.;
                    b.textured = 0.;
                    line(
                        &mut batch,
                        a,
                        b,
                        if g("thickoutline").abs() >= 1. {
                            2.
                        } else {
                            1.
                        },
                        w,
                        h,
                    );
                }
            }
        }
        batches.push(batch);
    }
    for c in &mut model.waves {
        globals(c, s, main, &q_slots);
        c.frame.run(&mut c.values);
        let count = c.values.get(s, "samples").clamp(1., 512.) as usize;
        let spectrum = c.values.get(s, "spectrum") != 0.;
        let sep = c.values.get(s, "sep").floor() as isize;
        let scale = if spectrum { 0.15 } else { 0.004 }
            * c.values.get(s, "scaling")
            * main.get(s, "wave_scale");
        let mix = (c.values.get(s, "smoothing") * 0.98).max(0.).sqrt();
        let mut data = [[0_f64; 512]; 2];
        for channel in 0..2 {
            let start = if spectrum {
                0
            } else {
                (512 - count) as isize / 2 + if channel == 0 { -sep / 2 } else { sep / 2 }
            };
            for i in 0..count {
                let index = if spectrum {
                    (i as f64 * (512 - sep) as f64 / count as f64).floor() as isize
                } else {
                    i as isize + start
                };
                let point = if spectrum {
                    audio.spectrum[channel][index.clamp(0, 511) as usize] as f64
                } else {
                    audio.waveform[channel][index.clamp(0, 511) as usize] as f64 * 128.
                };
                data[channel][i] = if i == 0 {
                    point
                } else {
                    point * (1. - mix) + data[channel][i - 1] * mix
                };
            }
            for i in (0..count.saturating_sub(1)).rev() {
                data[channel][i] = data[channel][i] * (1. - mix) + data[channel][i + 1] * mix;
            }
            for v in &mut data[channel][..count] {
                *v *= scale;
            }
        }
        let mut batch = Batch {
            vertices: Vec::with_capacity(count * 6),
            additive: c.values.get(s, "additive") != 0.,
        };
        let dots = c.values.get(s, "usedots") != 0.;
        let size = if c.values.get(s, "thick") != 0. {
            2.
        } else {
            1.
        };
        let color = [
            c.values.get(s, "r"),
            c.values.get(s, "g"),
            c.values.get(s, "b"),
            c.values.get(s, "a"),
        ];
        let mut points = Vec::with_capacity(count);
        for i in 0..count {
            for (name, value) in [
                ("sample", i as f64 / (count - 1).max(1) as f64),
                ("value1", data[0][i]),
                ("value2", data[1][i]),
                ("x", 0.5 + data[0][i]),
                ("y", 0.5 + data[1][i]),
                ("r", color[0]),
                ("g", color[1]),
                ("b", color[2]),
                ("a", color[3]),
            ] {
                c.values.set(s, name, value);
            }
            c.point.run(&mut c.values);
            let g = |name: &str| c.values.get(s, name);
            let point = vertex(
                [(g("x") * 2. - 1.) / ax, (1. - g("y") * 2.) / ay],
                [0., 0.],
                [g("r"), g("g"), g("b"), g("a")],
                false,
            );
            points.push(point);
        }
        if dots {
            for point in points {
                dot(&mut batch, point, size, w, h);
            }
        } else {
            // MilkDrop cubic midpoints retain the source point's color.
            let mut smooth = Vec::with_capacity(count.saturating_mul(2));
            for i in 0..points.len().saturating_sub(1) {
                smooth.push(points[i]);
                let mut midpoint = points[i];
                for c in 0..2 {
                    midpoint.p[c] = (-0.15 * points[i.saturating_sub(1)].p[c]
                        + 1.15 * points[i].p[c]
                        + 1.15 * points[i + 1].p[c]
                        - 0.15 * points[(i + 2).min(points.len() - 1)].p[c])
                        * 0.5;
                }
                smooth.push(midpoint);
            }
            if let Some(last) = points.last() {
                smooth.push(*last);
            }
            for pair in smooth.windows(2) {
                line(&mut batch, pair[0], pair[1], size, w, h);
            }
        }
        batches.push(batch);
    }
    basic(model, audio, w, h, &mut batches);
    if model.get("darken_center") != 0. {
        let mut b = Batch {
            vertices: Vec::with_capacity(12),
            additive: false,
        };
        let points = [
            [-0.05 * ay, 0.],
            [0., -0.05],
            [0.05 * ay, 0.],
            [0., 0.05],
            [-0.05 * ay, 0.],
        ];
        for pair in points.windows(2) {
            b.vertices.extend_from_slice(&[
                vertex([0., 0.], [0., 0.], [0., 0., 0., 3. / 32.], false),
                vertex(pair[0], [0., 0.], [0.; 4], false),
                vertex(pair[1], [0., 0.], [0.; 4], false),
            ]);
        }
        batches.push(b);
    }

    let g = |name: &str| model.get(name);
    for (prefix, inset) in [("ob", 0.), ("ib", g("ob_size"))] {
        let size = g(&format!("{prefix}_size"));
        let a = g(&format!("{prefix}_a"));
        if size > 0. && a > 0. {
            let c = [
                g(&format!("{prefix}_r")),
                g(&format!("{prefix}_g")),
                g(&format!("{prefix}_b")),
                a,
            ];
            let mut b = Batch {
                vertices: Vec::with_capacity(24),
                additive: false,
            };
            let x = 1. - inset * 2.;
            let y = x;
            let d = size * 2.;
            for r in [
                [-x, -y, x, -y + d],
                [-x, y - d, x, y],
                [-x, -y + d, -x + d, y - d],
                [x - d, -y + d, x, y - d],
            ] {
                let v = [
                    vertex([r[0], r[1]], [0., 0.], c, false),
                    vertex([r[2], r[1]], [0., 0.], c, false),
                    vertex([r[0], r[3]], [0., 0.], c, false),
                    vertex([r[2], r[3]], [0., 0.], c, false),
                ];
                b.vertices
                    .extend_from_slice(&[v[0], v[1], v[2], v[2], v[1], v[3]]);
            }
            batches.push(b);
        }
    }
    batches
}
fn motion_vectors(model: &Model, _w: u32, _h: u32, batches: &mut Vec<Batch>) {
    let g = |n: &str| model.get(n);
    let alpha = if g("bmotionvectorson") == 0. {
        0.
    } else {
        g("mv_a")
    };
    let nx = g("mv_x").floor().clamp(0., 64.) as usize;
    let ny = g("mv_y").floor().clamp(0., 48.) as usize;
    if alpha <= 0.001 || nx == 0 || ny == 0 {
        return;
    }
    let fracx = if g("mv_x") > 64. {
        0.
    } else {
        g("mv_x").fract()
    };
    let fracy = if g("mv_y") > 48. {
        0.
    } else {
        g("mv_y").fract()
    };
    let mut batch = Batch {
        vertices: Vec::with_capacity(nx * ny * 6),
        additive: false,
    };
    let color = [g("mv_r"), g("mv_g"), g("mv_b"), alpha];
    for y in 0..ny {
        let fy = (y as f64 + 0.25) / (ny as f64 + fracy - 0.75) - g("mv_dy");
        if !(0.0001..0.9999).contains(&fy) {
            continue;
        }
        for x in 0..nx {
            let fx = (x as f64 + 0.25) / (nx as f64 + fracx - 0.75) + g("mv_dx");
            if !(0.0001..0.9999).contains(&fx) {
                continue;
            }
            let make = |end, side| {
                let mut v = vertex([fx, fy], [g("mv_l"), side], color, false);
                v.textured = if end { -2. } else { -1. };
                v
            };
            let (a, b, c, d) = (
                make(false, 1.),
                make(false, -1.),
                make(true, 1.),
                make(true, -1.),
            );
            batch.vertices.extend_from_slice(&[a, b, c, c, b, d]);
        }
    }
    batches.push(batch);
}
fn basic(model: &Model, audio: &Audio, w: u32, h: u32, batches: &mut Vec<Batch>) {
    let g = |name: &str| model.get(name);
    let mut alpha = g("wave_a");
    if alpha <= 0.001 {
        return;
    }
    let mode = g("wave_mode") as usize % 8;
    let volume = audio.bands.iter().sum::<f64>() / 3.;
    if g("modwavealphabyvolume") > 0. {
        let d = g("modwavealphaend") - g("modwavealphastart");
        alpha *= if d.abs() < 1e-6 {
            if volume >= g("modwavealphaend") {
                1.
            } else {
                0.
            }
        } else {
            (volume - g("modwavealphastart")) / d
        };
    }
    if [2, 3, 5].contains(&mode) {
        alpha *= if mode == 3 {
            if w < 1024 {
                0.15
            } else if w < 2048 {
                0.22
            } else {
                0.33
            }
        } else {
            if w < 1024 {
                0.09
            } else if w < 2048 {
                0.11
            } else {
                0.13
            }
        };
        if mode == 3 {
            alpha *= 1.3 * g("treb").powi(2);
        }
    }
    alpha = alpha.clamp(0., 1.);
    if alpha <= 0. {
        return;
    }
    let mut wave = [[0_f64; 512]; 2];
    let smooth = g("wave_smoothing");
    for c in 0..2 {
        wave[c][0] = audio.waveform[c][0] as f64 * g("wave_scale");
        for i in 1..512 {
            wave[c][i] = audio.waveform[c][i] as f64 * g("wave_scale") * (1. - smooth)
                + wave[c][i - 1] * smooth;
        }
    }
    let ax = (w as f64 / h as f64).min(1.);
    let ay = (h as f64 / w as f64).min(1.);
    let x = g("wave_x") * 2. - 1.;
    let y = g("wave_y") * 2. - 1.;
    let mystery = g("wave_mystery");
    let mut rgb = [
        g("wave_r").clamp(0., 1.),
        g("wave_g").clamp(0., 1.),
        g("wave_b").clamp(0., 1.),
    ];
    if g("wave_brighten") != 0. {
        let max = rgb.iter().copied().fold(0_f64, f64::max);
        if max > 0.01 {
            for c in &mut rgb {
                *c /= max;
            }
        }
    }
    let color = [rgb[0], rgb[1], rgb[2], alpha];
    let mut paths: Vec<Vec<[f64; 2]>> = Vec::new();
    match mode {
        0 => {
            let mut p = Vec::new();
            for i in 0..256 {
                let mut radius = 0.5 + 0.4 * wave[1][i + 127] + mystery;
                if i < 25 {
                    let t = 0.5 - 0.5 * (i as f64 / 25.7 * std::f64::consts::PI).cos();
                    radius = (1. - t) * (0.5 + 0.4 * wave[1][i + 384] + mystery) + t * radius;
                }
                let a = i as f64 / 256. * std::f64::consts::TAU + model.time * 0.2;
                p.push([radius * a.cos() * ay + x, radius * a.sin() * ax + y]);
            }
            p.push(p[0]);
            paths.push(p);
        }
        2 | 3 => {
            paths.push(
                (0..512)
                    .map(|i| [wave[1][i] * ay + x, wave[0][(i + 32) % 512] * ax + y])
                    .collect(),
            );
        }
        5 => {
            let a = model.time * 0.3;
            paths.push(
                (0..512)
                    .map(|i| {
                        let j = (i + 32) % 512;
                        let xx = wave[1][i] * wave[0][j] + wave[0][i] * wave[1][j];
                        let yy = wave[1][i].powi(2) - wave[0][j].powi(2);
                        [
                            (xx * a.cos() - yy * a.sin()) * (ay + x),
                            (xx * a.sin() + yy * a.cos()) * (ax + y),
                        ]
                    })
                    .collect(),
            );
        }
        7 | 6 => {
            let count = 256.min(w as usize / 3);
            let a = std::f64::consts::FRAC_PI_2 * mystery;
            let dx = a.cos();
            let dy = a.sin();
            let mut ex = [
                x * (a + std::f64::consts::FRAC_PI_2).cos() - dx * 3.,
                x * (a + std::f64::consts::FRAC_PI_2).cos() + dx * 3.,
            ];
            let mut ey = [
                x * (a + std::f64::consts::FRAC_PI_2).sin() - dy * 3.,
                x * (a + std::f64::consts::FRAC_PI_2).sin() + dy * 3.,
            ];
            for i in 0..2 {
                for side in 0..4 {
                    let (value, bound, clip) = match side {
                        0 => (ex[i], 1.1, ex[i] > 1.1),
                        1 => (ex[i], -1.1, ex[i] < -1.1),
                        2 => (ey[i], 1.1, ey[i] > 1.1),
                        _ => (ey[i], -1.1, ey[i] < -1.1),
                    };
                    if clip {
                        let other = if side < 2 { ex[1 - i] } else { ey[1 - i] };
                        let t = (bound - other) / (value - other);
                        ex[i] = ex[1 - i] + (ex[i] - ex[1 - i]) * t;
                        ey[i] = ey[1 - i] + (ey[i] - ey[1 - i]) * t;
                    }
                }
            }
            let dx = (ex[1] - ex[0]) / count as f64;
            let dy = (ey[1] - ey[0]) / count as f64;
            let angle = dy.atan2(dx) + std::f64::consts::FRAC_PI_2;
            for channel in 0..if mode == 7 { 2 } else { 1 } {
                let sep = if mode == 7 {
                    (y * 0.5 + 0.5).powi(2) * if channel == 0 { 1. } else { -1. }
                } else {
                    0.
                };
                paths.push(
                    (0..count)
                        .map(|i| {
                            let sample = wave[channel][i + (512 - count) / 2];
                            [
                                ex[0] + dx * i as f64 + angle.cos() * (0.25 * sample + sep),
                                ey[0] + dy * i as f64 + angle.sin() * (0.25 * sample + sep),
                            ]
                        })
                        .collect(),
                );
            }
        }
        _ => {
            return;
        }
    }
    let mut batch = Batch {
        vertices: Vec::new(),
        additive: g("additivewave") != 0.,
    };
    for path in paths {
        let mut smooth = Vec::with_capacity(path.len() * 2);
        for i in 0..path.len().saturating_sub(1) {
            smooth.push(path[i]);
            smooth.push(std::array::from_fn(|c| {
                (-0.15 * path[i.saturating_sub(1)][c] + 1.15 * path[i][c] + 1.15 * path[i + 1][c]
                    - 0.15 * path[(i + 2).min(path.len() - 1)][c])
                    * 0.5
            }));
        }
        if let Some(last) = path.last() {
            smooth.push(*last);
        }
        let path = smooth;
        let mut previous = None;
        for point in path {
            let v = vertex([point[0], -point[1]], [0., 0.], color, false);
            let size = if g("wave_thick") != 0. { 2. } else { 1. };
            if g("wave_dots") != 0. {
                dot(&mut batch, v, size, w, h);
            } else if let Some(p) = previous {
                line(&mut batch, p, v, size, w, h);
            }
            previous = Some(v);
        }
    }
    batches.push(batch);
}
const SHADER: &str = r#"
struct V{@location(0)p:vec2<f32>,@location(1)uv:vec2<f32>,@location(2)color:vec4<f32>,@location(3)textured:f32};
struct O{@builtin(position)p:vec4<f32>,@location(0)uv:vec2<f32>,@location(1)color:vec4<f32>,@location(2)textured:f32};
@group(0)@binding(0)var source:texture_2d<f32>;@group(0)@binding(1)var smp:sampler;
@group(0)@binding(2)var<storage,read> mesh:array<f32>;@group(0)@binding(3)var<uniform> dimensions:vec4<f32>;
fn mesh_uv(x:u32,y:u32)->vec2<f32>{let at=(y*65u+x)*10u;return vec2(mesh[at+2u],mesh[at+3u]);}
@vertex fn vs(v:V)->O{
 if v.textured>=0.{return O(vec4(v.p,0.,1.),v.uv,v.color,v.textured);}
 let grid=v.p*vec2(64.,48.);let base=vec2<u32>(floor(grid));let f=fract(grid);
 let warped=mix(mix(mesh_uv(base.x,base.y),mesh_uv(base.x+1u,base.y),f.x),mix(mesh_uv(base.x,base.y+1u),mesh_uv(base.x+1u,base.y+1u),f.x),f.y);
 var delta=(vec2(warped.x,1.-warped.y)-v.p)*v.uv.x;let minimum=1./dimensions.x;let distance=length(delta);if distance<minimum&&distance>0.00000001{delta*=minimum/distance;}else{delta.x=minimum;}
 let direction=delta*dimensions.xy;var normal=vec2(0.,1.);if length(direction)>0.{normal=normalize(vec2(-direction.y,direction.x));}
 let point=v.p*2.-1.+select(vec2(0.),delta*2.,v.textured< -1.5)+normal/dimensions.xy*v.uv.y;
 return O(vec4(point,0.,1.),vec2(0.),v.color,0.);
}
@fragment fn fs(v:O)->@location(0)vec4<f32>{return mix(v.color,textureSample(source,smp,vec2(v.uv.x,1.-v.uv.y))*v.color,v.textured);}
"#;
pub struct SpriteRenderer {
    normal: wgpu::RenderPipeline,
    additive: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    dimensions: wgpu::Buffer,
}
impl SpriteRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
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
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Preset shapes and waveforms"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let attrs = wgpu::vertex_attr_array![0=>Float32x2,1=>Float32x2,2=>Float32x4,3=>Float32];
        let make = |additive| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pl),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs"),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &attrs,
                    }],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: if additive {
                                    wgpu::BlendFactor::One
                                } else {
                                    wgpu::BlendFactor::OneMinusSrcAlpha
                                },
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent::OVER,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        Self {
            dimensions: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Primitive dimensions"),
                size: 16,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            normal: make(false),
            additive: make(true),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            layout,
        }
    }
    pub fn render(
        &self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        source: &wgpu::TextureView,
        mesh: &wgpu::Buffer,
        model: &mut Model,
        audio: &Audio,
        w: u32,
        h: u32,
    ) {
        gpu.queue.write_buffer(
            &self.dimensions,
            0,
            bytemuck::cast_slice(&[w as f32, h as f32, 0., 0.]),
        );
        let batches = geometry(model, audio, w, h);
        let buffers: Vec<_> = batches
            .iter()
            .filter(|b| !b.vertices.is_empty())
            .map(|b| {
                (
                    b.additive,
                    b.vertices.len() as u32,
                    gpu.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Preset primitives"),
                            contents: bytemuck::cast_slice(&b.vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        }),
                )
            })
            .collect();
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: mesh.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.dimensions.as_entire_binding(),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Custom shapes and waves"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_bind_group(0, &bind, &[]);
        for (additive, count, buffer) in &buffers {
            pass.set_pipeline(if *additive {
                &self.additive
            } else {
                &self.normal
            });
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..*count, 0..1);
        }
    }
}
