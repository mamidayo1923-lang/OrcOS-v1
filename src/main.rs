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
use alloc::format;
use spin::Mutex;

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
// 画面に文字をタイピングするための「画面ライター」構造体
// ------------------------------------------------------------------------
pub struct ScreenWriter {
    fb_ptr: *mut u8,
    pitch: usize,
    bpp: usize,
    cursor_x: usize,
    cursor_y: usize,
    min_x: usize, // タイピング領域の左端
    max_x: usize, // タイピング領域の右端
}

impl ScreenWriter {
    pub fn new(fb_ptr: *mut u8, pitch: usize, bpp: usize, start_x: usize, start_y: usize, width: usize) -> Self {
        Self {
            fb_ptr,
            pitch,
            bpp,
            cursor_x: start_x,
            cursor_y: start_y,
            min_x: start_x,
            max_x: start_x + width,
        }
    }

    /// 1文字を画面に描き、カーソルを進める
    pub fn write_char(&mut self, c: char, color: u32) {
        if c == '\n' {
            self.new_line();
            return;
        }

        draw_char(self.fb_ptr, self.pitch, self.bpp, self.cursor_x, self.cursor_y, c, color);
        
        self.cursor_x += 8;

        if self.cursor_x + 8 > self.max_x {
            self.new_line();
        }
    }

    /// ★ 新機能：バックスペースで文字を消す！
    pub fn delete_char(&mut self, bg_color: u32) {
        if self.cursor_x > self.min_x {
            // カーソルを1文字分（8ピクセル）左に戻す
            self.cursor_x -= 8;
            // 戻した場所を背景色で塗りつぶして「消した」ように見せる（文字のサイズは 8x8 ピクセル）
            draw_rect(self.fb_ptr, self.pitch, self.bpp, self.cursor_x, self.cursor_y, 8, 8, bg_color);
        }
    }

    fn new_line(&mut self) {
        self.cursor_x = self.min_x;
        self.cursor_y += 12; // 次の行へ
    }
}

pub static SCREEN_WRITER: Mutex<Option<ScreenWriter>> = Mutex::new(None);
// ------------------------------------------------------------------------

pub unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val);
}

pub fn print_serial(s: &str) {
    for b in s.bytes() {
        unsafe { outb(0x3F8, b); }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    print_serial("KERNEL PANIC!\n");
    loop {
        x86_64::instructions::hlt();
    }
}

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
    for i in 0..height {
        for j in 0..width {
            draw_pixel(fb_ptr, pitch, bpp, x + j, y + i, color);
        }
    }
}

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

fn draw_string(fb_ptr: *mut u8, pitch: usize, bpp: usize, mut x: usize, y: usize, s: &str, color: u32) {
    for c in s.chars() {
        draw_char(fb_ptr, pitch, bpp, x, y, c, color);
        x += 8;
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    assert!(BASE_REVISION.is_supported());

    print_serial("OrcOS Microkernel Booted via Limine!\n");

    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();

    if let Some(memory_map) = MEMORY_MAP_REQUEST.response() {
        for entry in memory_map.entries() {
            if entry.entry_type == EntryType::USABLE {
                let heap_start = entry.base as usize;
                let heap_size = entry.length as usize;
                allocator::init_heap(heap_start, heap_size);
                print_serial("Heap Allocator Initialized!\n");
                break;
            }
        }
    }

    if let Some(response) = FRAMEBUFFER_REQUEST.response() {
        if let Some(fb) = response.framebuffers().first() {
            let pitch = fb.pitch as usize;
            let bpp = fb.bpp as usize;
            let width = fb.width as usize;
            let height = fb.height as usize;
            let ptr = fb.address() as *mut u8;

            draw_rect(ptr, pitch, bpp, 0, 0, width, height, 0x102040);
            let win_x = 50; let win_y = 50; let win_w = 400; let win_h = 200;
            draw_rect(ptr, pitch, bpp, win_x + 5, win_y + 5, win_w, win_h, 0x081020);
            draw_rect(ptr, pitch, bpp, win_x, win_y, win_w, win_h, 0x303030); // ウィンドウの背景は 0x303030
            draw_rect(ptr, pitch, bpp, win_x, win_y, win_w, 20, 0x0050A0);
            draw_string(ptr, pitch, bpp, win_x + 5, win_y + 6, "OrcOS System Info", 0xFFFFFF);
            
            draw_string(ptr, pitch, bpp, win_x + 10, win_y + 40, "Welcome to OrcOS!", 0xFFFFFF);
            draw_string(ptr, pitch, bpp, win_x + 10, win_y + 60, "Microkernel Architecture initialized.", 0x00FF00); 
            draw_string(ptr, pitch, bpp, win_x + 10, win_y + 80, "Architecture: x86_64", 0xFFFF00); 
            
            let writer = ScreenWriter::new(ptr, pitch, bpp, win_x + 10, win_y + 120, win_w - 20);
            *SCREEN_WRITER.lock() = Some(writer);

            let mem_msg = format!("Type something on your keyboard!");
            draw_string(ptr, pitch, bpp, win_x + 10, win_y + 100, &mem_msg, 0x00FFFF); 
        }
    }

    print_serial("Initialization complete. Halting CPU.\n");

    loop {
        x86_64::instructions::hlt();
    }
}
