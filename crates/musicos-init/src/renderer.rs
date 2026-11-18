use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use resvg::tiny_skia;
use rusttype::{point, Font, Scale};
use usvg::{Options, Transform, Tree};

use crate::assets;
use crate::framebuffer::FbDev;
use crate::ui::FontWeight;

#[derive(Clone)]
pub struct ScreenshotFrame {
    pub bytes: Arc<[u8]>,
    pub resolution: Resolution,
}

struct ScreenshotState {
    frame: Option<ScreenshotFrame>,
    last_update: Option<Instant>,
    last_demand: Option<Instant>,
}

static LATEST_SCREENSHOT: OnceLock<Mutex<ScreenshotState>> = OnceLock::new();

pub fn get_latest_screenshot() -> Option<ScreenshotFrame> {
    LATEST_SCREENSHOT.get().and_then(|mutex| {
        let mut state = mutex.lock().ok()?;
        state.last_demand = Some(Instant::now());
        state.frame.clone()
    })
}

fn screenshot_due(state: &ScreenshotState, now: Instant) -> bool {
    let interval = if state
        .last_demand
        .is_some_and(|last| now.saturating_duration_since(last) < Duration::from_secs(2))
    {
        Duration::from_millis(100)
    } else {
        Duration::from_secs(1)
    };
    state.frame.is_none()
        || state
            .last_update
            .is_none_or(|last| now.saturating_duration_since(last) >= interval)
}

pub fn update_latest_screenshot(bytes: &[u8], resolution: Resolution) {
    let mutex = LATEST_SCREENSHOT.get_or_init(|| {
        Mutex::new(ScreenshotState {
            frame: None,
            last_update: None,
            last_demand: None,
        })
    });
    if let Ok(mut guard) = mutex.lock() {
        if !screenshot_due(&guard, Instant::now()) {
            return;
        }
        guard.frame = Some(ScreenshotFrame {
            bytes: Arc::from(bytes),
            resolution,
        });
        guard.last_update = Some(Instant::now());
    }
}

static ICON_CACHE: OnceLock<Mutex<HashMap<String, tiny_skia::Pixmap>>> = OnceLock::new();

#[cfg(test)]
mod screenshot_tests {
    use super::*;

    #[test]
    fn screenshot_publication_uses_fast_demand_and_slow_background_intervals() {
        let now = Instant::now();
        let mut state = ScreenshotState {
            frame: None,
            last_update: None,
            last_demand: None,
        };
        assert!(screenshot_due(&state, now));
        state.frame = Some(ScreenshotFrame {
            bytes: Arc::from([0_u8; 4]),
            resolution: Resolution {
                width: 1,
                height: 1,
            },
        });
        state.last_update = Some(now);
        assert!(!screenshot_due(&state, now + Duration::from_millis(999)));
        assert!(screenshot_due(&state, now + Duration::from_secs(1)));
        assert!(screenshot_due(&state, now + Duration::from_secs(10)));
        state.last_demand = Some(now);
        assert!(!screenshot_due(&state, now + Duration::from_millis(99)));
        assert!(screenshot_due(&state, now + Duration::from_millis(100)));
        assert!(screenshot_due(&state, now + Duration::from_millis(1999)));
        state.last_update = Some(now + Duration::from_millis(1900));
        assert!(!screenshot_due(&state, now + Duration::from_secs(2)));
        assert!(!screenshot_due(&state, now + Duration::from_millis(2899)));
        assert!(screenshot_due(&state, now + Duration::from_millis(2900)));
        state.last_demand = Some(now + Duration::from_secs(10));
        assert!(screenshot_due(&state, now + Duration::from_secs(10)));
    }
}

