#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::arch::asm;
use limine::request::FramebufferRequest;
use limine::BaseRevision;
use font8x8::{BASIC_FONTS, UnicodeFonts};

// Limineのベースプロトコルのリビジョン指定
#[used]
#[link_section = ".requests"]
static BASE_REVISION: BaseRevision = BaseRevision::new();

// Limineへフレームバッファ（描画領域）を要求する
#[used]
#[link_section = ".requests"]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

/// I/Oポートへのバイト書き込み（シリアル通信用）
unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val);
}

/// COM1シリアルポートへ文字列を出力する関数
fn print_serial(s: &str) {
    for b in s.bytes() {
        unsafe { outb(0x3F8, b); }
    }
}

/// パニックハンドラ：OSのカーネルがクラッシュした時に呼ばれる
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    print_serial("KERNEL PANIC!\n");
    loop {
        // CPUを休止状態にして無駄な電力消費と発熱を防ぐ
        unsafe { asm!("hlt"); }
    }
}

/// 任意の色のピクセルを描画する基本関数
fn draw_pixel(fb_ptr: *mut u8, pitch: usize, bpp: usize, x: usize, y: usize, color: u32) {
    let pixel_offset = y * pitch + x * (bpp / 8);
    unsafe {
        let ptr = fb_ptr.add(pixel_offset);
        *ptr = (color & 0xFF) as u8;                 // B (Blue)
        *ptr.add(1) = ((color >> 8) & 0xFF) as u8;   // G (Green)
        *ptr.add(2) = ((color >> 16) & 0xFF) as u8;  // R (Red)
    }
}

/// 塗りつぶしの四角形を描画する関数（新機能！）
fn draw_rect(fb_ptr: *mut u8, pitch: usize, bpp: usize, x: usize, y: usize, width: usize, height: usize, color: u32) {
    for i in 0..height {
        for j in 0..width {
            draw_pixel(fb_ptr, pitch, bpp, x + j, y + i, color);
        }
    }
}

/// 指定された座標に1文字を描画する関数
fn draw_char(fb_ptr: *mut u8, pitch: usize, bpp: usize, x: usize, y: usize, c: char, color: u32) {
    if let Some(glyph) = BASIC_FONTS.get(c) {
        for (row_idx, row_data) in glyph.iter().enumerate() {
            for col_idx in 0..8 {
                if (*row_data & (1 << col_idx)) != 0 {
                    draw_pixel(fb_ptr, pitch, bpp, x + col_idx, y + row_idx, color);
                }
            }
        }
    }
}

/// 文字列を描画する関数
fn draw_string(fb_ptr: *mut u8, pitch: usize, bpp: usize, mut x: usize, y: usize, s: &str, color: u32) {
    for c in s.chars() {
        draw_char(fb_ptr, pitch, bpp, x, y, c, color);
        x += 8; // 次の文字へX座標を8ピクセルずらす
    }
}

/// OSのエントリポイント
#[no_mangle]
pub extern "C" fn _start() -> ! {
    assert!(BASE_REVISION.is_supported());

    print_serial("OrcOS Microkernel Booted via Limine!\n");

    // ★ get_response() から response() に修正完了！
    if let Some(response) = FRAMEBUFFER_REQUEST.response() {
        if let Some(fb) = response.framebuffers().first() {
            let pitch = fb.pitch as usize;
            let bpp = fb.bpp as usize;
            let width = fb.width as usize;
            let height = fb.height as usize;
            let ptr = fb.address() as *mut u8;

            // 背景を暗いネイビーブルーに塗りつぶす (R:0x10, G:0x20, B:0x40)
            draw_rect(ptr, pitch, bpp, 0, 0, width, height, 0x102040);

            // ---------------------------------------------
            // ウィンドウっぽいUIを描画してみよう！
            // ---------------------------------------------
            let win_x = 50;
            let win_y = 50;
            let win_w = 400;
            let win_h = 200;
            
            // 1. ウィンドウの影（少し右下にずらして暗い色を置く）
            draw_rect(ptr, pitch, bpp, win_x + 5, win_y + 5, win_w, win_h, 0x081020);
            
            // 2. ウィンドウ本体（ダークグレー）
            draw_rect(ptr, pitch, bpp, win_x, win_y, win_w, win_h, 0x303030);
            
            // 3. タイトルバー（青色）
            draw_rect(ptr, pitch, bpp, win_x, win_y, win_w, 20, 0x0050A0);

            // 4. タイトルバーのテキスト
            draw_string(ptr, pitch, bpp, win_x + 5, win_y + 6, "OrcOS System Info", 0xFFFFFF);
            
            // 5. ウィンドウ内のテキスト
            draw_string(ptr, pitch, bpp, win_x + 10, win_y + 40, "Welcome to OrcOS!", 0xFFFFFF);
            draw_string(ptr, pitch, bpp, win_x + 10, win_y + 60, "Microkernel Architecture initialized.", 0x00FF00); 
            draw_string(ptr, pitch, bpp, win_x + 10, win_y + 80, "Architecture: x86_64", 0xFFFF00); 
            draw_string(ptr, pitch, bpp, win_x + 10, win_y + 100, "Limine Bootloader: OK", 0x00FFFF); 
        }
    }

    print_serial("Initialization complete. Halting CPU.\n");

    loop {
        unsafe { asm!("hlt"); }
    }
}
