use anyhow::{Context, Result};
use rusttype::Font;
use std::sync::OnceLock;

pub static RASTER_GOTHIC_28_COND: &[u8] =
    include_bytes!("../../../assets/fonts/RasterGothic28Cond.ttf");
pub static RASTER_GOTHIC_28_COND_BOLD: &[u8] =
    include_bytes!("../../../assets/fonts/RasterGothic28CondBold.ttf");

pub static NOTO_SANS_JP_REGULAR: &[u8] =
    include_bytes!("../../../assets/fonts/NotoSansJP-Regular.ttf");
pub static NOTO_SANS_SC_REGULAR: &[u8] =
    include_bytes!("../../../assets/fonts/NotoSansSC-Regular.ttf");

static REGULAR_FONT: OnceLock<Font<'static>> = OnceLock::new();
static BOLD_FONT: OnceLock<Font<'static>> = OnceLock::new();
static REGULAR_FONT_JA: OnceLock<Font<'static>> = OnceLock::new();
static REGULAR_FONT_ZH: OnceLock<Font<'static>> = OnceLock::new();

pub fn regular_font() -> Result<Font<'static>> {
    Ok(REGULAR_FONT
        .get_or_init(|| {
            Font::try_from_bytes(RASTER_GOTHIC_28_COND)
                .context("failed to load RasterGothic28Cond.ttf")
                .expect("embedded regular font must be valid")
        })
        .clone())
}

pub fn bold_font() -> Result<Font<'static>> {
    Ok(BOLD_FONT
        .get_or_init(|| {
            Font::try_from_bytes(RASTER_GOTHIC_28_COND_BOLD)
                .context("failed to load RasterGothic28CondBold.ttf")
                .expect("embedded bold font must be valid")
        })
        .clone())
}

pub fn regular_font_ja() -> Result<Font<'static>> {
    Ok(REGULAR_FONT_JA
        .get_or_init(|| {
            Font::try_from_bytes(NOTO_SANS_JP_REGULAR)
                .context("failed to load NotoSansJP-Regular.ttf")
                .expect("embedded Japanese font must be valid")
        })
        .clone())
}

pub fn regular_font_zh() -> Result<Font<'static>> {
    Ok(REGULAR_FONT_ZH
        .get_or_init(|| {
            Font::try_from_bytes(NOTO_SANS_SC_REGULAR)
                .context("failed to load NotoSansSC-Regular.ttf")
                .expect("embedded Chinese font must be valid")
        })
        .clone())
}
