use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use crate::print_serial;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub const TIMER_INTERRUPT_ID: u8 = PIC_1_OFFSET;
pub const KEYBOARD_INTERRUPT_ID: u8 = PIC_1_OFFSET + 1;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt[TIMER_INTERRUPT_ID as usize].set_handler_fn(timer_interrupt_handler);
        idt[KEYBOARD_INTERRUPT_ID as usize].set_handler_fn(keyboard_interrupt_handler);
        idt
    };

    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
        Mutex::new(Keyboard::new(ScancodeSet1::new(), layouts::Us104Key, HandleControl::Ignore));
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    print_serial("EXCEPTION: BREAKPOINT\n");
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        PICS.lock().notify_end_of_interrupt(TIMER_INTERRUPT_ID);
    }
}

/// キーボード割り込み（バックスペース対応！）
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    let mut keyboard = KEYBOARD.lock();
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => {
                    let mut buf = [0; 4];
                    crate::print_serial(character.encode_utf8(&mut buf));

                    if let Some(ref mut writer) = *crate::SCREEN_WRITER.lock() {
                        // ★ 新機能：バックスペース文字（\x08）が送られてきた時の処理
                        if character == '\x08' {
                            // ウィンドウの背景色（0x303030）で塗りつぶして文字を消す
                            writer.delete_char(0x303030);
                        } else {
                            // 通常の文字は緑色で描画
                            writer.write_char(character, 0x00FF00);
                        }
                    }
                },
                DecodedKey::RawKey(_key) => {
                    print_serial("<RAW>");
                }
            }
        }
    }

    unsafe {
        PICS.lock().notify_end_of_interrupt(KEYBOARD_INTERRUPT_ID);
    }
}