fn get_svg_pixmap(path: &str, width: u32, height: u32) -> Option<tiny_skia::Pixmap> {
    let mut cache = ICON_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap();
    let key = format!("{}_{}x{}", path, width, height);
    if let Some(pixmap) = cache.get(&key) {
        return Some(pixmap.clone());
    }

    let svg_data = std::fs::read(path).ok()?;
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    let mut opt = Options::default();
    opt.fontdb = std::sync::Arc::new(fontdb);

    let tree = Tree::from_data(&svg_data, &opt).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;

    let svg_width = tree.size().width();
    let svg_height = tree.size().height();
    let scale_x = width as f32 / svg_width;
    let scale_y = height as f32 / svg_height;
    let transform = Transform::from_scale(scale_x, scale_y);

    resvg::render(&tree, transform, &mut pixmap.as_mut());
    cache.insert(key.clone(), pixmap.clone());
    Some(pixmap)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub const HD_4_3: Self = Self {
        width: 1280,
        height: 960,
    };
    pub const VGA_4_3: Self = Self {
        width: 640,
        height: 480,
    };

    pub fn from_env_or_default() -> Self {
        if let Ok(value) = std::env::var("MUSICOS_RESOLUTION") {
            return Self::parse(&value).unwrap_or(Self::HD_4_3);
        }
        if let Ok(size_str) = std::fs::read_to_string("/sys/class/graphics/fb0/virtual_size") {
            let parts: Vec<&str> = size_str.trim().split(',').collect();
            if parts.len() == 2 {
                if let (Ok(w), Ok(h)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    return Self {
                        width: w,
                        height: h,
                    };
                }
            }
        }
        std::fs::read_to_string("/proc/cmdline")
            .ok()
            .and_then(|cmdline| {
                cmdline.split_whitespace().find_map(|part| {
                    part.strip_prefix("MUSICOS_RESOLUTION=")
                        .and_then(Self::parse)
                })
            })
            .unwrap_or(Self::HD_4_3)
    }

    fn parse(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split('x').collect();
        if parts.len() == 2 {
            let w = parts[0].parse::<u32>().ok()?;
            let h = parts[1].parse::<u32>().ok()?;
            Some(Self {
                width: w,
                height: h,
            })
        } else {
            None
        }
    }

    pub fn offsets(&self) -> (f32, f32, f32) {
        let logical_aspect = 1280.0 / 960.0;
        let target_aspect = self.width as f32 / self.height as f32;

        if (target_aspect - logical_aspect).abs() < 0.01 {
            (self.width as f32 / 1280.0, 0.0, 0.0)
        } else if target_aspect > logical_aspect {
            let scale = self.height as f32 / 960.0;
            let actual_w = 1280.0 * scale;
            let dx = (self.width as f32 - actual_w) / 2.0;
            (scale, dx, 0.0)
        } else {
            let scale = self.width as f32 / 1280.0;
            let actual_h = 960.0 * scale;
            let dy = (self.height as f32 - actual_h) / 2.0;
            (scale, 0.0, dy)
        }
    }

    pub fn scale(&self) -> f32 {
        self.offsets().0
    }

    pub fn scale_x(&self, value: f32) -> f32 {
        let (scale, dx, _) = self.offsets();
        dx + value * scale
    }

    pub fn scale_y(&self, value: f32) -> f32 {
        let (scale, _, dy) = self.offsets();
        dy + value * scale
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const BLUE: Self = Self::rgb(35, 108, 220);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    pub fn mix(self, other: Color, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let inv = 1.0 - t;
        Self::rgba(
            (self.r as f32 * inv + other.r as f32 * t).round() as u8,
            (self.g as f32 * inv + other.g as f32 * t).round() as u8,
            (self.b as f32 * inv + other.b as f32 * t).round() as u8,
            (self.a as f32 * inv + other.a as f32 * t).round() as u8,
        )
    }

    pub fn over(self, dst: Color) -> Color {
        let alpha = self.a as u32;
        let inv = 255 - alpha;
        Color::rgb(
            ((self.r as u32 * alpha + dst.r as u32 * inv) / 255) as u8,
            ((self.g as u32 * alpha + dst.g as u32 * inv) / 255) as u8,
            ((self.b as u32 * alpha + dst.b as u32 * inv) / 255) as u8,
        )
    }
}

#[derive(Clone, Debug)]
pub struct CoverArt {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Color>,
}

pub enum Renderer {
    Fb(FbDev),
    Memory(MemorySurface),
}

impl Renderer {
    pub fn open(resolution: Resolution) -> Result<Self> {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if Path::new("/dev/dri/card0").exists() {
                log::info!(
                    "DRM card detected; using framebuffer fallback until dumb-buffer path is enabled"
                );
            }

            if let Ok(fb) = FbDev::open("/dev/fb0", resolution) {
                return Ok(Self::Fb(fb));
            }

            if Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        log::info!("no framebuffer device found; using in-memory surface");
        Ok(Self::Memory(MemorySurface::new(resolution)))
    }

    pub fn draw(&mut self, f: impl FnOnce(&mut Canvas<'_>)) -> Result<()> {
        match self {
            Renderer::Fb(fb) => fb.draw(|canvas| {
                f(canvas);
                update_latest_screenshot(canvas.buffer, canvas.resolution);
            }),
            Renderer::Memory(surface) => {
                let mut canvas = surface.canvas()?;
                f(&mut canvas);
                update_latest_screenshot(canvas.buffer, canvas.resolution);
                Ok(())
            }
        }
    }
}

pub struct MemorySurface {
    resolution: Resolution,
    buffer: Vec<u8>,
}

impl MemorySurface {
    pub fn new(resolution: Resolution) -> Self {
        let len = resolution.width as usize * resolution.height as usize * 4;
        Self {
            resolution,
            buffer: vec![0; len],
        }
    }

    pub fn canvas(&mut self) -> Result<Canvas<'_>> {
        Canvas::new(self.resolution, &mut self.buffer)
    }
}

pub struct Canvas<'a> {
    resolution: Resolution,
    stride: usize,
    buffer: &'a mut [u8],
    regular: Font<'static>,
    bold: Font<'static>,
    regular_ja: Font<'static>,
    regular_zh: Font<'static>,
    pub language: crate::ui::localization::Language,
    pub selected_box: Option<(f32, f32, f32, f32, f32, f32)>, // x, y, w, h, r, thick
    pub theme_top_color: Option<Color>,
    pub theme_bottom_color: Option<Color>,
}

