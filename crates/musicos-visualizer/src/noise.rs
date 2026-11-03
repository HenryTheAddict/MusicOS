//! Original Butterchurn two-dimensional noise construction (MIT, Jordan Berg).
//! This includes periodic cubic interpolation and unsigned-byte wrapping.
pub fn texture(size: usize, zoom: usize, mut random: impl FnMut() -> f64) -> Vec<u8> {
    let range = if zoom > 1 { 216. } else { 256. };
    let mut out = (0..size * size * 4)
        .map(|_| ((random() * range + range * 0.5).floor() as u32 % 256) as u8)
        .collect::<Vec<_>>();
    if zoom == 1 {
        return out;
    }
    let cubic = |v: [u8; 4], t: f64| {
        let [y0, y1, y2, y3] = v.map(|x| x as f64 / 255.);
        let a = y3 - y2 - y0 + y1;
        let b = y0 - y1 - a;
        ((a * t * t * t + b * t * t + (y2 - y0) * t + y1).clamp(0., 1.) * 255.) as u8
    };
    for y in (0..size).step_by(zoom) {
        for x in 0..size {
            if x % zoom != 0 {
                let base = x / zoom * zoom + size;
                for c in 0..4 {
                    let points = [base - zoom, base, base + zoom, base + 2 * zoom]
                        .map(|xx| out[(y * size + xx % size) * 4 + c]);
                    out[(y * size + x) * 4 + c] = cubic(points, (x % zoom) as f64 / zoom as f64);
                }
            }
        }
    }
    for x in 0..size {
        for y in 0..size {
            if y % zoom != 0 {
                let base = y / zoom * zoom + size;
                for c in 0..4 {
                    let points = [base - zoom, base, base + zoom, base + 2 * zoom]
                        .map(|yy| out[((yy % size) * size + x) * 4 + c]);
                    out[(y * size + x) * 4 + c] = cubic(points, (y % zoom) as f64 / zoom as f64);
                }
            }
        }
    }
    out
}
#[cfg(test)]
mod tests {
    #[test]
    fn bounded_periodic_noise_retains_channels() {
        let a = super::texture(32, 8, || 0.5);
        assert_eq!(a.len(), 4096);
        assert!(a.iter().all(|v| *v == 216));
        let b = super::texture(32, 1, || 0.75);
        assert!(b.iter().all(|v| *v == 64));
    }
}
