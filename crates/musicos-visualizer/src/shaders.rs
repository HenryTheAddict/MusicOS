//! Adapt the vendored GLSL shader bodies to explicit wgpu bindings. Shader
//! programs remain the original preset programs; no visual approximations.
use anyhow::{Context, Result};
pub const SCALARS: &[&str] = &[
    "time",
    "frame",
    "fps",
    "decay",
    "bass",
    "mid",
    "treb",
    "vol",
    "bass_att",
    "mid_att",
    "treb_att",
    "vol_att",
    "gammaAdj",
    "echo_zoom",
    "echo_alpha",
    "echo_orientation",
    "blur1_min",
    "blur1_max",
    "blur2_min",
    "blur2_max",
    "blur3_min",
    "blur3_max",
    "scale1",
    "scale2",
    "scale3",
    "bias1",
    "bias2",
    "bias3",
    "fShader",
];
pub const VECTORS: &[&str] = &[
    "aspect",
    "texsize",
    "resolution",
    "rand_frame",
    "rand_preset",
    "slow_roam_cos",
    "roam_cos",
    "slow_roam_sin",
    "roam_sin",
    "texsize_noise_lq",
    "texsize_noise_hq",
    "texsize_noise_mq",
    "texsize_noise_lq_lite",
    "texsize_noisevol_lq",
    "texsize_noisevol_hq",
];
pub const VECTOR_START: usize = 8;
pub const Q_START: usize = 24;
pub fn glsl(body: &str, comp: bool) -> String {
    let (head, body) = if let Some(at) = body.find("shader_body") {
        let first = body[at..].find('{').unwrap() + at;
        (&body[..at], &body[first + 1..body.rfind('}').unwrap()])
    } else {
        ("", body)
    };
    let mut text=String::from("#version 450\nlayout(set=0,binding=0,std140) uniform Globals {vec4 v[64];} G;\nlayout(location=0) in vec2 input_uv;\nlayout(location=1) in vec2 input_original;\nlayout(location=2) in vec4 vColor;\nlayout(location=0) out vec4 fragColor;\n");
    for (i, name) in SCALARS.iter().enumerate() {
        text += &format!(
            "#define {name} G.v[{}].{}\n",
            i / 4,
            ["x", "y", "z", "w"][i % 4]
        );
    }
    for (i, name) in VECTORS.iter().enumerate() {
        text += &format!(
            "#define {name} G.v[{}]{}\n",
            VECTOR_START + i,
            if *name == "resolution" { ".xy" } else { "" }
        );
    }
    for i in 0..32 {
        text += &format!(
            "#define q{} G.v[{}].{}\n",
            i + 1,
            Q_START + i / 4,
            ["x", "y", "z", "w"][i % 4]
        );
    }
    for (i, name) in ["main", "blur1", "blur2", "blur3", "noise_lq", "noise_hq"]
        .iter()
        .enumerate()
    {
        text += &format!(
            "layout(set=0,binding={}) uniform texture2D tex_{name};\n",
            i + 1
        );
    }
    for (i, name) in ["wrap", "clamp", "point_wrap", "point_clamp", "main"]
        .iter()
        .enumerate()
    {
        text += &format!(
            "layout(set=0,binding={}) uniform sampler samp_{name};\n",
            i + 7
        );
    }
    for (name, texture, sampler) in [
        ("main", "main", "main"),
        ("fw_main", "main", "wrap"),
        ("fc_main", "main", "clamp"),
        ("pw_main", "main", "point_wrap"),
        ("pc_main", "main", "point_clamp"),
        ("blur1", "blur1", "clamp"),
        ("blur2", "blur2", "clamp"),
        ("blur3", "blur3", "clamp"),
        ("noise_lq", "noise_lq", "wrap"),
        ("noise_hq", "noise_hq", "wrap"),
    ] {
        text += &format!("#define sampler_{name} sampler2D(tex_{texture},samp_{sampler})\n");
    }
    text+="const float PI=3.141592653589793;\nvec3 lum(vec3 value){return vec3(dot(value,vec3(0.32,0.49,0.29)));}\n";
    for name in [
        "main", "fw_main", "fc_main", "pw_main", "pc_main", "blur1", "blur2", "blur3",
    ] {
        text += &format!(
            "vec4 sample_{name}(vec2 p){{return texture(sampler_{name},vec2(p.x,1.0-p.y));}}\n"
        );
    }
    let rewrite = |source: &str| {
        let mut result = source.to_string();
        for name in [
            "main", "fw_main", "fc_main", "pw_main", "pc_main", "blur1", "blur2", "blur3",
        ] {
            // Curated GLSL exports use these two spellings; unknown forms are
            // rejected by the catalog shader tests rather than evaluated as JS.
            for pattern in [
                format!("texture (sampler_{name},"),
                format!("texture(sampler_{name},"),
            ] {
                result = result.replace(&pattern, &format!("sample_{name}("));
            }
        }
        result
    };
    text += &rewrite(head);
    text += "\nvoid main(){ vec3 ret=vec3(0.0);vec2 uv=input_uv;vec2 uv_orig=input_original;";
    if comp {
        text += "uv.y=1.0-uv.y;uv_orig.y=1.0-uv_orig.y;vec3 hue_shader=vColor.rgb;";
    }
    text += "float rad=length(uv_orig-0.5);float ang=atan(uv_orig.x-0.5,uv_orig.y-0.5);\n";
    text += &rewrite(body);
    text += if comp {
        "\nfragColor=vec4(ret,vColor.a);}"
    } else {
        "\nfragColor=vec4(ret,1.0)*vColor;}"
    };
    text
}
pub fn compile(body: &str, comp: bool) -> Result<String> {
    let source = glsl(body, comp);
    let module = naga::front::glsl::Frontend::default()
        .parse(
            &naga::front::glsl::Options {
                stage: naga::ShaderStage::Fragment,
                defines: Default::default(),
            },
            &source,
        )
        .map_err(|e| anyhow::anyhow!("{}", e.emit_to_string(&source)))?;
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .context("Invalid preset GPU program")?;
    naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::empty())
        .context("Could not compile preset to WGSL")
}
#[cfg(test)]
mod tests {
    #[test]
    fn every_original_gpu_program_is_supported() {
        for preset in crate::catalog::catalog() {
            let value = crate::catalog::source(&preset.id).unwrap();
            for pass in ["warp", "comp"] {
                super::compile(value[pass].as_str().unwrap(), pass == "comp")
                    .unwrap_or_else(|e| panic!("{} {pass}: {e:#}", preset.id));
            }
        }
    }
}