impl<'a> Canvas<'a> {
    pub fn new(resolution: Resolution, buffer: &'a mut [u8]) -> Result<Self> {
        let expected = resolution.width as usize * resolution.height as usize * 4;
        if buffer.len() < expected {
            return Err(anyhow!("surface too small: {} < {expected}", buffer.len()));
        }
        Ok(Self {
            resolution,
            stride: resolution.width as usize * 4,
            buffer,
            regular: assets::regular_font()?,
            bold: assets::bold_font()?,
            regular_ja: assets::regular_font_ja()?,
            regular_zh: assets::regular_font_zh()?,
            language: crate::ui::localization::Language::En,
            selected_box: None,
            theme_top_color: None,
            theme_bottom_color: None,
        })
    }

    pub fn register_selected(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, thick: f32) {
        self.selected_box = Some((x, y, w, h, r, thick));
    }

    pub fn resolution(&self) -> Resolution {
        self.resolution
    }

    pub fn clear(&mut self, color: Color) {
        if color.a == 0 {
            return;
        }
        if color.a == 255 {
            for chunk in self.buffer.chunks_exact_mut(4) {
                chunk[0] = color.b;
                chunk[1] = color.g;
                chunk[2] = color.r;
                chunk[3] = 255;
            }
        } else {
            for y in 0..self.resolution.height as i32 {
                for x in 0..self.resolution.width as i32 {
                    self.put_pixel(x, y, color);
                }
            }
        }
    }

    pub fn draw_svg(&mut self, path: &str, x: f32, y: f32, w: f32, h: f32) {
        let res = self.resolution();
        let scaled_w = (w * res.scale()).round() as u32;
        let scaled_h = (h * res.scale()).round() as u32;
        if scaled_w == 0 || scaled_h == 0 {
            return;
        }

        if let Some(pixmap) = get_svg_pixmap(path, scaled_w, scaled_h) {
            let x0 = res.scale_x(x).round() as i32;
            let y0 = res.scale_y(y).round() as i32;

            let pixels = pixmap.pixels();
            for py in 0..scaled_h {
                for px in 0..scaled_w {
                    let pixel = pixels[(py * scaled_w + px) as usize];
                    let a = pixel.alpha();
                    if a > 0 {
                        let r = pixel.red();
                        let g = pixel.green();
                        let b = pixel.blue();

                        let up_r = if a == 255 {
                            r
                        } else {
                            (r as u32 * 255 / a as u32) as u8
                        };
                        let up_g = if a == 255 {
                            g
                        } else {
                            (g as u32 * 255 / a as u32) as u8
                        };
                        let up_b = if a == 255 {
                            b
                        } else {
                            (b as u32 * 255 / a as u32) as u8
                        };

                        self.put_pixel(
                            x0 + px as i32,
                            y0 + py as i32,
                            Color::rgba(up_r, up_g, up_b, a),
                        );
                    }
                }
            }
        } else {
            log::warn!("Failed to render SVG: {}", path);
            self.draw_svg_fallback(x, y, w, h);
        }
    }

    fn draw_svg_fallback(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let res = self.resolution();
        let x = res.scale_x(x);
        let y = res.scale_y(y);
        let w = w * res.scale();
        let h = h * res.scale();
        if w < 8.0 || h < 8.0 {
            return;
        }
        let r = (w.min(h) * 0.22).clamp(4.0, 18.0);
        self.rounded_rect(x, y, w, h, r, Color::rgba(255, 255, 255, 180));
        self.rounded_rect_stroke(x, y, w, h, r, 2.0, Color::rgba(0, 0, 0, 210));
        self.line(
            x + w * 0.28,
            y + h * 0.28,
            x + w * 0.72,
            y + h * 0.72,
            3.0,
            Color::BLACK,
        );
        self.line(
            x + w * 0.72,
            y + h * 0.28,
            x + w * 0.28,
            y + h * 0.72,
            3.0,
            Color::BLACK,
        );
    }

