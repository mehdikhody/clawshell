//! The Windows half of the taskbar identity: finds an account's Claude
//! windows, gives them their own AppUserModelID and paints the badge icon on
//! them. Windows group taskbar buttons by AppUserModelID, so a window with its
//! own id gets its own button. Checked by hand; the decisions (the id, the
//! colours, the pixels) are in `core::badge`.

use crate::core::badge::{BASE_SIDE, DISC, plain_base, render_badge, resample, rgb};
use crate::core::shortcuts::encode_ico;
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{BOOL, w};

/// The window class of Claude's own (Electron) windows. Chromium's hidden
/// helper windows use other classes, so they are not mistaken for the window.
const CLAUDE_WINDOW_CLASS: &str = "Chrome_WidgetWin_1";

/// The base of the badge: Claude's own icon from `exe`, or the plain orange
/// square when there is no exe or no icon in it. `BASE_SIDE` pixels across.
pub fn claude_base(exe: Option<PathBuf>) -> Vec<u8> {
    exe.as_deref().and_then(exe_icon).unwrap_or_else(plain_base)
}

/// Gives every Claude window of `pids` the identity `aumid` and the badge
/// icon, hidden windows included. Returns how many windows it did.
pub fn badge_windows(pids: &[u32], aumid: &str, letter: char, colour: &str, base: &[u8]) -> usize {
    let Some(colour) = rgb(colour) else {
        return 0;
    };
    let windows = claude_windows(pids);
    if windows.is_empty() {
        return 0;
    }
    // Windows' property store is COM.
    // SAFETY: balanced with CoUninitialize below when it succeeded.
    let com = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();

    let small = icon(letter, colour, metric(SM_CXSMICON, 16), base);
    let big = icon(letter, colour, metric(SM_CXICON, 32), base);
    let mut done = 0;
    for hwnd in windows {
        // The id first: the icon is only shown for the group it names.
        if set_aumid(hwnd, aumid).is_err() {
            continue;
        }
        send_icon(hwnd, ICON_SMALL, small);
        send_icon(hwnd, ICON_BIG, big);
        done += 1;
    }
    if com {
        // SAFETY: CoInitializeEx succeeded on this thread above.
        unsafe { CoUninitialize() };
    }
    done
}

/// Sizes stored in a shortcut's icon file: the small and large Explorer and
/// Start sizes, up to the 256 px jumbo icon.
const ICO_SIZES: [i32; 6] = [16, 24, 32, 48, 64, 256];

/// The badge as the bytes of an `.ico` file (`None` when `colour` is not
/// `#rrggbb` or GDI cannot draw the letter).
pub fn badge_ico(letter: char, colour: &str, base: &[u8]) -> Option<Vec<u8>> {
    let colour = rgb(colour)?;
    let images: Option<Vec<(usize, Vec<u8>)>> = ICO_SIZES
        .iter()
        .map(|&size| {
            let side = usize::try_from(size).ok()?;
            let pixels = render_badge(
                side,
                colour,
                &letter_coverage(letter, size)?,
                &resample(base, BASE_SIDE, side),
            );
            Some((side, pixels))
        })
        .collect();
    Some(encode_ico(&images?))
}

/// The badge as the bytes of an `.ico` file of one `size` x `size` image, for
/// the account list (`None` when `colour` is not `#rrggbb` or GDI cannot draw
/// the letter).
pub fn badge_image(letter: char, colour: &str, base: &[u8], size: i32) -> Option<Vec<u8>> {
    let colour = rgb(colour)?;
    let side = usize::try_from(size).ok().filter(|s| *s > 0)?;
    let pixels = render_badge(
        side,
        colour,
        &letter_coverage(letter, size)?,
        &resample(base, BASE_SIDE, side),
    );
    Some(encode_ico(&[(side, pixels)]))
}

