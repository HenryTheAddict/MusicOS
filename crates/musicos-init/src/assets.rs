use anyhow::{Context, Result};
use rusttype::Font;

pub static RASTER_GOTHIC_28_COND: &[u8] =
    include_bytes!("../../../assets/fonts/RasterGothic28Cond.ttf");
pub static RASTER_GOTHIC_28_COND_BOLD: &[u8] =
    include_bytes!("../../../assets/fonts/RasterGothic28CondBold.ttf");

pub static NOTO_SANS_JP_REGULAR: &[u8] =
    include_bytes!("../../../assets/fonts/NotoSansJP-Regular.ttf");
pub static NOTO_SANS_SC_REGULAR: &[u8] =
    include_bytes!("../../../assets/fonts/NotoSansSC-Regular.ttf");

pub fn regular_font() -> Result<Font<'static>> {
    Font::try_from_bytes(RASTER_GOTHIC_28_COND).context("failed to load RasterGothic28Cond.ttf")
}

pub fn bold_font() -> Result<Font<'static>> {
    Font::try_from_bytes(RASTER_GOTHIC_28_COND_BOLD)
        .context("failed to load RasterGothic28CondBold.ttf")
}

pub fn regular_font_ja() -> Result<Font<'static>> {
    Font::try_from_bytes(NOTO_SANS_JP_REGULAR).context("failed to load NotoSansJP-Regular.ttf")
}

pub fn regular_font_zh() -> Result<Font<'static>> {
    Font::try_from_bytes(NOTO_SANS_SC_REGULAR).context("failed to load NotoSansSC-Regular.ttf")
}