    pub fn draw_image(&mut self, image: &CoverArt, x: f32, y: f32, w: f32, h: f32) {
        let res = self.resolution();
        let scaled_w = (w * res.scale()).round() as i32;
        let scaled_h = (h * res.scale()).round() as i32;
        if scaled_w <= 0 || scaled_h <= 0 {
            return;
        }

        let x0 = res.scale_x(x).round() as i32;
        let y0 = res.scale_y(y).round() as i32;

        for py in 0..scaled_h {
            let src_y = (py as f32 / scaled_h as f32 * image.height as f32) as u32;
            let src_y = src_y.min(image.height - 1);
            for px in 0..scaled_w {
                let src_x = (px as f32 / scaled_w as f32 * image.width as f32) as u32;
                let src_x = src_x.min(image.width - 1);

                let idx = (src_y * image.width + src_x) as usize;
                let color = image.pixels[idx];
                if color.a > 0 {
                    self.put_pixel(x0 + px, y0 + py, color);
                }
            }
        }
    }

    pub fn draw_image_rounded(
        &mut self,
        image: &CoverArt,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        opacity: f32,
    ) {
        if opacity <= 0.0 {
            return;
        }
        let res = self.resolution();
        let scaled_w = (w * res.scale()).round() as i32;
        let scaled_h = (h * res.scale()).round() as i32;
        if scaled_w <= 0 || scaled_h <= 0 {
            return;
        }

        let x0 = res.scale_x(x).round() as i32;
        let y0 = res.scale_y(y).round() as i32;
        let sw = scaled_w as f32;
        let sh = scaled_h as f32;
        let sr = (r * res.scale()).max(0.0).min(sw / 2.0).min(sh / 2.0);

        // Fixed-point scale factor: 16.16 format
        let step_y = ((image.height as u64) << 16) / (scaled_h as u64);
        let step_x = ((image.width as u64) << 16) / (scaled_w as u64);

        let mut src_y_accum = 0u64;

        for py in 0..scaled_h {
            let dst_y = y0 + py;
            let src_y = ((src_y_accum >> 16) as u32).min(image.height - 1);
            src_y_accum += step_y;

            let mut src_x_accum = 0u64;

            let in_top_corner_zone = (py as f32) < sr;
            let in_bottom_corner_zone = (py as f32) >= sh - sr;

            for px in 0..scaled_w {
                let dst_x = x0 + px;
                let src_x = ((src_x_accum >> 16) as u32).min(image.width - 1);
                src_x_accum += step_x;

                let mut is_inside = true;
                if in_top_corner_zone {
                    if (px as f32) < sr {
                        let dx = (px as f32 + 0.5) - sr;
                        let dy = (py as f32 + 0.5) - sr;
                        if dx * dx + dy * dy > sr * sr {
                            is_inside = false;
                        }
                    } else if (px as f32) >= sw - sr {
                        let dx = (px as f32 + 0.5) - (sw - sr);
                        let dy = (py as f32 + 0.5) - sr;
                        if dx * dx + dy * dy > sr * sr {
                            is_inside = false;
                        }
                    }
                } else if in_bottom_corner_zone {
                    if (px as f32) < sr {
                        let dx = (px as f32 + 0.5) - sr;
                        let dy = (py as f32 + 0.5) - (sh - sr);
                        if dx * dx + dy * dy > sr * sr {
                            is_inside = false;
                        }
                    } else if (px as f32) >= sw - sr {
                        let dx = (px as f32 + 0.5) - (sw - sr);
                        let dy = (py as f32 + 0.5) - (sh - sr);
                        if dx * dx + dy * dy > sr * sr {
                            is_inside = false;
                        }
                    }
                }

                if !is_inside {
                    continue;
                }

                let idx = (src_y * image.width + src_x) as usize;
                let color = image.pixels[idx];
                let alpha = (color.a as f32 * opacity).round() as u8;
                if alpha > 0 {
                    let mut faded_color = color;
                    faded_color.a = alpha;
                    self.put_pixel(dst_x, dst_y, faded_color);
                }
            }
        }
    }