/// The icon Windows finds in `exe` at 256 px, as `BASE_SIDE` square BGRA
/// pixels; `None` when the exe has no icon.
fn exe_icon(exe: &Path) -> Option<Vec<u8>> {
    // A NUL-terminated path in a MAX_PATH buffer, as the API takes it.
    let mut file = [0u16; 260];
    let wide: Vec<u16> = exe.as_os_str().encode_wide().collect();
    if wide.len() >= file.len() {
        return None;
    }
    file[..wide.len()].copy_from_slice(&wide);
    let mut icon = HICON::default();
    // SAFETY: one icon is written to `icon`, which is destroyed below.
    let found = unsafe {
        PrivateExtractIconsW(
            &file,
            0,
            BASE_SIDE as i32,
            BASE_SIDE as i32,
            Some(std::slice::from_mut(&mut icon)),
            None,
            0,
        )
    };
    if found == 0 || found == u32::MAX || icon.is_invalid() {
        return None;
    }
    let mut info = ICONINFO::default();
    // SAFETY: the bitmaps GetIconInfo hands out are deleted below, every time.
    let pixels = unsafe {
        let pixels = GetIconInfo(icon, &mut info)
            .ok()
            .and_then(|()| bitmap_pixels(info.hbmColor));
        for bitmap in [info.hbmColor, info.hbmMask] {
            if !bitmap.is_invalid() {
                let _ = DeleteObject(bitmap.into());
            }
        }
        let _ = DestroyIcon(icon);
        pixels
    };
    pixels.map(|pixels| resample(&pixels.0, pixels.1, BASE_SIDE))
}

/// A square bitmap's pixels as 32-bit BGRA, top row first, with its side.
fn bitmap_pixels(bitmap: HBITMAP) -> Option<(Vec<u8>, usize)> {
    let mut bmp = BITMAP::default();
    // SAFETY: `bmp` is the size GetObjectW is told it is.
    let read = unsafe {
        GetObjectW(
            bitmap.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(std::ptr::addr_of_mut!(bmp).cast()),
        )
    };
    let side = bmp.bmWidth;
    if read == 0 || side <= 0 || side != bmp.bmHeight {
        return None;
    }
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: side,
            biHeight: -side,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = vec![0u8; (side * side * 4) as usize];
    // SAFETY: the buffer holds the side x side x 32-bit rows asked for; the DC
    // is deleted here.
    let rows = unsafe {
        let dc = CreateCompatibleDC(None);
        let rows = GetDIBits(
            dc,
            bitmap,
            0,
            side as u32,
            Some(pixels.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(dc);
        rows
    };
    (rows == side).then_some((pixels, side as usize))
}

fn metric(index: SYSTEM_METRICS_INDEX, fallback: i32) -> i32 {
    // SAFETY: plain query.
    match unsafe { GetSystemMetrics(index) } {
        0 => fallback,
        size => size,
    }
}

fn set_aumid(hwnd: HWND, aumid: &str) -> windows::core::Result<()> {
    // SAFETY: `hwnd` is a window handle; the store and value live through the calls.
    unsafe {
        let store: IPropertyStore = SHGetPropertyStoreForWindow(hwnd)?;
        store.SetValue(&PKEY_AppUserModel_ID, &PROPVARIANT::from(aumid))?;
        store.Commit()
    }
}

/// Sets the window's icon. Sent with a timeout so a hung Claude window cannot
/// hang clawshell. The icon handle stays valid for the window's life: a few
/// icons per identity change is all clawshell ever leaks.
fn send_icon(hwnd: HWND, kind: u32, icon: Option<HICON>) {
    let Some(icon) = icon else { return };
    // SAFETY: the message carries a live icon handle; the call is synchronous.
    unsafe {
        SendMessageTimeoutW(
            hwnd,
            WM_SETICON,
            WPARAM(kind as usize),
            LPARAM(icon.0 as isize),
            SMTO_ABORTIFHUNG,
            1000,
            None,
        );
    }
}

/// Top-level, unowned Claude windows of any of `pids`, visible or not.
fn claude_windows(pids: &[u32]) -> Vec<HWND> {
    struct Search<'a> {
        pids: &'a [u32],
        found: Vec<HWND>,
    }

    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: lparam is the `&mut Search` passed to EnumWindows below,
        // alive for the whole (synchronous) enumeration.
        let search = unsafe { &mut *(lparam.0 as *mut Search) };
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if search.pids.contains(&pid)
            && unsafe { GetWindow(hwnd, GW_OWNER) }.map_or(true, |owner| owner.is_invalid())
        {
            let mut class = [0u16; 64];
            let len = unsafe { GetClassNameW(hwnd, &mut class) };
            let len = usize::try_from(len).unwrap_or(0);
            if String::from_utf16_lossy(&class[..len]) == CLAUDE_WINDOW_CLASS {
                search.found.push(hwnd);
            }
        }
        BOOL(1)
    }

    if pids.is_empty() {
        return Vec::new();
    }
    let mut search = Search {
        pids,
        found: Vec::new(),
    };
    // SAFETY: `search` outlives the synchronous enumeration.
    let _ = unsafe { EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize)) };
    search.found
}

