//! An account's badge: the letter and colour shown on its row, in its taskbar
//! button and on its icon. All pure; the OS adapter only paints what this says.

/// The colours handed out to new accounts, in order. Each is dark enough to
/// carry a white letter.
pub const PALETTE: [&str; 10] = [
    "#2563eb", "#dc2626", "#16a34a", "#9333ea", "#ea580c", "#0891b2", "#db2777", "#a16207",
    "#4f46e5", "#0d9488",
];

/// The orange of Claude's icon: the badge's base when Claude's own icon is not
/// there to take.
pub const CLAUDE_ORANGE: [u8; 3] = [0xd9, 0x77, 0x57];

/// The side, in pixels, of the base art the icons are cut from.
pub const BASE_SIDE: usize = 256;

/// The letter's disc as fractions of the icon's side: centre x, centre y, radius.
/// It sits on the bottom-right corner, as on Claude's own multi-account icons.
pub const DISC: [f32; 3] = [0.70, 0.70, 0.25];

/// The width of the white ring round the disc, as a fraction of the side.
pub const RING: f32 = 0.045;

/// The first colour of the palette no account in `used` has. Once every colour
/// is taken, the least used one (earliest first).
pub fn next_colour<'a>(used: impl IntoIterator<Item = &'a str>) -> &'static str {
    let used: Vec<&str> = used.into_iter().collect();
    let count = |colour: &str| {
        used.iter()
            .filter(|u| u.eq_ignore_ascii_case(colour))
            .count()
    };
    PALETTE
        .iter()
        .min_by_key(|colour| count(colour))
        .copied()
        .expect("the palette is not empty")
}

/// `#rrggbb`, any case.
pub fn is_valid_colour(colour: &str) -> bool {
    colour.len() == 7
        && colour.starts_with('#')
        && colour[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// The first letter (or digit) of the label, in capitals; `?` when it has none.
pub fn badge_letter(label: &str) -> char {
    label
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().next().unwrap_or(c))
        .unwrap_or('?')
}

/// The window's AppUserModelID. Windows groups taskbar buttons by it, so the
/// badge letter and colour are part of it: change either and the window
/// becomes a new group with the new icon. Ascii only, well under the 128
/// character limit.
pub fn app_user_model_id(account_id: &str, letter: char, colour: &str) -> String {
    format!(
        "clawshell.{account_id}.{:x}.{}",
        u32::from(letter),
        colour.trim_start_matches('#').to_ascii_lowercase()
    )
}

/// The red, green and blue of a `#rrggbb` colour.
pub fn rgb(colour: &str) -> Option<[u8; 3]> {
    if !is_valid_colour(colour) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&colour[i..i + 2], 16).ok();
    Some([byte(1)?, byte(3)?, byte(5)?])
}

/// Signed distance to a rounded rectangle centred on `centre` with half sides
/// `half`; negative inside.
fn rounded_distance(at: (f32, f32), centre: (f32, f32), half: (f32, f32), radius: f32) -> f32 {
    let dx = (at.0 - centre.0).abs() - (half.0 - radius);
    let dy = (at.1 - centre.1).abs() - (half.1 - radius);
    dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - radius
}

/// How much of a pixel a shape with signed `distance` covers, 0 to 1.
fn cover(distance: f32) -> f32 {
    (0.5 - distance).clamp(0.0, 1.0)
}

/// A rounded square of `rgb` with see-through corners, as straight-alpha BGRA,
/// `size` x `size` pixels, top row first.
pub fn rounded_square(size: usize, rgb: [u8; 3]) -> Vec<u8> {
    let half = size as f32 / 2.0;
    let radius = size as f32 * 0.22;
    let mut pixels = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let at = (x as f32 + 0.5, y as f32 + 0.5);
            let alpha = cover(rounded_distance(at, (half, half), (half, half), radius));
            pixels.extend_from_slice(&[rgb[2], rgb[1], rgb[0], (alpha * 255.0).round() as u8]);
        }
    }
    pixels
}

/// The base of the badge when Claude's own icon is not there: the orange
/// rounded square, `BASE_SIDE` pixels across.
pub fn plain_base() -> Vec<u8> {
    rounded_square(BASE_SIDE, CLAUDE_ORANGE)
}

/// `src`, a square of `from` x `from` straight-alpha BGRA pixels, averaged down
/// or up to `to` x `to`. Colours are weighted by alpha, so the transparent
/// pixels around an icon do not bleed their colour into its edge.
pub fn resample(src: &[u8], from: usize, to: usize) -> Vec<u8> {
    assert_eq!(src.len(), from * from * 4, "a square of pixels");
    let span = |i: usize| {
        let start = i * from / to;
        let end = ((i + 1) * from / to).max(start + 1).min(from);
        start..end
    };
    let mut pixels = Vec::with_capacity(to * to * 4);
    for y in 0..to {
        for x in 0..to {
            let mut weighted = [0u32; 3];
            let mut alpha = 0u32;
            let mut count = 0u32;
            for sy in span(y) {
                for sx in span(x) {
                    let p = &src[(sy * from + sx) * 4..][..4];
                    let a = u32::from(p[3]);
                    for c in 0..3 {
                        weighted[c] += u32::from(p[c]) * a;
                    }
                    alpha += a;
                    count += 1;
                }
            }
            for c in weighted {
                pixels.push(if alpha == 0 { 0 } else { (c / alpha) as u8 });
            }
            pixels.push((alpha / count) as u8);
        }
    }
    pixels
}

/// The badge icon as straight-alpha BGRA, `size` x `size` pixels, top row
/// first: `base` (a `size` square, see `resample`) with a disc of `colour` in
/// its bottom-right corner, ringed in white, the letter in white on the disc.
/// `coverage` is the letter's anti-aliased mask (one byte per pixel, 255 =
/// fully inked), drawn by the OS because only it has the fonts.
pub fn render_badge(size: usize, colour: [u8; 3], coverage: &[u8], base: &[u8]) -> Vec<u8> {
    assert_eq!(coverage.len(), size * size, "one coverage byte per pixel");
    assert_eq!(base.len(), size * size * 4, "one base pixel per pixel");
    let side = size as f32;
    let [cx, cy, radius] = DISC.map(|f| f * side);
    let ring = RING * side;
    let mut pixels = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let i = y * size + x;
            let distance = (x as f32 + 0.5 - cx).hypot(y as f32 + 0.5 - cy);
            let outer = cover(distance - (radius + ring));
            let disc = cover(distance - radius);
            let ink = f32::from(coverage[i]) / 255.0;
            let lettered = |c: u8| f32::from(c) * (1.0 - ink) + 255.0 * ink;
            // The white ring goes over the base, the disc over the ring.
            let over = |b: u8, c: u8| {
                let ringed = f32::from(b) * (1.0 - outer) + 255.0 * outer;
                (ringed * (1.0 - disc) + lettered(c) * disc).round() as u8
            };
            let p = &base[i * 4..][..4];
            pixels.extend_from_slice(&[
                over(p[0], colour[2]),
                over(p[1], colour[1]),
                over(p[2], colour[0]),
                p[3],
            ]);
        }
    }
    pixels
}