    pub fn put_pixel(&mut self, x: i32, y: i32, color: Color) {
        if color.a == 0 {
            return;
        }
        if x < 0 || y < 0 || x >= self.resolution.width as i32 || y >= self.resolution.height as i32
        {
            return;
        }
        let offset = y as usize * self.stride + x as usize * 4;
        if color.a == 255 {
            self.buffer[offset] = color.b;
            self.buffer[offset + 1] = color.g;
            self.buffer[offset + 2] = color.r;
            self.buffer[offset + 3] = 255;
        } else {
            let dst = Color::rgb(
                self.buffer[offset + 2],
                self.buffer[offset + 1],
                self.buffer[offset],
            );
            let out = color.over(dst);
            self.buffer[offset] = out.b;
            self.buffer[offset + 1] = out.g;
            self.buffer[offset + 2] = out.r;
            self.buffer[offset + 3] = 255;
        }
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        if color.a == 0 {
            return;
        }
        let x0 = (x.round() as i32).max(0);
        let y0 = (y.round() as i32).max(0);
        let x1 = ((x + w).round() as i32).min(self.resolution.width as i32);
        let y1 = ((y + h).round() as i32).min(self.resolution.height as i32);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        if color.a == 255 {
            for yy in y0..y1 {
                let row_offset = yy as usize * self.stride;
                for xx in x0..x1 {
                    let offset = row_offset + xx as usize * 4;
                    self.buffer[offset] = color.b;
                    self.buffer[offset + 1] = color.g;
                    self.buffer[offset + 2] = color.r;
                    self.buffer[offset + 3] = 255;
                }
            }
        } else {
            for yy in y0..y1 {
                let row_offset = yy as usize * self.stride;
                for xx in x0..x1 {
                    let offset = row_offset + xx as usize * 4;
                    let dst = Color::rgb(
                        self.buffer[offset + 2],
                        self.buffer[offset + 1],
                        self.buffer[offset],
                    );
                    let out = color.over(dst);
                    self.buffer[offset] = out.b;
                    self.buffer[offset + 1] = out.g;
                    self.buffer[offset + 2] = out.r;
                    self.buffer[offset + 3] = 255;
                }
            }
        }
    }

    pub fn vertical_gradient(&mut self, top: Color, bottom: Color) {
        let height = self.resolution.height.max(1);
        for y in 0..height {
            let t = y as f32 / height.saturating_sub(1).max(1) as f32;
            self.rect(
                0.0,
                y as f32,
                self.resolution.width as f32,
                1.0,
                top.mix(bottom, t),
            );
        }
    }

