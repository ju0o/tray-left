/// Colored arc on a clear background. It starts at the top and grows clockwise
/// for the remaining amount. A faint ring stays visible when little is left.
pub fn remaining_icon(size: u32, remaining: u32, color: [u8; 3]) -> Vec<u8> {
    let size = size.max(16);
    let remaining = remaining.min(100);
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    let center = (size as f32 - 1.0) / 2.0;
    let outer = center - 0.5;
    let inner = outer * 0.58;
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
            let index = ((y * size + x) * 4) as usize;
            pixels[index] = scale_channel(color[0], alpha);
            pixels[index + 1] = scale_channel(color[1], alpha);
            pixels[index + 2] = scale_channel(color[2], alpha);
            pixels[index + 3] = alpha;
        }
    }
    pixels
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
        let full = remaining_icon(32, 100, teal);
        let empty = remaining_icon(32, 0, [180, 83, 9]);
        let top = ((2 * 32 + 16) * 4) as usize;
        assert_eq!(&full[top..top + 4], &[15, 118, 110, 255]);
        assert_eq!(empty[top + 3], 64);
        assert!(empty[top] < 180);
        assert_eq!(full.len(), 32 * 32 * 4);
    }
}
