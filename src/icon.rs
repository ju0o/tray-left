/// Colored arc with the remaining number in the middle.
/// The arc starts at the top and grows clockwise. A faint ring stays visible
/// when little is left. Digits are a bold 5 by 7 grid with a dark edge so they
/// survive the tray slot scaling the icon down.
pub fn remaining_icon(size: u32, remaining: u32, color: [u8; 3], show_number: bool) -> Vec<u8> {
    let size = size.max(16);
    let remaining = remaining.min(100);
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    let center = (size as f32 - 1.0) / 2.0;
    let outer = center - 0.15;
    let inner = outer * 0.80;
    let sweep = remaining as f32 / 100.0 * std::f32::consts::TAU;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt();
            let cover = ring_cover(distance, outer, inner);
            if cover <= 0.0 {
                continue;
            }
            let mut angle = dy.atan2(dx) + std::f32::consts::FRAC_PI_2;
            if angle < 0.0 {
                angle += std::f32::consts::TAU;
            }
            let ink = if sweep > 0.0 && angle <= sweep {
                255.0
            } else {
                64.0
            };
            let alpha = (ink * cover).round().clamp(0.0, 255.0) as u8;
            put(&mut pixels, size, x, y, color, alpha);
        }
    }
    if show_number {
        stamp_number(&mut pixels, size, remaining);
    }
    pixels
}

/// Two rings for the open card. The outer ring is usage left.
/// The inner ring is time left until refill, and it empties at the refill moment.
pub fn dial(
    size: u32,
    usage: u32,
    usage_color: [u8; 3],
    charge: Option<f32>,
    charge_color: [u8; 3],
) -> Vec<u8> {
    let size = size.max(32);
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    let center = (size as f32 - 1.0) / 2.0;
    let outer = center - 1.0;
    let inner = outer * 0.78;
    paint_arc(
        &mut pixels,
        size,
        center,
        outer,
        inner,
        usage.min(100) as f32 / 100.0,
        usage_color,
    );
    if let Some(fraction) = charge {
        let gap = (outer - inner) * 0.7;
        let charge_outer = inner - gap;
        let charge_inner = charge_outer * 0.70;
        if charge_inner > 10.0 {
            paint_arc(
                &mut pixels,
                size,
                center,
                charge_outer,
                charge_inner,
                fraction.clamp(0.0, 1.0),
                charge_color,
            );
        }
    }
    pixels
}

fn paint_arc(
    pixels: &mut [u8],
    size: u32,
    center: f32,
    outer: f32,
    inner: f32,
    fraction: f32,
    color: [u8; 3],
) {
    let sweep = fraction.clamp(0.0, 1.0) * std::f32::consts::TAU;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt();
            let cover = ring_cover(distance, outer, inner);
            if cover <= 0.0 {
                continue;
            }
            let mut angle = dy.atan2(dx) + std::f32::consts::FRAC_PI_2;
            if angle < 0.0 {
                angle += std::f32::consts::TAU;
            }
            let ink = if sweep > 0.001 && angle <= sweep {
                255.0
            } else {
                42.0
            };
            let alpha = (ink * cover).round().clamp(0.0, 255.0) as u8;
            put(pixels, size, x, y, color, alpha);
        }
    }
}

fn ring_cover(distance: f32, outer: f32, inner: f32) -> f32 {
    let outer_edge = ((outer + 0.8) - distance).clamp(0.0, 1.6) / 1.6;
    let inner_edge = (distance - (inner - 0.8)).clamp(0.0, 1.6) / 1.6;
    outer_edge.min(inner_edge)
}

fn put(pixels: &mut [u8], size: u32, x: u32, y: u32, color: [u8; 3], alpha: u8) {
    if x >= size || y >= size || alpha == 0 {
        return;
    }
    let index = ((y * size + x) * 4) as usize;
    pixels[index] = scale_channel(color[0], alpha);
    pixels[index + 1] = scale_channel(color[1], alpha);
    pixels[index + 2] = scale_channel(color[2], alpha);
    pixels[index + 3] = alpha;
}

fn stamp_number(pixels: &mut [u8], size: u32, number: u32) {
    let text = number.to_string();
    let digits: Vec<u8> = text
        .bytes()
        .filter(|byte| byte.is_ascii_digit())
        .map(|byte| byte - b'0')
        .collect();
    if digits.is_empty() {
        return;
    }
    let scale = digit_scale(size, digits.len());
    let gap = (scale / 2).max(1);
    let glyph_w = scale * 5;
    let glyph_h = scale * 7;
    let total_w = glyph_w * digits.len() as u32 + gap * (digits.len() as u32 - 1);
    let origin_x = (size.saturating_sub(total_w) / 2) as i32;
    let origin_y = (size.saturating_sub(glyph_h) / 2) as i32;
    let halo = if scale <= 2 { 1 } else { 2 };
    for (index, digit) in digits.iter().enumerate() {
        let x = origin_x + index as i32 * (glyph_w + gap) as i32;
        let y = origin_y;
        for step in 1..=halo {
            stamp_glyph(
                pixels,
                size,
                x + step,
                y + step,
                *digit,
                scale,
                [28, 32, 34],
            );
        }
        for (dx, dy) in [
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 1),
        ] {
            stamp_glyph(pixels, size, x + dx, y + dy, *digit, scale, [28, 32, 34]);
        }
        stamp_glyph(pixels, size, x, y, *digit, scale, [248, 248, 246]);
    }
}