/// The badge as a square icon of `size` pixels: GDI draws the letter into a
/// mask (it has the fonts), `core::badge` composes the pixels.
fn icon(letter: char, colour: [u8; 3], size: i32, base: &[u8]) -> Option<HICON> {
    let side = usize::try_from(size).ok().filter(|s| *s > 0)?;
    let coverage = letter_coverage(letter, size)?;
    let pixels = render_badge(side, colour, &coverage, &resample(base, BASE_SIDE, side));

    // SAFETY: every GDI object created here is deleted before returning; the
    // DIB's pixel memory is owned by the bitmap and written within its size.
    unsafe {
        let (colour_bitmap, bits) = dib(size)?;
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u8>(), pixels.len());
        let mask = CreateBitmap(size, size, 1, 1, None);
        let info = ICONINFO {
            fIcon: BOOL(1),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: colour_bitmap,
        };
        let icon = CreateIconIndirect(&info).ok();
        let _ = DeleteObject(mask.into());
        let _ = DeleteObject(colour_bitmap.into());
        icon
    }
}

/// A zeroed top-down 32-bit bitmap of `size` x `size` and its pixel memory.
///
/// # Safety
/// The caller deletes the bitmap, and writes only `size * size * 4` bytes.
unsafe fn dib(size: i32) -> Option<(HBITMAP, *mut c_void)> {
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size,
            biHeight: -size,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut c_void = std::ptr::null_mut();
    let bitmap =
        unsafe { CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0) }.ok()?;
    if bits.is_null() {
        let _ = unsafe { DeleteObject(bitmap.into()) };
        return None;
    }
    unsafe { std::ptr::write_bytes(bits.cast::<u8>(), 0, (size * size * 4) as usize) };
    Some((bitmap, bits))
}

