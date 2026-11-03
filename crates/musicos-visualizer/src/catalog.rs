use serde::Deserialize;
#[derive(Clone, Debug, Deserialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub original_filename: String,
    pub authors: String,
    pub source_url: String,
    pub sha256: String,
    pub source: String,
}
#[derive(Deserialize)]
struct Manifest {
    presets: Vec<Preset>,
}
pub fn catalog() -> Vec<Preset> {
    serde_json::from_str::<Manifest>(include_str!("../../../assets/visualizers/manifest.json"))
        .expect("validated bundled visualizer manifest")
        .presets
}
pub fn source(id: &str) -> anyhow::Result<serde_json::Value> {
    let text = match id {
        "boxes" => include_str!("../../../assets/visualizers/original/boxes.json"),
        "oil-puddle" => include_str!("../../../assets/visualizers/original/oil-puddle.json"),
        "carbonated" => include_str!("../../../assets/visualizers/original/carbonated.json"),
        "watercolor" => include_str!("../../../assets/visualizers/original/watercolor.json"),
        "cells" => include_str!("../../../assets/visualizers/original/cells.json"),
        "tweaker" => include_str!("../../../assets/visualizers/original/tweaker.json"),
        "universe" => include_str!("../../../assets/visualizers/original/universe.json"),
        "mold" => include_str!("../../../assets/visualizers/original/mold.json"),
        "bazinga" => include_str!("../../../assets/visualizers/original/bazinga.json"),
        "infinite" => include_str!("../../../assets/visualizers/original/infinite.json"),
        "outfinate" => include_str!("../../../assets/visualizers/original/outfinate.json"),
        "paint-stir" => include_str!("../../../assets/visualizers/original/paint-stir.json"),
        "mucus" => include_str!("../../../assets/visualizers/original/mucus.json"),
        _ => anyhow::bail!("Unknown bundled visualizer"),
    };
    Ok(serde_json::from_str(text)?)
}
#[cfg(test)]
mod tests {
    #[test]
    fn only_the_requested_presets_are_bundled() {
        let list = super::catalog();
        assert_eq!(list.len(), 13);
        for p in list {
            let s = super::source(&p.id).unwrap();
            assert!(s["frame_eqs_eel"].is_string());
            assert!(s["warp"].is_string());
            assert_eq!(p.sha256.len(), 64);
        }
    }
}
