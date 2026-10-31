//! The Windows half of the taskbar identity: finds an account's Claude
//! windows, gives them their own AppUserModelID and paints the badge icon on
//! them. Windows group taskbar buttons by AppUserModelID, so a window with its
//! own id gets its own button. Checked by hand; the decisions (the id, the
//! colours, the pixels) are in `core::badge`.

use crate::core::badge::{render_badge, rgb};
use std::ffi::c_void;
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

/// Gives every Claude window of `pids` the identity `aumid` and the badge
/// icon, hidden windows included. Returns how many windows it did.
pub fn badge_windows(pids: &[u32], aumid: &str, letter: char, colour: &str) -> usize {
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

    let small = icon(letter, colour, metric(SM_CXSMICON, 16));
    let big = icon(letter, colour, metric(SM_CXICON, 32));
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
fn icon(letter: char, colour: [u8; 3], size: i32) -> Option<HICON> {
    let side = usize::try_from(size).ok().filter(|s| *s > 0)?;
    let coverage = letter_coverage(letter, size)?;
    let pixels = render_badge(side, colour, &coverage);

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
        let font = CreateFontW(
            -(size * 62 / 100),
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
        // A capital sits above its line's centre; nudge it down to look centred.
        let mut rect = RECT {
            left: 0,
            top: size / 16,
            right: size,
            bottom: size + size / 16,
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
        assert!(inked > 30, "the letter leaves ink");
        assert!(inked < 32 * 32 / 2, "the letter is not a block");
        // Centred: the middle band holds ink, the outer border does not.
        assert!(mask[16 * 32 + 8..16 * 32 + 24].iter().any(|c| *c > 0));
        assert!(mask[..32].iter().all(|c| *c == 0));
    }

    #[test]
    fn the_badge_becomes_an_icon_handle() {
        let handle = icon('W', [0x25, 0x63, 0xeb], 32).expect("an icon");
        // SAFETY: the handle was just created and is not used elsewhere.
        unsafe { DestroyIcon(handle).unwrap() };
    }
}
