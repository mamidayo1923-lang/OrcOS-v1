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

/// 指定された座標に1文字を描画する関数
fn draw_char(fb_ptr: *mut u8, pitch: usize, bpp: usize, x: usize, y: usize, c: char, color: u32) {
    // font8x8から文字のビットマップデータを取得
    if let Some(glyph) = BASIC_FONTS.get(c) {
        for (row_idx, row_data) in glyph.iter().enumerate() {
            for col_idx in 0..8 {
                // ビットが立っている（1である）か判定
                if (*row_data & (1 << col_idx)) != 0 {
                    let pixel_offset = (y + row_idx) * pitch + (x + col_idx) * (bpp / 8);
                    unsafe {
                        // LimineのデフォルトはB-G-R-Aの順序
                        let ptr = fb_ptr.add(pixel_offset);
                        *ptr = (color & 0xFF) as u8;                 // B (Blue)
                        *ptr.add(1) = ((color >> 8) & 0xFF) as u8;   // G (Green)
                        *ptr.add(2) = ((color >> 16) & 0xFF) as u8;  // R (Red)
                    }
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
    // ブートローダがLimineプロトコルをサポートしているか確認
    assert!(BASE_REVISION.is_supported());

    // シリアルポートへ起動メッセージを出力
    print_serial("OrcOS Microkernel Booted via Limine!\n");

    // フレームバッファの取得と画面描画
    // ★ 修正箇所：limine 0.6.5の最新の記述方法に合わせてスッキリさせたよ！
    if let Some(response) = FRAMEBUFFER_REQUEST.response() {
        if let Some(fb) = response.framebuffers().first() {
            let pitch = fb.pitch as usize;
            let bpp = fb.bpp as usize;
            let width = fb.width as usize;
            let height = fb.height as usize;
            let ptr = fb.address() as *mut u8;

            // 背景を暗いネイビーブルーに塗りつぶす
            for y in 0..height {
                for x in 0..width {
                    let offset = y * pitch + x * (bpp / 8);
                    unsafe {
                        *ptr.add(offset) = 0x40;     // B
                        *ptr.add(offset + 1) = 0x20; // G
                        *ptr.add(offset + 2) = 0x10; // R
                    }
                }
            }

            // テキストの描画 (X: 10, Y: 10) に白色 (0xFFFFFF) で出力
            draw_string(ptr, pitch, bpp, 10, 10, "Welcome to OrcOS!", 0xFFFFFF);
            // (X: 10, Y: 30) に緑色 (0x00FF00) で出力
            draw_string(ptr, pitch, bpp, 10, 30, "Microkernel Architecture initialized.", 0x00FF00); 
            // (X: 10, Y: 50) に黄色 (0xFFFF00) で出力
            draw_string(ptr, pitch, bpp, 10, 50, "Architecture: x86_64", 0xFFFF00); 
        }
    }

    print_serial("Initialization complete. Halting CPU.\n");

    // カーネルが終了しないようにCPUを待機状態にするループ
    loop {
        unsafe { asm!("hlt"); }
    }
}
