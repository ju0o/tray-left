/// Colored arc with the remaining number in the middle.
/// The arc starts at the top and grows clockwise. A faint ring stays visible
/// when little is left. Digits are drawn on a 2px grid so a tray scale-down stays sharp.
pub fn remaining_icon(size: u32, remaining: u32, color: [u8; 3], show_number: bool) -> Vec<u8> {
    let size = size.max(16);
    let remaining = remaining.min(100);
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    let center = (size as f32 - 1.0) / 2.0;
    let outer = center - 0.15;
    let inner = outer * 0.70;
    let sweep = remaining as f32 / 100.0 * std::f32::consts::TAU;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > outer || distance < inner {
                continue;
            }
            let mut angle = dy.atan2(dx) + std::f32::consts::FRAC_PI_2;
            if angle < 0.0 {
                angle += std::f32::consts::TAU;
            }
            let alpha = if sweep > 0.0 && angle <= sweep { 255 } else { 64 };
            put(&mut pixels, size, x, y, color, alpha);
        }
    }
    if show_number {
        stamp_number(&mut pixels, size, remaining);
    }
    pixels
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
    let digits: Vec<u8> = text.bytes().filter(|byte| byte.is_ascii_digit()).map(|byte| byte - b'0').collect();
    if digits.is_empty() {
        return;
    }
    let scale = digit_scale(size, digits.len());
    let gap = scale;
    let glyph_w = scale * 3;
    let glyph_h = scale * 5;
    let total_w = glyph_w * digits.len() as u32 + gap * (digits.len() as u32 - 1);
    let origin_x = (size - total_w) / 2;
    let origin_y = (size - glyph_h) / 2;
    for (index, digit) in digits.iter().enumerate() {
        let x = origin_x + index as u32 * (glyph_w + gap);
        stamp_glyph(pixels, size, x, origin_y, *digit, scale, [28, 32, 34], 1, 1);
        stamp_glyph(pixels, size, x, origin_y, *digit, scale, [248, 248, 246], 0, 0);
    }
}

fn digit_scale(size: u32, count: usize) -> u32 {
    let raw_w = 3 * count as u32 + (count as u32 - 1);
    let by_h = (size * 50 / 100) / 5;
    let by_w = (size * 64 / 100) / raw_w.max(1);
    by_h.min(by_w).max(1)
}

fn stamp_glyph(pixels: &mut [u8], size: u32, origin_x: u32, origin_y: u32, digit: u8, scale: u32, color: [u8; 3], shift_x: u32, shift_y: u32) {
    let rows = glyph(digit);
    for (row, bits) in rows.iter().enumerate() {
        for col in 0..3 {
            if bits & (1 << (2 - col)) == 0 {
                continue;
            }
            for dy in 0..scale {
                for dx in 0..scale {
                    put(
                        pixels,
                        size,
                        origin_x + shift_x + col * scale + dx,
                        origin_y + shift_y + row as u32 * scale + dy,
                        color,
                        255,
                    );
                }
            }
        }
    }
}

fn glyph(digit: u8) -> [u8; 5] {
    match digit {
        0 => [0b111, 0b101, 0b101, 0b101, 0b111],
        1 => [0b010, 0b110, 0b010, 0b010, 0b111],
        2 => [0b111, 0b001, 0b111, 0b100, 0b111],
        3 => [0b111, 0b001, 0b111, 0b001, 0b111],
        4 => [0b101, 0b101, 0b111, 0b001, 0b001],
        5 => [0b111, 0b100, 0b111, 0b001, 0b111],
        6 => [0b111, 0b100, 0b111, 0b101, 0b111],
        7 => [0b111, 0b001, 0b001, 0b001, 0b001],
        8 => [0b111, 0b101, 0b111, 0b101, 0b111],
        _ => [0b111, 0b101, 0b111, 0b001, 0b111],
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
        assert!(bright > 12, "digits should be visible in the center");
    }
}