    pub fn rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let x1 = (x + w).ceil() as i32;
        let y1 = (y + h).ceil() as i32;
        for yy in y0..y1 {
            for xx in x0..x1 {
                if Self::point_in_rounded_rect(xx as f32 + 0.5, yy as f32 + 0.5, x, y, w, h, r) {
                    self.put_pixel(xx, yy, color);
                }
            }
        }
    }

    pub fn rounded_rect_vertical_gradient(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        top: Color,
        bottom: Color,
    ) {
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let x1 = (x + w).ceil() as i32;
        let y1 = (y + h).ceil() as i32;
        let denom = (h - 1.0).max(1.0);
        for yy in y0..y1 {
            let py = yy as f32 + 0.5;
            let t = ((py - y) / denom).clamp(0.0, 1.0);
            let color = top.mix(bottom, t);
            for xx in x0..x1 {
                if Self::point_in_rounded_rect(xx as f32 + 0.5, py, x, y, w, h, r) {
                    self.put_pixel(xx, yy, color);
                }
            }
        }
    }

    pub fn rounded_rect_three_stop_vertical_gradient(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        top: Color,
        middle: Color,
        bottom: Color,
        middle_at: f32,
    ) {
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let x1 = (x + w).ceil() as i32;
        let y1 = (y + h).ceil() as i32;
        let denom = (h - 1.0).max(1.0);
        let middle_at = middle_at.clamp(0.01, 0.99);
        for yy in y0..y1 {
            let py = yy as f32 + 0.5;
            let t = ((py - y) / denom).clamp(0.0, 1.0);
            let color = if t <= middle_at {
                top.mix(middle, t / middle_at)
            } else {
                middle.mix(bottom, (t - middle_at) / (1.0 - middle_at))
            };
            for xx in x0..x1 {
                if Self::point_in_rounded_rect(xx as f32 + 0.5, py, x, y, w, h, r) {
                    self.put_pixel(xx, yy, color);
                }
            }
        }
    }

    pub fn rounded_rect_stroke(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        thickness: f32,
        color: Color,
    ) {
        let thickness = thickness.max(1.0);
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let x1 = (x + w).ceil() as i32;
        let y1 = (y + h).ceil() as i32;
        let inner_x = x + thickness;
        let inner_y = y + thickness;
        let inner_w = w - thickness * 2.0;
        let inner_h = h - thickness * 2.0;
        let inner_r = (r - thickness).max(0.0);
        for yy in y0..y1 {
            for xx in x0..x1 {
                let px = xx as f32 + 0.5;
                let py = yy as f32 + 0.5;
                let outer = Self::point_in_rounded_rect(px, py, x, y, w, h, r);
                let inner = inner_w > 0.0
                    && inner_h > 0.0
                    && Self::point_in_rounded_rect(
                        px, py, inner_x, inner_y, inner_w, inner_h, inner_r,
                    );
                if outer && !inner {
                    self.put_pixel(xx, yy, color);
                }
            }
        }
    }

    pub fn shadow_rounded_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        color: Color,
        offset_y: f32,
        blur: f32,
    ) {
        let layers = blur.max(1.0).round() as i32;
        let base_alpha = color.a as f32 / layers as f32;
        for i in 1..=layers {
            let grow = i as f32;
            let a = (base_alpha * (1.0 - (i as f32 / layers as f32))).round() as u8;
            if a > 0 {
                let shadow_x = x - grow;
                let shadow_y = y + offset_y - grow;
                let shadow_w = w + grow * 2.0;
                let shadow_h = h + grow * 2.0;
                let shadow_r = r + grow;

                let x0 = shadow_x.floor() as i32;
                let y0 = shadow_y.floor() as i32;
                let x1 = (shadow_x + shadow_w).ceil() as i32;
                let y1 = (shadow_y + shadow_h).ceil() as i32;

                for yy in y0..y1 {
                    for xx in x0..x1 {
                        let px = xx as f32 + 0.5;
                        let py = yy as f32 + 0.5;
                        if Self::point_in_rounded_rect(
                            px, py, shadow_x, shadow_y, shadow_w, shadow_h, shadow_r,
                        ) {
                            if !Self::point_in_rounded_rect(px, py, x, y, w, h, r) {
                                self.put_pixel(xx, yy, color.with_alpha(a));
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn liquid_glass_rounded_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        opacity: u8,
    ) {
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let x1 = (x + w).ceil() as i32;
        let y1 = (y + h).ceil() as i32;
        let denom = (h - 1.0).max(1.0);
        for yy in y0..y1 {
            let py = yy as f32 + 0.5;
            let t = ((py - y) / denom).clamp(0.0, 1.0);
            let highlight = (1.0 - t * 1.8).max(0.0);
            let base_alpha = opacity as f32 / 255.0;
            let combined_alpha = (base_alpha + highlight * 0.08).min(1.0);
            let white_a = (combined_alpha * 255.0).round() as u8;
            for xx in x0..x1 {
                let px = xx as f32 + 0.5;
                if Self::point_in_rounded_rect(px, py, x, y, w, h, r) {
                    self.put_pixel(xx, yy, Color::rgba(255, 255, 255, white_a));
                }
            }
        }
        let edge_thickness = 2.0;
        for yy in y0..(y0 + (edge_thickness * 2.0) as i32).min(y1) {
            let py = yy as f32 + 0.5;
            let edge_t = ((py - y) / (edge_thickness * 2.0)).clamp(0.0, 1.0);
            let edge_a = ((1.0 - edge_t) * 48.0).round() as u8;
            for xx in x0..x1 {
                let px = xx as f32 + 0.5;
                if Self::point_in_rounded_rect(px, py, x, y, w, h, r) {
                    self.put_pixel(xx, yy, Color::rgba(255, 255, 255, edge_a));
                }
            }
        }
    }

    pub fn inner_shadow_rounded_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        shadow_size: f32,
    ) {
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let x1 = (x + w).ceil() as i32;
        let y1 = (y + h).ceil() as i32;
        for yy in y0..y1 {
            let py = yy as f32 + 0.5;
            let dist_from_bottom = (y + h) - py;
            if dist_from_bottom > shadow_size {
                continue;
            }
            let t = (1.0 - dist_from_bottom / shadow_size).clamp(0.0, 1.0);
            let a = (t * t * 24.0).round() as u8;
            for xx in x0..x1 {
                let px = xx as f32 + 0.5;
                if Self::point_in_rounded_rect(px, py, x, y, w, h, r) {
                    self.put_pixel(xx, yy, Color::rgba(0, 0, 0, a));
                }
            }
        }
    }

    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, thickness: f32, color: Color) {
        let steps = (x1 - x0).abs().max((y1 - y0).abs()).ceil() as i32;
        if steps <= 0 {
            return;
        }
        let radius = (thickness / 2.0).max(1.0);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = x0 + (x1 - x0) * t;
            let y = y0 + (y1 - y0) * t;
            self.circle(x, y, radius, color);
        }
    }

    pub fn bezier_curve(
        &mut self,
        x0: f32,
        y0: f32,
        cx0: f32,
        cy0: f32,
        cx1: f32,
        cy1: f32,
        x1: f32,
        y1: f32,
        thickness: f32,
        color: Color,
    ) {
        let steps = 20;
        let mut prev_x = x0;
        let mut prev_y = y0;
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let mt = 1.0 - t;
            let b0 = mt * mt * mt;
            let b1 = 3.0 * mt * mt * t;
            let b2 = 3.0 * mt * t * t;
            let b3 = t * t * t;
            let x = b0 * x0 + b1 * cx0 + b2 * cx1 + b3 * x1;
            let y = b0 * y0 + b1 * cy0 + b2 * cy1 + b3 * y1;
            self.line(prev_x, prev_y, x, y, thickness, color);
            prev_x = x;
            prev_y = y;
        }
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, color: Color) {
        let x0 = (cx - r).floor() as i32;
        let y0 = (cy - r).floor() as i32;
        let x1 = (cx + r).ceil() as i32;
        let y1 = (cy + r).ceil() as i32;
        let rr = r * r;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                if dx * dx + dy * dy <= rr {
                    self.put_pixel(x, y, color);
                }
            }
        }
    }

    pub fn circle_vertical_gradient(
        &mut self,
        cx: f32,
        cy: f32,
        r: f32,
        top: Color,
        bottom: Color,
    ) {
        let x0 = (cx - r).floor() as i32;
        let y0 = (cy - r).floor() as i32;
        let x1 = (cx + r).ceil() as i32;
        let y1 = (cy + r).ceil() as i32;
        let rr = r * r;
        let denom = (r * 2.0 - 1.0).max(1.0);
        for y in y0..=y1 {
            let py = y as f32 + 0.5;
            let t = ((py - (cy - r)) / denom).clamp(0.0, 1.0);
            let color = top.mix(bottom, t);
            for x in x0..=x1 {
                let dx = x as f32 + 0.5 - cx;
                let dy = py - cy;
                if dx * dx + dy * dy <= rr {
                    self.put_pixel(x, y, color);
                }
            }
        }
    }

    pub fn circle_stroke(&mut self, cx: f32, cy: f32, r: f32, thickness: f32, color: Color) {
        let x0 = (cx - r).floor() as i32;
        let y0 = (cy - r).floor() as i32;
        let x1 = (cx + r).ceil() as i32;
        let y1 = (cy + r).ceil() as i32;
        let outer = r * r;
        let inner_r = (r - thickness).max(0.0);
        let inner = inner_r * inner_r;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let d = dx * dx + dy * dy;
                if d <= outer && d >= inner {
                    self.put_pixel(x, y, color);
                }
            }
        }
    }

    fn point_in_rounded_rect(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> bool {
        if w <= 0.0 || h <= 0.0 || px < x || py < y || px >= x + w || py >= y + h {
            return false;
        }
        let r = r.max(0.0).min(w / 2.0).min(h / 2.0);
        if r <= 0.0 {
            return true;
        }
        if (px >= x + r && px <= x + w - r) || (py >= y + r && py <= y + h - r) {
            return true;
        }
        let cx = if px < x + r { x + r } else { x + w - r };
        let cy = if py < y + r { y + r } else { y + h - r };
        let dx = px - cx;
        let dy = py - cy;
        dx * dx + dy * dy <= r * r
    }

    pub fn text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
        weight: FontWeight,
    ) {
        let primary = match self.language {
            crate::ui::localization::Language::Ja => self.regular_ja.clone(),
            crate::ui::localization::Language::Zh => self.regular_zh.clone(),
            _ => match weight {
                FontWeight::Regular => self.regular.clone(),
                FontWeight::Bold => self.bold.clone(),
            },
        };
        let ja = self.regular_ja.clone();
        let zh = self.regular_zh.clone();

        let size = (size * self.resolution.scale()).round().max(1.0);
        let scale = Scale::uniform(size);
        let v_metrics = primary.v_metrics(scale);
        let line_height = (v_metrics.ascent - v_metrics.descent + v_metrics.line_gap).max(size);

        for (line_index, line) in text.lines().enumerate() {
            let origin_y =
                (self.resolution.scale_y(y) + v_metrics.ascent + line_height * line_index as f32)
                    .round();
            let mut caret_x = self.resolution.scale_x(x).round();
            let mut last_glyph_id = None;
            for ch in line.chars() {
                let chosen_font = if primary.glyph(ch).id().0 != 0 {
                    &primary
                } else if ja.glyph(ch).id().0 != 0 {
                    &ja
                } else if zh.glyph(ch).id().0 != 0 {
                    &zh
                } else {
                    &primary
                };
                let glyph = chosen_font.glyph(ch).scaled(scale);
                if let Some(last_id) = last_glyph_id {
                    caret_x += chosen_font.pair_kerning(scale, last_id, glyph.id());
                }
                last_glyph_id = Some(glyph.id());

                let positioned = glyph.positioned(point(caret_x, origin_y));
                caret_x += positioned.unpositioned().h_metrics().advance_width;

                if let Some(bb) = positioned.pixel_bounding_box() {
                    positioned.draw(|gx, gy, alpha| {
                        let a = (alpha * color.a as f32).round() as u8;
                        self.put_pixel(
                            bb.min.x + gx as i32,
                            bb.min.y + gy as i32,
                            Color::rgba(color.r, color.g, color.b, a),
                        );
                    });
                }
            }
        }
    }

    pub fn text_centered(
        &mut self,
        text: &str,
        cx: f32,
        y: f32,
        size: f32,
        color: Color,
        weight: FontWeight,
    ) {
        if text.lines().count() > 1 {
            let line_height = self.text_line_height(size, weight);
            for (line_index, line) in text.lines().enumerate() {
                let x = cx - self.text_width(line, size, weight) / 2.0;
                self.text(
                    line,
                    x,
                    y + line_height * line_index as f32,
                    size,
                    color,
                    weight,
                );
            }
            return;
        }

        let x = cx - self.text_width(text, size, weight) / 2.0;
        self.text(text, x, y, size, color, weight);
    }

    pub fn text_right(
        &mut self,
        text: &str,
        right: f32,
        y: f32,
        size: f32,
        color: Color,
        weight: FontWeight,
    ) {
        let x = right - self.text_width(text, size, weight);
        self.text(text, x, y, size, color, weight);
    }

    pub fn text_width(&self, text: &str, size: f32, weight: FontWeight) -> f32 {
        let primary = match self.language {
            crate::ui::localization::Language::Ja => self.regular_ja.clone(),
            crate::ui::localization::Language::Zh => self.regular_zh.clone(),
            _ => match weight {
                FontWeight::Regular => self.regular.clone(),
                FontWeight::Bold => self.bold.clone(),
            },
        };
        let ja = self.regular_ja.clone();
        let zh = self.regular_zh.clone();
        let scaled_size = (size * self.resolution.scale()).round().max(1.0);
        let scale = Scale::uniform(scaled_size);
        let mut max_width = 0.0_f32;
        for line in text.lines() {
            let mut line_width = 0.0_f32;
            let mut last_glyph_id = None;
            for ch in line.chars() {
                let chosen_font = if primary.glyph(ch).id().0 != 0 {
                    &primary
                } else if ja.glyph(ch).id().0 != 0 {
                    &ja
                } else if zh.glyph(ch).id().0 != 0 {
                    &zh
                } else {
                    &primary
                };
                let glyph = chosen_font.glyph(ch).scaled(scale);
                if let Some(last_id) = last_glyph_id {
                    line_width += chosen_font.pair_kerning(scale, last_id, glyph.id());
                }
                last_glyph_id = Some(glyph.id());
                line_width += glyph.h_metrics().advance_width;
            }
            max_width = max_width.max(line_width / self.resolution.scale());
        }
        max_width
    }

    fn text_line_height(&self, size: f32, weight: FontWeight) -> f32 {
        let font = match self.language {
            crate::ui::localization::Language::Ja => self.regular_ja.clone(),
            crate::ui::localization::Language::Zh => self.regular_zh.clone(),
            _ => match weight {
                FontWeight::Regular => self.regular.clone(),
                FontWeight::Bold => self.bold.clone(),
            },
        };
        let scaled_size = (size * self.resolution.scale()).round().max(1.0);
        let scale = Scale::uniform(scaled_size);
        let v_metrics = font.v_metrics(scale);
        (v_metrics.ascent - v_metrics.descent + v_metrics.line_gap).max(scaled_size)
            / self.resolution.scale()
    }
}