fn digit_scale(size: u32, count: usize) -> u32 {
    let gaps = count.saturating_sub(1) as u32;
    let raw_twice = 5 * count as u32 * 2 + gaps;
    let by_h = (size * 86 / 100) / 7;
    let by_w = (size * 94 / 100 * 2) / raw_twice.max(1);
    by_h.min(by_w).max(1)
}

fn stamp_glyph(
    pixels: &mut [u8],
    size: u32,
    origin_x: i32,
    origin_y: i32,
    digit: u8,
    scale: u32,
    color: [u8; 3],
) {
    let rows = glyph(digit);
    let scale_i = scale as i32;
    for (row, bits) in rows.iter().enumerate() {
        for col in 0..5 {
            if bits & (1 << (4 - col)) == 0 {
                continue;
            }
            for dy in 0..scale_i {
                for dx in 0..scale_i {
                    put_at(
                        pixels,
                        size,
                        origin_x + col * scale_i + dx,
                        origin_y + row as i32 * scale_i + dy,
                        color,
                    );
                }
            }
        }
    }
}

fn put_at(pixels: &mut [u8], size: u32, x: i32, y: i32, color: [u8; 3]) {
    if x < 0 || y < 0 {
        return;
    }
    put(pixels, size, x as u32, y as u32, color, 255);
}

fn glyph(digit: u8) -> [u8; 7] {
    match digit {
        0 => [
            0b01110, 0b11111, 0b11011, 0b11011, 0b11011, 0b11111, 0b01110,
        ],
        1 => [
            0b01110, 0b11110, 0b01110, 0b01110, 0b01110, 0b01110, 0b11111,
        ],
        2 => [
            0b11110, 0b11111, 0b00011, 0b01110, 0b11100, 0b11111, 0b11111,
        ],
        3 => [
            0b11110, 0b11111, 0b00011, 0b01110, 0b00011, 0b11111, 0b11110,
        ],
        4 => [
            0b11011, 0b11011, 0b11111, 0b11111, 0b00011, 0b00011, 0b00011,
        ],
        5 => [
            0b11111, 0b11111, 0b11100, 0b11110, 0b00011, 0b11111, 0b11110,
        ],
        6 => [
            0b01111, 0b11111, 0b11100, 0b11110, 0b11011, 0b11111, 0b01110,
        ],
        7 => [
            0b11111, 0b11111, 0b00011, 0b00110, 0b01100, 0b01100, 0b01100,
        ],
        8 => [
            0b01110, 0b11111, 0b11011, 0b01110, 0b11011, 0b11111, 0b01110,
        ],
        _ => [
            0b01110, 0b11111, 0b11011, 0b01111, 0b00011, 0b11111, 0b11110,
        ],
    }
}

fn scale_channel(color: u8, alpha: u8) -> u8 {
    ((color as u16 * alpha as u16) / 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_ring_is_opaque_and_empty_ring_is_faint() {
        let teal = [15, 118, 110];
        let full = remaining_icon(32, 100, teal, false);
        let empty = remaining_icon(32, 0, [180, 83, 9], false);
        let top = ((2 * 32 + 16) * 4) as usize;
        assert_eq!(&full[top..top + 4], &[15, 118, 110, 255]);
        assert_eq!(empty[top + 3], 64);
        assert!(empty[top] < 180);
        assert_eq!(full.len(), 32 * 32 * 4);
    }

    #[test]
    fn the_number_sits_in_the_middle() {
        let icon = remaining_icon(40, 48, [15, 118, 110], true);
        let mut bright = 0u32;
        let center = 20i32;
        for y in 12..28 {
            for x in 10..30 {
                let index = ((y * 40 + x) * 4) as usize;
                if icon[index + 3] == 255 && icon[index] > 200 {
                    bright += 1;
                    let _ = (x as i32 - center, y as i32 - center);
                }
            }
        }
        assert!(
            bright > 40,
            "bold digits should fill the center, counted {bright}"
        );
    }

    #[test]
    fn the_inner_ring_is_the_one_that_empties() {
        let full = dial(80, 100, [15, 118, 110], Some(0.0), [90, 98, 106]);
        let outer = pixel(&full, 80, 40, 8);
        let inner = pixel(&full, 80, 40, 22);
        assert!(
            outer[3] > 200,
            "outer usage ring stays bright, alpha {}",
            outer[3]
        );
        assert!(
            inner[3] < 90,
            "an empty charge ring stays faint, alpha {}",
            inner[3]
        );
        let charged = dial(80, 100, [15, 118, 110], Some(1.0), [90, 98, 106]);
        assert!(pixel(&charged, 80, 40, 22)[3] > 200);
    }

    fn pixel(icon: &[u8], size: u32, x: u32, y: u32) -> [u8; 4] {
        let index = ((y * size + x) * 4) as usize;
        [
            icon[index],
            icon[index + 1],
            icon[index + 2],
            icon[index + 3],
        ]
    }
}
