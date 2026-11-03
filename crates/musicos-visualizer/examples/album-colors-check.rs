//! Read back real GPU output; never opens the app, user library, or audio.
use musicos_visualizer::{
    gpu::Gpu,
    renderer::{Image, Presenter},
};
fn read(gpu: &Gpu, image: &Image) -> anyhow::Result<Vec<u8>> {
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Album colors readback"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        image.texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        image.texture.size(),
    );
    gpu.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    gpu.device.poll(wgpu::PollType::Wait)?;
    rx.recv()??;
    let bytes = buffer.slice(..).get_mapped_range()[..16].to_vec();
    Ok(bytes)
}
fn main() -> anyhow::Result<()> {
    let gpu = pollster::block_on(Gpu::new(None, Gpu::instance()))?;
    let source = Image::new(&gpu.device, 4, 1);
    let target = Image::new(&gpu.device, 4, 1);
    let pixels = [
        0, 0, 0, 255, 50, 120, 180, 255, 130, 130, 130, 255, 255, 255, 255, 255,
    ];
    gpu.queue.write_texture(
        source.texture.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(16),
            rows_per_image: Some(1),
        },
        source.texture.size(),
    );
    let present = Presenter::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm);
    present.draw(&gpu, &target.view, &source, &source, 0., 1.);
    let original = read(&gpu, &target)?;
    assert_eq!(original, pixels);
    present.draw_colored(
        &gpu,
        &target.view,
        &source,
        &source,
        0.,
        1.,
        Some([[0.4, 0.02, 0.02], [0.9, 0.1, 0.1], [1., 0.5, 0.5]]),
    );
    let red = read(&gpu, &target)?;
    assert_eq!(&red[..4], &pixels[..4]);
    assert_eq!(&red[12..], &pixels[12..]);
    assert!(red[4] > red[5] * 2);
    assert!(red[8] > red[9] * 2);
    present.draw_colored(
        &gpu,
        &target.view,
        &source,
        &source,
        0.,
        1.,
        Some([[f32::NAN, 0., 0.]; 3]),
    );
    assert_eq!(read(&gpu, &target)?, original);
    present.draw_colored(&gpu, &target.view, &source, &source, 0., 1., None);
    assert_eq!(read(&gpu, &target)?, original);
    // Equal-luminance red and green must retain distinct regions after tinting.
    // A luminance-only color map made both pixels the same purple.
    let regions = [
        128, 0, 0, 255, 0, 38, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255,
    ];
    gpu.queue.write_texture(
        source.texture.as_image_copy(),
        &regions,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(16),
            rows_per_image: Some(1),
        },
        source.texture.size(),
    );
    present.draw_colored(
        &gpu,
        &target.view,
        &source,
        &source,
        0.,
        1.,
        Some([[0.1, 0.2, 0.9], [0.9, 0.1, 0.2], [0.1, 0.8, 0.3]]),
    );
    let tinted = read(&gpu, &target)?;
    assert!(tinted[2] > tinted[0]);
    assert!(tinted[4] > tinted[6]);
    let light = |rgb: &[u8]| {
        f64::from(rgb[0]) * 0.2126 + f64::from(rgb[1]) * 0.7152 + f64::from(rgb[2]) * 0.0722
    };
    assert!((light(&tinted[..3]) - light(&regions[..3])).abs() < 1.);
    assert!((light(&tinted[4..7]) - light(&regions[4..7])).abs() < 1.);
    println!("GPU album colors: original exact, hue regions distinct, luminance and black/white preserved, invalid palette safe, off restores exact original");
    Ok(())
}
