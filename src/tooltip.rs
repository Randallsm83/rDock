//! Tooltip support for dock items - styled popup window

use std::cell::RefCell;
use std::sync::Once;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM, LRESULT, COLORREF, SIZE};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

static REGISTER_CLASS: Once = Once::new();
const TOOLTIP_CLASS: &str = "RDockTooltip";
const CORNER_RADIUS: i32 = 6;

// Thread-local storage for tooltip state
thread_local! {
    static TOOLTIP_BG: RefCell<u32> = const { RefCell::new(0x2E1E1E) };
    static TOOLTIP_TEXT: RefCell<u32> = const { RefCell::new(0xE0E0E0) };
    static TOOLTIP_FONT: RefCell<HFONT> = const { RefCell::new(HFONT(std::ptr::null_mut())) };
}

pub struct Tooltip {
    hwnd: HWND,
    visible: bool,
    current_text: String,
    font: HFONT,
}

unsafe extern "system" fn tooltip_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            
            // Get colors from thread-local storage
            let bg_color = TOOLTIP_BG.with(|c| *c.borrow());
            let text_color = TOOLTIP_TEXT.with(|c| *c.borrow());
            
            // Get window dimensions
            let mut rect = std::mem::zeroed();
            let _ = GetClientRect(hwnd, &mut rect);
            
            // Create rounded region for the window
            let rgn = CreateRoundRectRgn(0, 0, rect.right + 1, rect.bottom + 1, CORNER_RADIUS, CORNER_RADIUS);
            let _ = SelectClipRgn(hdc, rgn);
            
            // Fill background
            let bg_brush = CreateSolidBrush(COLORREF(bg_color));
            FillRect(hdc, &rect, bg_brush);
            let _ = DeleteObject(bg_brush);
            
            
            // Get window text
            let len = GetWindowTextLengthW(hwnd);
            if len > 0 {
                let mut buf = vec![0u16; (len + 1) as usize];
                GetWindowTextW(hwnd, &mut buf);
                
                // Select our font
                let font = TOOLTIP_FONT.with(|f| *f.borrow());
                let old_font = SelectObject(hdc, font);
                
                // Set text properties
                let _ = SetBkMode(hdc, TRANSPARENT);
                let _ = SetTextColor(hdc, COLORREF(text_color));
                
                // Draw text centered
                let mut text_rect = rect;
                text_rect.left += 12;
                text_rect.right -= 12;
                let _ = DrawTextW(hdc, &mut buf, &mut text_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
                
                SelectObject(hdc, old_font);
            }
            
            let _ = DeleteObject(rgn);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_ERASEBKGND => {
            // Prevent flicker
            LRESULT(1)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn register_class() {
    REGISTER_CLASS.call_once(|| {
        unsafe {
            let class_name: Vec<u16> = TOOLTIP_CLASS.encode_utf16().chain(std::iter::once(0)).collect();
            let hinstance = GetModuleHandleW(PCWSTR::null()).unwrap_or_default();
            
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW | CS_DROPSHADOW,
                lpfnWndProc: Some(tooltip_wnd_proc),
                hInstance: hinstance.into(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                lpszClassName: PCWSTR(class_name.as_ptr()),
                hbrBackground: HBRUSH(0 as *mut _), // No background - we paint it ourselves
                ..Default::default()
            };
            
            RegisterClassExW(&wc);
        }
    });
}

/// Parse a hex color string like "#1e1e2e" to BGR u32 for Windows
fn parse_color_bgr(hex: &str) -> u32 {
    let hex = hex.trim_start_matches('#');
    if hex.len() >= 6 {
        if let Ok(rgb) = u32::from_str_radix(hex, 16) {
            // Convert RGB to BGR for Windows
            let r = (rgb >> 16) & 0xFF;
            let g = (rgb >> 8) & 0xFF;
            let b = rgb & 0xFF;
            return (b << 16) | (g << 8) | r;
        }
    }
    0x2E1E1E // fallback dark color
}

/// Lighten a BGR color for the border
#[allow(dead_code)]
fn lighten_color(bgr: u32, amount: u32) -> u32 {
    let b = ((bgr >> 16) & 0xFF).saturating_add(amount).min(255);
    let g = ((bgr >> 8) & 0xFF).saturating_add(amount).min(255);
    let r = (bgr & 0xFF).saturating_add(amount).min(255);
    (b << 16) | (g << 8) | r
}

impl Tooltip {
    /// Create a new tooltip with colors derived from background_color
    pub fn new_with_color(_parent_hwnd: HWND, background_color: &str) -> Option<Self> {
        // Set colors in thread-local storage
        let bg = parse_color_bgr(background_color);
        let text = 0xE0E0E0u32; // Light gray text
        
        TOOLTIP_BG.with(|c| *c.borrow_mut() = bg);
        TOOLTIP_TEXT.with(|c| *c.borrow_mut() = text);
        
        Self::new_internal()
    }
    
    #[allow(dead_code)]
    pub fn new(_parent_hwnd: HWND) -> Option<Self> {
        Self::new_internal()
    }
    
    fn new_internal() -> Option<Self> {
        register_class();
        
        unsafe {
            let class_name: Vec<u16> = TOOLTIP_CLASS.encode_utf16().chain(std::iter::once(0)).collect();
            let hinstance = GetModuleHandleW(PCWSTR::null()).ok()?;
            
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                PCWSTR(class_name.as_ptr()),
                PCWSTR::null(),
                WS_POPUP,
                0, 0, 0, 0,
                None,
                None,
                hinstance,
                None,
            ).ok()?;
            
            // Create a nice font - Segoe UI Semibold
            let font_name: Vec<u16> = "Segoe UI".encode_utf16().chain(std::iter::once(0)).collect();
            let font = CreateFontW(
                -15, // Height (negative = character height)
                0,   // Width (0 = default)
                0, 0, // Escapement, orientation
                FW_SEMIBOLD.0 as i32,
                0, 0, 0, // Italic, underline, strikeout
                DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                CLEARTYPE_QUALITY.0 as u32,
                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                PCWSTR(font_name.as_ptr()),
            );
            
            // Store font in thread-local for paint handler
            TOOLTIP_FONT.with(|f| *f.borrow_mut() = font);
            
            Some(Self {
                hwnd,
                visible: false,
                current_text: String::new(),
                font,
            })
        }
    }
    
    pub fn show(&mut self, text: &str, x: i32, y: i32) {
        if text.is_empty() {
            self.hide();
            return;
        }
        
        unsafe {
            // Update text if changed
            if text != self.current_text || !self.visible {
                self.current_text = text.to_string();
                
                // Set window text
                let text_wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
                let _ = SetWindowTextW(self.hwnd, PCWSTR(text_wide.as_ptr()));
                
                // Calculate size needed for text with our font
                let hdc = GetDC(self.hwnd);
                let old_font = SelectObject(hdc, self.font);
                let mut size = SIZE::default();
                let _ = GetTextExtentPoint32W(hdc, &text_wide[..text_wide.len()-1], &mut size);
                SelectObject(hdc, old_font);
                let _ = ReleaseDC(self.hwnd, hdc);
                
                // Add generous padding to ensure text fits
                let width = size.cx + 32;
                let height = size.cy + 12;
                
                // Position above cursor, centered on x
                let tip_x = x - width / 2;
                let tip_y = y - height - 10;
                
                // Move and resize
                let _ = SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    tip_x, tip_y, width, height,
                    SWP_NOACTIVATE,
                );
                
                // Apply font to window for painting
                SendMessageW(self.hwnd, WM_SETFONT, WPARAM(self.font.0 as usize), LPARAM(1));
                
                if !self.visible {
                    let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
                    self.visible = true;
                } else {
                    // Force redraw
                    let _ = InvalidateRect(self.hwnd, None, true);
                }
            }
        }
    }
    
    pub fn hide(&mut self) {
        if self.visible {
            unsafe {
                let _ = ShowWindow(self.hwnd, SW_HIDE);
            }
            self.visible = false;
            self.current_text.clear();
        }
    }
}

impl Drop for Tooltip {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.font);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
