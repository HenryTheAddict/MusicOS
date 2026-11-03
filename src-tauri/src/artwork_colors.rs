//! Actual artwork swatches, separate from contrast-adjusted interface surfaces.
//! Bounded, deterministic perceptual clustering; cached by the artwork provider.
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct Sample {
    rgb: [f64; 3],
    lab: [f64; 3],
    weight: f64,
}
fn lab(rgb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rgb.map(|v| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    });
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
    ]
}
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum()
}
fn chroma(s: &Sample) -> f64 {
    s.lab[1].hypot(s.lab[2])
}

pub fn swatches(image: &image::RgbaImage) -> Vec<String> {
    let mut bins = BTreeMap::<u16, (f64, [f64; 3])>::new();
    for pixel in image.pixels() {
        if pixel[3] < 32 {
            continue;
        }
        let weight = f64::from(pixel[3]) / 255.;
        let key = ((u16::from(pixel[0]) / 16) << 8)
            | ((u16::from(pixel[1]) / 16) << 4)
            | (u16::from(pixel[2]) / 16);
        let bin = bins.entry(key).or_default();
        bin.0 += weight;
        for i in 0..3 {
            bin.1[i] += f64::from(pixel[i]) / 255. * weight;
        }
    }
    let samples: Vec<_> = bins
        .values()
        .map(|(weight, sum)| {
            let rgb = sum.map(|v| v / weight);
            Sample {
                rgb,
                lab: lab(rgb),
                weight: *weight,
            }
        })
        .collect();
    if samples.is_empty() {
        return vec![];
    }
    let total = samples.iter().map(|s| s.weight).sum::<f64>();
    let mut centers = vec![*samples
        .iter()
        .max_by(|a, b| a.weight.total_cmp(&b.weight))
        .unwrap()];
    for _ in 1..8 {
        let next = samples
            .iter()
            .max_by(|a, b| {
                let score = |s: &Sample| {
                    s.weight
                        * centers
                            .iter()
                            .map(|c| distance(s.lab, c.lab))
                            .fold(f64::INFINITY, f64::min)
                };
                score(a).total_cmp(&score(b))
            })
            .unwrap();
        if centers.iter().any(|c| distance(c.lab, next.lab) < 0.00001) {
            break;
        }
        centers.push(*next);
    }
    for _ in 0..8 {
        let mut groups = vec![(0_f64, [0_f64; 3]); centers.len()];
        for sample in &samples {
            let index = centers
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    distance(sample.lab, a.lab).total_cmp(&distance(sample.lab, b.lab))
                })
                .unwrap()
                .0;
            groups[index].0 += sample.weight;
            for i in 0..3 {
                groups[index].1[i] += sample.rgb[i] * sample.weight;
            }
        }
        centers = groups
            .into_iter()
            .filter(|(weight, _)| *weight > 0.)
            .map(|(weight, sum)| {
                let rgb = sum.map(|v| v / weight);
                Sample {
                    rgb,
                    lab: lab(rgb),
                    weight,
                }
            })
            .collect();
    }
    // Compression specks and tiny printed details must not dominate the cover.
    centers.retain(|s| s.weight >= total * 0.008);
    let colorful: Vec<_> = centers
        .iter()
        .copied()
        .filter(|s| chroma(s) >= 0.045 && s.weight >= total * 0.015)
        .collect();
    let candidates = if colorful.iter().map(|s| s.weight).sum::<f64>() >= total * 0.06 {
        colorful
    } else {
        centers
    };
    if candidates.is_empty() {
        return vec![];
    }
    let mut chosen = vec![*candidates
        .iter()
        .max_by(|a, b| {
            let score = |s: &Sample| s.weight * (1. + chroma(s).min(0.2) * 4.);
            score(a).total_cmp(&score(b))
        })
        .unwrap()];
    while chosen.len() < 3 {
        let next = candidates
            .iter()
            .max_by(|a, b| {
                let score = |s: &Sample| {
                    s.weight.sqrt()
                        * chosen
                            .iter()
                            .map(|c| distance(s.lab, c.lab))
                            .fold(f64::INFINITY, f64::min)
                };
                score(a).total_cmp(&score(b))
            })
            .unwrap();
        if chosen.iter().any(|c| distance(c.lab, next.lab) < 0.001) {
            break;
        }
        chosen.push(*next);
    }
    while chosen.len() < 3 {
        chosen.push(chosen[0]);
    }
    chosen.sort_by(|a, b| a.lab[0].total_cmp(&b.lab[0]));
    chosen
        .iter()
        .map(|s| {
            let [r, g, b] = s.rgb.map(|v| (v * 255.).round().clamp(0., 255.) as u8);
            format!("#{r:02x}{g:02x}{b:02x}")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rgb(color: &str) -> [u8; 3] {
        [1, 3, 5].map(|i| u8::from_str_radix(&color[i..i + 2], 16).unwrap())
    }
    #[test]
    fn retains_distinct_art_colors_instead_of_theme_whitening() {
        let mut art = image::RgbaImage::from_pixel(128, 128, image::Rgba([245, 245, 245, 255]));
        for y in 10..118 {
            for x in 10..118 {
                art.put_pixel(
                    x,
                    y,
                    image::Rgba(if x < 64 {
                        [12, 48, 190, 255]
                    } else {
                        [235, 160, 15, 255]
                    }),
                );
            }
        }
        let colors = swatches(&art);
        assert!(colors.iter().map(|s| rgb(s)).any(|[r, _, b]| b > r + 80));
        assert!(colors
            .iter()
            .map(|s| rgb(s))
            .any(|[r, g, b]| r > 180 && g > 100 && b < 50));
        assert_eq!(colors, swatches(&art));
    }
    #[test]
    fn ignores_transparency_and_tiny_vivid_specks() {
        let mut art = image::RgbaImage::from_pixel(128, 128, image::Rgba([35, 125, 62, 255]));
        art.put_pixel(1, 1, image::Rgba([255, 0, 255, 255]));
        for x in 0..128 {
            art.put_pixel(x, 0, image::Rgba([255, 0, 0, 0]));
        }
        assert!(swatches(&art)
            .iter()
            .map(|s| rgb(s))
            .all(|[r, g, b]| g > r * 2 && g > b));
    }
    #[test]
    fn monochrome_and_empty_art_do_not_invent_hues() {
        let gray = image::RgbaImage::from_fn(128, 128, |x, _| {
            image::Rgba([x as u8, x as u8, x as u8, 255])
        });
        assert!(swatches(&gray)
            .iter()
            .map(|s| rgb(s))
            .all(|[r, g, b]| r == g && g == b));
        assert!(swatches(&image::RgbaImage::new(32, 32)).is_empty());
    }
}
