#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::arch::asm;
use limine::request::FramebufferRequest;
use limine::BaseRevision;

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
/// ※ QEMUの起動オプションに `-serial stdio` を付けるとターミナルに表示される
fn print_serial(s: &str) {
    for b in s.bytes() {
        unsafe { outb(0x3F8, b); }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    print_serial("KERNEL PANIC!\n");
    loop {
        // CPUを休止状態にして無駄な電力消費と発熱を防ぐ
        unsafe { asm!("hlt"); }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // ブートローダがLimineプロトコルをサポートしているか確認
    assert!(BASE_REVISION.is_supported());

    // シリアルポートへ起動メッセージを出力
    print_serial("OrcOS Microkernel Booted via Limine!\n");

    // フレームバッファの取得と画面の塗りつぶし（成功の証として青色にする）
    if let Some(response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(framebuffers) = response.framebuffers() {
            if let Some(fb) = framebuffers.first() {
                let pitch = fb.pitch() as usize;
                let bpp = fb.bpp() as usize;
                let width = fb.width() as usize;
                let height = fb.height() as usize;
                let ptr = fb.addr() as *mut u8;

                for y in 0..height {
                    for x in 0..width {
                        let offset = y * pitch + x * (bpp / 8);
                        unsafe {
                            *ptr.add(offset) = 0xFF;     // B (Blue)
                            *ptr.add(offset + 1) = 0x00; // G (Green)
                            *ptr.add(offset + 2) = 0x00; // R (Red)
                        }
                    }
                }
            }
        }
    }

    print_serial("Initialization complete. Halting CPU.\n");

    // カーネルが終了しないようにCPUを待機状態にするループ
    loop {
        unsafe { asm!("hlt"); }
    }
}
