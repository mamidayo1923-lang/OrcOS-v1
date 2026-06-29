#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

pub mod gdt;
pub mod interrupts;
pub mod allocator;

use core::panic::PanicInfo;
use core::arch::asm;
use limine::request::{FramebufferRequest, MemoryMapRequest};
use limine::memory_map::EntryType;
use limine::BaseRevision;
use font8x8::{BASIC_FONTS, UnicodeFonts};
use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;

#[used]
#[link_section = ".requests"]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[link_section = ".requests"]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[link_section = ".requests"]
static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

// ------------------------------------------------------------------------
// ★ 新機能：OSが「記憶」するためのグローバル変数（これがレイヤーの元になります！）
// ------------------------------------------------------------------------
lazy_static! {
    // タイピングされた文字を記憶する場所
    pub static ref TYPED_TEXT: Mutex<String> = Mutex::new(String::new());
}
// マウスの座標を記憶する場所
pub static MOUSE_X: Mutex<i32> = Mutex::new(400);
pub static MOUSE_Y: Mutex<i32> = Mutex::new(300);
// ------------------------------------------------------------------------

pub unsafe fn outb(port: u16, val: u8) { asm!("out dx, al", in("dx") port, in("al") val); }
pub fn print_serial(s: &str) { for b in s.bytes() { unsafe { outb(0x3F8, b); } } }

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! { loop { unsafe { asm!("hlt"); } } }

// ------------------------------------------------------------------------
// 描画関数（すべて「裏画面（fb_ptr）」に対して描くように進化！）
// ------------------------------------------------------------------------
fn draw_pixel(fb_ptr: *mut u8, pitch: usize, bpp: usize, x: usize, y: usize, color: u32) {
    let pixel_offset = y * pitch + x * (bpp / 8);
    unsafe {
        let ptr = fb_ptr.add(pixel_offset);
        *ptr = (color & 0xFF) as u8;                 
        *ptr.add(1) = ((color >> 8) & 0xFF) as u8;   
        *ptr.add(2) = ((color >> 16) & 0xFF) as u8;  
    }
}

fn draw_rect(fb_ptr: *mut u8, pitch: usize, bpp: usize, x: usize, y: usize, width: usize, height: usize, color: u32) {
    for i in 0..height { for j in 0..width { draw_pixel(fb_ptr, pitch, bpp, x + j, y + i, color); } }
}

fn draw_char(fb_ptr: *mut u8, pitch: usize, bpp: usize, x: usize, y: usize, c: char, color: u32) {
    if let Some(glyph) = BASIC_FONTS.get(c) {
        for (row_idx, row_data) in glyph.iter().enumerate() {
            for col_idx in 0..8 {
                if (*row_data & (1 << col_idx)) != 0 { draw_pixel(fb_ptr, pitch, bpp, x + col_idx, y + row_idx, color); }
            }
        }
    }
}

fn draw_text_box(fb_ptr: *mut u8, pitch: usize, bpp: usize, start_x: usize, start_y: usize, max_w: usize, text: &str, color: u32) {
    let mut cx = start_x;
    let mut cy = start_y;
    for c in text.chars() {
        if c == '\n' { cx = start_x; cy += 12; continue; }
        draw_char(fb_ptr, pitch, bpp, cx, cy, c, color);
        cx += 8;
        if cx + 8 > start_x + max_w { cx = start_x; cy += 12; }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    assert!(BASE_REVISION.is_supported());

    gdt::init();
    interrupts::init_idt();
    
    unsafe { interrupts::PICS.lock().initialize() };
    interrupts::MOUSE.lock().init(); // マウスの電源ON！
    x86_64::instructions::interrupts::enable();

    // メモリ（ヒープ）の初期化
    if let Some(memory_map) = MEMORY_MAP_REQUEST.response() {
        for entry in memory_map.entries() {
            if entry.entry_type == EntryType::USABLE {
                allocator::init_heap(entry.base as usize, entry.length as usize);
                break;
            }
        }
    }

    // ★ 画面情報の取得と、「裏画面（ダブルバッファ）」の作成！
    let fb_response = FRAMEBUFFER_REQUEST.response().unwrap();
    let fb = fb_response.framebuffers().first().unwrap();
    let pitch = fb.pitch as usize;
    let bpp = fb.bpp as usize;
    let width = fb.width as usize;
    let height = fb.height as usize;
    let real_fb_ptr = fb.address() as *mut u8;

    // メモリ領域に「本物の画面と全く同じサイズの裏画面」を作る！
    let buffer_size = height * pitch;
    let mut back_buffer = alloc::vec![0u8; buffer_size];
    let back_fb_ptr = back_buffer.as_mut_ptr();

    let win_x = 50; let win_y = 50; let win_w = 400; let win_h = 200;

    // =========================================================
    // ★ OSのメイン・レンダリングループ（ずっと回り続ける！）
    // =========================================================
    loop {
        // --- 【レイヤー1】 一番下の背景 ---
        draw_rect(back_fb_ptr, pitch, bpp, 0, 0, width, height, 0x102040);
        
        // --- 【レイヤー2】 ウィンドウ ---
        draw_rect(back_fb_ptr, pitch, bpp, win_x + 5, win_y + 5, win_w, win_h, 0x081020); // 影
        draw_rect(back_fb_ptr, pitch, bpp, win_x, win_y, win_w, win_h, 0x303030); // 本体
        draw_rect(back_fb_ptr, pitch, bpp, win_x, win_y, win_w, 20, 0x0050A0); // タイトルバー
        
        // --- 【レイヤー3】 固定の文字 ---
        draw_string(back_fb_ptr, pitch, bpp, win_x + 5, win_y + 6, "OrcOS System Info", 0xFFFFFF);
        draw_string(back_fb_ptr, pitch, bpp, win_x + 10, win_y + 40, "Welcome to OrcOS Layer System!", 0xFFFFFF);
        
        // --- 【レイヤー4】 タイピングされた動的な文字 ---
        let text = TYPED_TEXT.lock();
        draw_text_box(back_fb_ptr, pitch, bpp, win_x + 10, win_y + 70, win_w - 20, &text, 0x00FF00);
        drop(text); // ロック解除

        // --- 【レイヤー5】 一番上のマウスカーソル ---
        let mx = *MOUSE_X.lock() as usize;
        let my = *MOUSE_Y.lock() as usize;
        draw_rect(back_fb_ptr, pitch, bpp, mx, my, 5, 5, 0xFFFFFF);

        // --- ★ 仕上げ：完成した裏画面を、本物の画面に一気にコピーする！ ---
        unsafe {
            core::ptr::copy_nonoverlapping(back_fb_ptr, real_fb_ptr, buffer_size);
        }

        // 次の割り込み（キーボードやマウスの操作）が来るまでCPUを休ませる
        x86_64::instructions::hlt();
    }
}