//! An account's badge: the letter and colour shown on its row, in its taskbar
//! button and on its icon. All pure; the OS adapter only paints what this says.

/// The colours handed out to new accounts, in order. Each is dark enough to
/// carry a white letter.
pub const PALETTE: [&str; 10] = [
    "#2563eb", "#dc2626", "#16a34a", "#9333ea", "#ea580c", "#0891b2", "#db2777", "#a16207",
    "#4f46e5", "#0d9488",
];

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

/// The badge icon as straight-alpha BGRA, `size` x `size` pixels, top row
/// first: a rounded square of `colour` with the letter in white. `coverage`
/// is the letter's anti-aliased mask (one byte per pixel, 255 = fully inked),
/// drawn by the OS because only it has the fonts.
pub fn render_badge(size: usize, colour: [u8; 3], coverage: &[u8]) -> Vec<u8> {
    assert_eq!(coverage.len(), size * size, "one coverage byte per pixel");
    let half = size as f32 / 2.0;
    let radius = size as f32 * 0.22;
    let mut pixels = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            // Signed distance to the rounded square: negative inside.
            let dx = (x as f32 + 0.5 - half).abs() - (half - radius);
            let dy = (y as f32 + 0.5 - half).abs() - (half - radius);
            let outside = dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - radius;
            let alpha = (0.5 - outside).clamp(0.0, 1.0);
            let ink = f32::from(coverage[y * size + x]) / 255.0;
            let mix = |c: u8| (f32::from(c) * (1.0 - ink) + 255.0 * ink).round() as u8;
            pixels.extend_from_slice(&[
                mix(colour[2]),
                mix(colour[1]),
                mix(colour[0]),
                (alpha * 255.0).round() as u8,
            ]);
        }
    }
    pixels
}
