/// Midnight Slate tokens. Body text uses ink and muted. Edge is a border, not small text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn colorref(self) -> u32 {
        (self.b as u32) << 16 | (self.g as u32) << 8 | self.r as u32
    }

    pub const fn rgb(self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }
}

pub const DEPTH: Rgb = Rgb::new(0x11, 0x15, 0x22);
pub const PLATE: Rgb = Rgb::new(0x1E, 0x26, 0x3B);
pub const CARD: Rgb = Rgb::new(0x30, 0x3D, 0x59);
pub const EDGE: Rgb = Rgb::new(0x53, 0x66, 0x84);
pub const MUTED: Rgb = Rgb::new(0x98, 0xA9, 0xC4);
pub const INK: Rgb = Rgb::new(0xDC, 0xE3, 0xEF);

/// Dark charge fill is the secondary text color, so the bar stays visible on the deep track.
pub const DARK_CHARGE: Rgb = MUTED;
/// Light charge fill stays on the border color. It is not a status color.
pub const LIGHT_CHARGE: Rgb = EDGE;

pub const LIGHT_DEPTH: Rgb = Rgb::new(0xC5, 0xD0, 0xE2);
pub const LIGHT_PLATE: Rgb = Rgb::new(0xF4, 0xF7, 0xFB);
pub const LIGHT_INK: Rgb = DEPTH;
pub const LIGHT_MUTED: Rgb = Rgb::new(0x3E, 0x4D, 0x68);

pub fn contrast(text: Rgb, background: Rgb) -> f32 {
    let text = luminance(text);
    let background = luminance(background);
    let (hi, lo) = if text > background {
        (text, background)
    } else {
        (background, text)
    };
    (hi + 0.05) / (lo + 0.05)
}

fn luminance(color: Rgb) -> f32 {
    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}

fn channel(value: u8) -> f32 {
    let unit = value as f32 / 255.0;
    if unit <= 0.04045 {
        unit / 12.92
    } else {
        ((unit + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midnight_tokens_are_the_six_colors() {
        assert_eq!((DEPTH.r, DEPTH.g, DEPTH.b), (0x11, 0x15, 0x22));
        assert_eq!((PLATE.r, PLATE.g, PLATE.b), (0x1E, 0x26, 0x3B));
        assert_eq!((CARD.r, CARD.g, CARD.b), (0x30, 0x3D, 0x59));
        assert_eq!((EDGE.r, EDGE.g, EDGE.b), (0x53, 0x66, 0x84));
        assert_eq!((MUTED.r, MUTED.g, MUTED.b), (0x98, 0xA9, 0xC4));
        assert_eq!((INK.r, INK.g, INK.b), (0xDC, 0xE3, 0xEF));
        assert_eq!(DARK_CHARGE, MUTED);
        assert_eq!(LIGHT_CHARGE, EDGE);
        assert_eq!(INK.colorref(), 0x00EF_E3DC);
        assert_eq!(LIGHT_INK.colorref(), 0x0022_1511);
        assert_eq!(LIGHT_DEPTH.rgb(), [0xC5, 0xD0, 0xE2]);
        assert!(contrast(DARK_CHARGE, DEPTH) > contrast(EDGE, DEPTH));
        assert!(contrast(DARK_CHARGE, DEPTH) >= 4.5);
    }

    #[test]
    fn dark_text_clears_aa_and_the_border_is_not_body_text() {
        assert!(contrast(INK, PLATE) >= 7.0);
        assert!(contrast(INK, CARD) >= 7.0);
        assert!(contrast(INK, DEPTH) >= 7.0);
        assert!(contrast(MUTED, PLATE) >= 4.5);
        assert!(contrast(MUTED, CARD) >= 4.5);
        assert!(contrast(EDGE, PLATE) < 3.0);
        assert!(contrast(EDGE, CARD) < 3.0);
    }

    #[test]
    fn light_text_uses_a_darker_slate_than_the_dark_theme_muted() {
        assert_ne!(LIGHT_MUTED, MUTED);
        assert!(contrast(LIGHT_INK, LIGHT_PLATE) >= 7.0);
        assert!(contrast(LIGHT_MUTED, LIGHT_PLATE) >= 7.0);
        assert!(contrast(MUTED, LIGHT_PLATE) < 3.0);
    }
}
