//! Curated MilkDrop-compatible renderer. Original preset programs and licenses
//! live in assets/visualizers and third_party/licenses/butterchurn.
pub mod catalog;

pub mod eel;
pub mod gpu;
pub mod model;
pub mod renderer;
pub mod shaders;
pub mod sprites;

pub mod control;

pub mod noise;

pub mod engine;
// Keep the host and renderer on one exact wgpu version.
pub use wgpu;

pub mod random;

pub mod pixel;

pub mod palette_transition;