/// White-on-black text drawn by GDI, read back as one coverage byte per pixel.
fn letter_coverage(letter: char, size: i32) -> Option<Vec<u8>> {
    let mut text: Vec<u16> = letter.to_string().encode_utf16().collect();
    // SAFETY: the memory DC, bitmap and font are created and deleted here; the
    // DIB memory is read only while the bitmap is alive.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return None;
        }
        let Some((bitmap, bits)) = dib(size) else {
            let _ = DeleteDC(dc);
            return None;
        };
        let old_bitmap = SelectObject(dc, bitmap.into());
        let [cx, cy, radius] = DISC.map(|f| f * size as f32);
        let font = CreateFontW(
            -(radius * 2.0 * 0.78) as i32,
            0,
            0,
            0,
            700,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            ANTIALIASED_QUALITY,
            0,
            w!("Segoe UI"),
        );
        let old_font = SelectObject(dc, font.into());
        SetBkMode(dc, TRANSPARENT);
        SetTextColor(dc, COLORREF(0x00FF_FFFF));
        // The letter is centred on the disc, the only part of the mask inked.
        let mut rect = RECT {
            left: (cx - radius) as i32,
            top: (cy - radius) as i32,
            right: (cx + radius) as i32,
            bottom: (cy + radius) as i32,
        };
        DrawTextW(
            dc,
            &mut text,
            &mut rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        let _ = GdiFlush();

        let pixels = std::slice::from_raw_parts(bits.cast::<u8>(), (size * size * 4) as usize);
        // Grey text: any channel is the coverage.
        let coverage = pixels.as_chunks::<4>().0.iter().map(|p| p[1]).collect();

        SelectObject(dc, old_font);
        SelectObject(dc, old_bitmap);
        let _ = DeleteObject(font.into());
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(dc);
        Some(coverage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gdi_draws_the_letter_into_a_mask() {
        let mask = letter_coverage('W', 32).expect("GDI draws text");
        assert_eq!(mask.len(), 32 * 32);
        let inked = mask.iter().filter(|c| **c > 0).count();
        assert!(inked > 10, "the letter leaves ink");
        assert!(inked < 32 * 32 / 4, "the letter is not a block");
        // Inside the disc only: the top of the icon has no ink.
        assert!(mask[..12 * 32].iter().all(|c| *c == 0));
        assert!(mask[20 * 32..].iter().any(|c| *c > 0));
    }

    #[test]
    fn the_badge_becomes_an_icon_handle() {
        let handle = icon('W', [0x25, 0x63, 0xeb], 32, &plain_base()).expect("an icon");
        // SAFETY: the handle was just created and is not used elsewhere.
        unsafe { DestroyIcon(handle).unwrap() };
    }

    #[test]
    fn the_badge_becomes_an_ico_file_with_every_size() {
        let ico = badge_ico('W', "#2563eb", &plain_base()).expect("an ico");

        assert_eq!(&ico[..4], &[0, 0, 1, 0], "an icon file");
        assert_eq!(
            u16::from_le_bytes([ico[4], ico[5]]) as usize,
            ICO_SIZES.len()
        );
        assert_eq!(badge_ico('W', "blue", &plain_base()), None);
    }

    #[test]
    fn the_account_icon_is_one_image_of_the_asked_size() {
        let ico = badge_image('W', "#2563eb", &plain_base(), 64).expect("an ico");

        assert_eq!(&ico[..4], &[0, 0, 1, 0], "an icon file");
        assert_eq!(u16::from_le_bytes([ico[4], ico[5]]), 1, "one image");
        assert_eq!(ico[6], 64, "64 px wide");
    }

    #[test]
    fn claudes_own_icon_is_the_base_of_the_badge() {
        let explorer = std::path::PathBuf::from(r"C:\Windows\explorer.exe");
        let base = claude_base(Some(explorer));

        assert_eq!(base.len(), BASE_SIDE * BASE_SIDE * 4);
        assert!(
            base.chunks_exact(4).any(|p| p[3] == 255),
            "the icon has ink"
        );
        assert_eq!(claude_base(None), plain_base());
    }

    #[test]
    fn windows_loads_the_badge_ico_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("badge.ico");
        std::fs::write(&path, badge_ico('W', "#2563eb", &plain_base()).unwrap()).unwrap();

        for size in [16, 32, 256] {
            // SAFETY: loads from a file path that outlives the call; the
            // handle is destroyed at once.
            let handle = unsafe {
                LoadImageW(
                    None,
                    &windows::core::HSTRING::from(path.as_os_str()),
                    IMAGE_ICON,
                    size,
                    size,
                    LR_LOADFROMFILE,
                )
            }
            .expect("Windows reads the .ico");
            unsafe { DestroyIcon(HICON(handle.0)).unwrap() };
        }
    }
}
