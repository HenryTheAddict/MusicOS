use crate::eel::{Program, Symbols, Values};
use anyhow::{Context, Result};
#[derive(Clone)]
pub struct Audio {
    pub waveform: [[f32; 512]; 2],
    pub spectrum: [[f32; 512]; 2],
    pub bands: [f64; 3],
    pub attenuated: [f64; 3],
}
impl Default for Audio {
    fn default() -> Self {
        Self {
            waveform: [[0.; 512]; 2],
            spectrum: [[0.; 512]; 2],
            bands: [1.; 3],
            attenuated: [1.; 3],
        }
    }
}
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub original: [f32; 2],
    pub color: [f32; 4],
}
pub struct Component {
    pub base: Vec<(usize, f64)>,
    pub initial: Program,
    pub frame: Program,
    pub point: Program,
    pub values: Values,
    pub initialized: bool,
    pub t_initial: Vec<(usize, f64)>,
}
pub struct Model {
    pub id: String,
    pub symbols: Symbols,
    pub values: Values,
    pub frame: Program,
    pub pixel: Program,
    pub shapes: Vec<Component>,
    pub waves: Vec<Component>,
    base: Vec<(usize, f64)>,
    q_init: Vec<(usize, f64)>,
    pixel_values: Values,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub time: f64,
    pub frames: u64,
}
fn base(value: &serde_json::Value, symbols: &mut Symbols) -> Vec<(usize, f64)> {
    value
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(k, v)| v.as_f64().map(|n| (symbols.slot(k), n)))
        .collect()
}
fn program(value: &serde_json::Value, key: &str, symbols: &mut Symbols) -> Result<Program> {
    Program::compile(value[key].as_str().unwrap_or(""), symbols)
        .with_context(|| format!("Preset {key}"))
}
impl Model {
    pub fn validate_frame(&self) -> Result<()> {
        let mode = self.get("wave_mode") as usize % 8;
        anyhow::ensure!(
            self.get("wave_a") <= 0.001 || [0, 2, 3, 5, 6, 7].contains(&mode),
            "{} requests unsupported basic waveform mode {mode}",
            self.id
        );
        Ok(())
    }
    pub fn load(id: &str, seed: u64) -> Result<Self> {
        Self::with_random(id, crate::random::Random::new(seed), 640, 480)
    }
    pub fn with_random(
        id: &str,
        random: crate::random::Random,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let source = crate::catalog::source(id)?;
        let mut symbols = Symbols::default();
        let mut defaults: serde_json::Value =
            serde_json::from_str(include_str!("../../../assets/visualizers/base-values.json"))?;
        for (k, v) in source["baseVals"]
            .as_object()
            .context("Missing base values")?
        {
            defaults[k] = v.clone();
        }
        for name in [
            "time", "fps", "frame", "bass", "mid", "treb", "bass_att", "mid_att", "treb_att",
            "vol", "vol_att", "meshx", "meshy", "aspectx", "aspecty", "pixelsx", "pixelsy", "x",
            "y", "rad", "ang", "sample", "value1", "value2", "instance", "b1n", "b1x", "b2n",
            "b2x", "b3n", "b3x", "b1ed",
        ] {
            symbols.slot(name);
        }
        for i in 1..=32 {
            symbols.slot(&format!("q{i}"));
        }
        for i in 0..100 {
            symbols.slot(&format!("reg{i:02}"));
        }
        let main_base = base(&defaults, &mut symbols);
        let initial = program(&source, "init_eqs_eel", &mut symbols)?;
        let frame = program(&source, "frame_eqs_eel", &mut symbols)?;
        let pixel = program(&source, "pixel_eqs_eel", &mut symbols)?;
        let component_defaults: serde_json::Value = serde_json::from_str(include_str!(
            "../../../assets/visualizers/component-defaults.json"
        ))?;
        let mut components = |key: &str| -> Result<Vec<Component>> {
            source[key]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|c| c["baseVals"]["enabled"].as_f64().unwrap_or(0.) != 0.)
                .map(|c| {
                    Ok(Component {
                        base: {
                            let mut merged = component_defaults
                                [if key == "shapes" { "shape" } else { "wave" }]
                            .clone();
                            for (k, v) in c["baseVals"].as_object().unwrap() {
                                merged[k] = v.clone();
                            }
                            base(&merged, &mut symbols)
                        },
                        initial: program(c, "init_eqs_eel", &mut symbols)?,
                        frame: program(c, "frame_eqs_eel", &mut symbols)?,
                        point: program(c, "point_eqs_eel", &mut symbols)?,
                        values: Values::with_random(&Symbols::default(), random.clone()),
                        initialized: false,
                        t_initial: Vec::new(),
                    })
                })
                .collect()
        };
        let mut shapes = components("shapes")?;
        let mut waves = components("waves")?;
        let mut values = Values::with_random(&symbols, random.clone());
        for (i, v) in &main_base {
            values.slots[*i] = *v;
        }
        values.set(&symbols, "fps", 30.);
        for (name, value) in [
            ("meshx", 64.),
            ("meshy", 48.),
            ("pixelsx", width as f64),
            ("pixelsy", height as f64),
            ("aspectx", (height as f64 / width as f64).max(1.)),
            ("aspecty", (width as f64 / height as f64).max(1.)),
        ] {
            values.set(&symbols, name, value);
        }
        values.set(&symbols, "b1x", 1.);
        values.set(&symbols, "b2x", 1.);
        values.set(&symbols, "b3x", 1.);
        for n in ["bass_att", "mid_att", "treb_att"] {
            values.set(&symbols, n, 1.);
        }
        initial.run(&mut values);
        let q_init: Vec<(usize, f64)> = (1..=32)
            .map(|i| {
                let slot = symbols.find(&format!("q{i}")).unwrap();
                (slot, values.slots[slot])
            })
            .collect();
        // The original runner evaluates the main frame once before component init.
        // Restore base controls while retaining only user variables and initialized q's.
        for (slot, value) in &main_base {
            values.slots[*slot] = *value;
        }
        frame.run(&mut values);
        for c in waves.iter_mut().chain(shapes.iter_mut()) {
            c.values = Values::with_random(&symbols, random.clone());
            initialize_component(c, &symbols, &values);
        }
        let pixel_values = values.clone();
        let mut indices = Vec::new();
        for y in 0..48 {
            for x in 0..64 {
                let a = x + y * 65;
                indices.extend_from_slice(&[a, a + 65, a + 1, a + 65, a + 66, a + 1]);
            }
        }
        Ok(Self {
            id: id.into(),
            symbols,
            values,
            frame,
            pixel,
            shapes,
            waves,
            base: main_base,
            q_init,
            pixel_values,
            vertices: (0..65 * 49)
                .map(|i| {
                    let x = (i % 65) as f32 / 32. - 1.;
                    let y = (i / 65) as f32 / 24. - 1.;
                    Vertex {
                        position: [x, -y],
                        uv: [x * 0.5 + 0.5, -y * 0.5 + 0.5],
                        original: [x * 0.5 + 0.5, -y * 0.5 + 0.5],
                        color: [1.; 4],
                    }
                })
                .collect(),
            indices,
            time: 0.,
            frames: 0,
        })
    }
    pub fn get(&self, name: &str) -> f64 {
        self.values.get(&self.symbols, name)
    }
    pub fn advance(&mut self, dt: f64, width: u32, height: u32, audio: &Audio) {
        self.advance_internal(dt, width, height, audio, true);
    }
    pub fn advance_frame(&mut self, dt: f64, width: u32, height: u32, audio: &Audio) {
        self.advance_internal(dt, width, height, audio, false);
    }
    fn advance_internal(
        &mut self,
        dt: f64,
        width: u32,
        height: u32,
        audio: &Audio,
        cpu_mesh: bool,
    ) {
        self.time += dt.clamp(0., 0.1);
        self.frames += 1;
        let s = &self.symbols;
        let v = &mut self.values;
        for (i, n) in self.base.iter().chain(self.q_init.iter()) {
            v.slots[*i] = *n;
        }
        let ax = (width as f64 / height as f64).min(1.);
        let ay = (height as f64 / width as f64).min(1.);
        for (name, value) in [
            ("time", self.time),
            ("frame", self.frames as f64),
            ("fps", 1. / dt.max(1. / 144.)),
            ("meshx", 64.),
            ("meshy", 48.),
            ("aspectx", 1. / ax),
            ("aspecty", 1. / ay),
            ("pixelsx", width as f64),
            ("pixelsy", height as f64),
            ("vol", audio.bands.iter().sum::<f64>() / 3.),
            ("vol_att", audio.attenuated.iter().sum::<f64>() / 3.),
        ] {
            v.set(s, name, value);
        }
        for (i, name) in ["bass", "mid", "treb"].iter().enumerate() {
            v.set(s, name, audio.bands[i]);
            v.set(s, &format!("{name}_att"), audio.attenuated[i]);
        }
        self.frame.run(v);
        if cpu_mesh {
            self.reference_mesh(width, height);
        }
    }
    /// Independent f64 pixel-equation/warp reference for the current main frame.
    /// GPU validation uses this after rendering, so main/component random state
    /// and initial conditions are identical on both paths.
    pub fn reference_mesh(&mut self, width: u32, height: u32) {
        let s = &self.symbols;
        let v = &self.values;
        let ax = (width as f64 / height as f64).min(1.);
        let ay = (height as f64 / width as f64).min(1.);
        self.pixel_values.slots.clone_from(&v.slots);
        let time = self.time * v.get(s, "warpanimspeed");
        let scale = 1. / v.get(s, "warpscale").max(1e-6);
        let wf = [
            11.68 + 4. * (time * 1.413 + 10.).cos(),
            8.77 + 3. * (time * 1.113 + 7.).cos(),
            10.54 + 3. * (time * 1.233 + 3.).cos(),
            11.49 + 4. * (time * 0.933 + 5.).cos(),
        ];
        let pixel_slots = [
            "zoom", "zoomexp", "rot", "warp", "cx", "cy", "dx", "dy", "sx", "sy", "x", "y", "rad",
            "ang",
        ]
        .map(|n| s.find(n).unwrap());
        for y in 0..=48 {
            for x in 0..=64 {
                let px = x as f64 / 32. - 1.;
                let py = y as f64 / 24. - 1.;
                let rad = (px * px * ax * ax + py * py * ay * ay).sqrt();
                let p = &mut self.pixel_values;
                for &slot in &pixel_slots[..10] {
                    p.slots[slot] = v.slots[slot];
                }
                for (slot, value) in pixel_slots[10..].iter().zip([
                    px * 0.5 * ax + 0.5,
                    -py * 0.5 * ay + 0.5,
                    rad,
                    if x == 32 && y == 24 {
                        0.
                    } else {
                        (py * ay).atan2(px * ax)
                    },
                ]) {
                    p.slots[*slot] = value;
                }
                self.pixel.run(p);
                let zoom =
                    p.slots[pixel_slots[0]].powf(p.slots[pixel_slots[1]].powf(rad * 2. - 1.));
                let mut u = px * 0.5 * ax / zoom + 0.5;
                let mut w = -py * 0.5 * ay / zoom + 0.5;
                let cx = p.slots[pixel_slots[4]];
                let cy = p.slots[pixel_slots[5]];
                u = (u - cx) / p.slots[pixel_slots[8]] + cx;
                w = (w - cy) / p.slots[pixel_slots[9]] + cy;
                let warp = p.slots[pixel_slots[3]] * 0.0035;
                if warp != 0. {
                    u += warp * (time * 0.333 + scale * (px * wf[0] - py * wf[3])).sin();
                    w += warp * (time * 0.375 - scale * (px * wf[2] + py * wf[1])).cos();
                    u += warp * (time * 0.753 - scale * (px * wf[1] - py * wf[2])).cos();
                    w += warp * (time * 0.825 + scale * (px * wf[0] + py * wf[3])).sin();
                }
                let a = u - cx;
                let b = w - cy;
                let r = p.slots[pixel_slots[2]];
                u = (a * r.cos() - b * r.sin() + cx - p.slots[pixel_slots[6]] - 0.5) / ax + 0.5;
                w = (a * r.sin() + b * r.cos() + cy - p.slots[pixel_slots[7]] - 0.5) / ay + 0.5;
                self.vertices[y * 65 + x] = Vertex {
                    position: [px as f32, -py as f32],
                    uv: [
                        if u.is_finite() { u as f32 } else { 0.5 },
                        if w.is_finite() { w as f32 } else { 0.5 },
                    ],
                    original: [(px * 0.5 + 0.5) as f32, (-py * 0.5 + 0.5) as f32],
                    color: [1., 1., 1., 1.],
                };
            }
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn every_preset_simulates_without_nonfinite_output() {
        for p in crate::catalog::catalog() {
            let mut model = super::Model::load(&p.id, 42).unwrap();
            for _ in 0..3 {
                model.advance(1. / 60., 720, 480, &Default::default());
            }
            assert!(model
                .vertices
                .iter()
                .all(|v| v.uv.iter().all(|x| x.is_finite())));
        }
    }
}

/// Components initialize at preset load, before any visible frame or shader randoms.
pub(crate) fn initialize_component(c: &mut Component, s: &Symbols, main: &Values) {
    for &(slot, value) in &c.base {
        c.values.slots[slot] = value;
    }
    for name in [
        "time", "frame", "fps", "bass", "mid", "treb", "bass_att", "mid_att", "treb_att", "vol",
        "vol_att", "aspectx", "aspecty", "pixelsx", "pixelsy", "meshx", "meshy",
    ] {
        c.values.set(s, name, main.get(s, name));
    }
    for name in (1..=32)
        .map(|i| format!("q{i}"))
        .chain((0..100).map(|i| format!("reg{i:02}")))
    {
        c.values.set(s, &name, main.get(s, &name));
    }
    c.initial.run(&mut c.values);
    c.t_initial = (1..=8)
        .filter_map(|i| {
            s.find(&format!("t{i}"))
                .map(|slot| (slot, c.values.slots[slot]))
        })
        .collect();
    c.initialized = true;
}
