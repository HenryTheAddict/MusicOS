#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

#[cfg(target_os = "linux")]
use std::fs::OpenOptions;
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
use std::path::Path;

use anyhow::Result;
#[cfg(target_os = "linux")]
use anyhow::{anyhow, Context};

use crate::renderer::{Canvas, Resolution};

#[cfg(target_os = "linux")]
const FBIOGET_VSCREENINFO: libc::c_ulong = 0x4600;
#[cfg(target_os = "linux")]
const FBIOGET_FSCREENINFO: libc::c_ulong = 0x4602;

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
struct FbBitfield {
    offset: u32,
    length: u32,
    msb_right: u32,
}

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
struct FbVarScreeninfo {
    xres: u32,
    yres: u32,
    xres_virtual: u32,
    yres_virtual: u32,
    xoffset: u32,
    yoffset: u32,
    bits_per_pixel: u32,
    grayscale: u32,
    red: FbBitfield,
    green: FbBitfield,
    blue: FbBitfield,
    transp: FbBitfield,
    nonstd: u32,
    activate: u32,
    height: u32,
    width: u32,
    accel_flags: u32,
    pixclock: u32,
    left_margin: u32,
    right_margin: u32,
    upper_margin: u32,
    lower_margin: u32,
    hsync_len: u32,
    vsync_len: u32,
    sync: u32,
    vmode: u32,
    rotate: u32,
    colorspace: u32,
    reserved: [u32; 4],
}

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct FbFixScreeninfo {
    id: [libc::c_char; 16],
    smem_start: libc::c_ulong,
    smem_len: u32,
    type_: u32,
    type_aux: u32,
    visual: u32,
    xpanstep: u16,
    ypanstep: u16,
    ywrapstep: u16,
    line_length: u32,
    mmio_start: libc::c_ulong,
    mmio_len: u32,
    accel: u32,
    capabilities: u16,
    reserved: [u16; 2],
}

#[cfg(target_os = "linux")]
impl Default for FbFixScreeninfo {
    fn default() -> Self {
        // SAFETY: Linux framebuffer structs are plain-old-data C structs.
        unsafe { std::mem::zeroed() }
    }
}

#[cfg(target_os = "linux")]
pub struct FbDev {
    file: std::fs::File,
    ptr: *mut libc::c_void,
    len: usize,
    line_length: usize,
    resolution: Resolution,
    back_buffer: Vec<u8>,
}

#[cfg(not(target_os = "linux"))]
pub struct FbDev;

#[cfg(target_os = "linux")]
impl FbDev {
    pub fn open(path: impl AsRef<Path>, expected: Resolution) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.as_ref())
            .with_context(|| format!("open {}", path.as_ref().display()))?;
        let fd = file.as_raw_fd();
        let mut var = FbVarScreeninfo::default();
        let mut fix = FbFixScreeninfo::default();

        // SAFETY: ioctl writes into valid framebuffer info structs for an open fb fd.
        let var_rc = unsafe { libc::ioctl(fd, FBIOGET_VSCREENINFO as _, &mut var) };
        if var_rc != 0 {
            return Err(anyhow!("FBIOGET_VSCREENINFO failed"));
        }
        // SAFETY: ioctl writes into valid framebuffer info structs for an open fb fd.
        let fix_rc = unsafe { libc::ioctl(fd, FBIOGET_FSCREENINFO as _, &mut fix) };
        if fix_rc != 0 {
            return Err(anyhow!("FBIOGET_FSCREENINFO failed"));
        }

        if var.bits_per_pixel != 32 {
            return Err(anyhow!(
                "unsupported framebuffer bpp {}",
                var.bits_per_pixel
            ));
        }
        if var.xres < expected.width || var.yres < expected.height {
            return Err(anyhow!(
                "framebuffer {}x{} smaller than requested {}x{}",
                var.xres,
                var.yres,
                expected.width,
                expected.height
            ));
        }

        let len = fix.smem_len as usize;
        let line_length = fix.line_length as usize;
        let visible_stride = expected.width as usize * 4;
        let visible_len = visible_stride * expected.height as usize;
        if line_length < visible_stride {
            return Err(anyhow!(
                "framebuffer stride {} smaller than requested stride {}",
                line_length,
                visible_stride
            ));
        }
        if len < line_length * expected.height as usize {
            return Err(anyhow!(
                "framebuffer memory {} too small for {} rows at stride {}",
                len,
                expected.height,
                line_length
            ));
        }

        // SAFETY: mmap maps the framebuffer fd for read/write shared access.
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            return Err(anyhow!("mmap framebuffer failed"));
        }

        Ok(Self {
            file,
            ptr,
            len,
            line_length,
            resolution: expected,
            back_buffer: vec![0; visible_len],
        })
    }

    pub fn draw(&mut self, f: impl FnOnce(&mut Canvas<'_>)) -> Result<()> {
        let _keep_fd_alive = self.file.as_raw_fd();
        let mut canvas = Canvas::new(self.resolution, &mut self.back_buffer)?;
        f(&mut canvas);
        self.present();
        Ok(())
    }

    fn present(&mut self) {
        let visible_stride = self.resolution.width as usize * 4;
        // SAFETY: ptr/len are a live mmap for the lifetime of self.
        let fb = unsafe { std::slice::from_raw_parts_mut(self.ptr as *mut u8, self.len) };
        for y in 0..self.resolution.height as usize {
            let src_start = y * visible_stride;
            let dst_start = y * self.line_length;
            fb[dst_start..dst_start + visible_stride]
                .copy_from_slice(&self.back_buffer[src_start..src_start + visible_stride]);
        }
    }
}

#[cfg(not(target_os = "linux"))]
impl FbDev {
    pub fn open(path: impl AsRef<Path>, _expected: Resolution) -> Result<Self> {
        anyhow::bail!(
            "framebuffer devices are only supported on Linux: {}",
            path.as_ref().display()
        )
    }

    pub fn draw(&mut self, _f: impl FnOnce(&mut Canvas<'_>)) -> Result<()> {
        anyhow::bail!("framebuffer drawing is only supported on Linux")
    }
}

#[cfg(target_os = "linux")]
impl Drop for FbDev {
    fn drop(&mut self) {
        // SAFETY: ptr/len were returned by mmap and are unmapped once here.
        unsafe {
            libc::munmap(self.ptr, self.len);
        }
    }
}
