use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use x86_64::instructions::port::Port;
use crate::print_serial;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;
pub const TIMER_INTERRUPT_ID: u8 = PIC_1_OFFSET;
pub const KEYBOARD_INTERRUPT_ID: u8 = PIC_1_OFFSET + 1;
pub const MOUSE_INTERRUPT_ID: u8 = PIC_1_OFFSET + 12;

pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt[TIMER_INTERRUPT_ID as usize].set_handler_fn(timer_interrupt_handler);
        idt[KEYBOARD_INTERRUPT_ID as usize].set_handler_fn(keyboard_interrupt_handler);
        idt[MOUSE_INTERRUPT_ID as usize].set_handler_fn(mouse_interrupt_handler); // マウス追加
        idt
    };

    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
        Mutex::new(Keyboard::new(ScancodeSet1::new(), layouts::Us104Key, HandleControl::Ignore));

    // ★ マウスのドライバ本体をここに配置
    pub static ref MOUSE: Mutex<Mouse> = Mutex::new(Mouse::new());
}

pub fn init_idt() { IDT.load(); }

extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame) {}
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe { PICS.lock().notify_end_of_interrupt(TIMER_INTERRUPT_ID); }
}

/// キーボード割り込み：メインループの「記憶（TYPED_TEXT）」を更新するだけ！
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    let mut keyboard = KEYBOARD.lock();
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => {
                    let mut text = crate::TYPED_TEXT.lock();
                    if character == '\x08' {
                        text.pop(); // バックスペースなら最後の文字を消す
                    } else {
                        text.push(character); // 文字を追加
                    }
                },
                DecodedKey::RawKey(_) => {}
            }
        }
    }
    unsafe { PICS.lock().notify_end_of_interrupt(KEYBOARD_INTERRUPT_ID); }
}

/// マウス割り込み：メインループの「座標」を更新するだけ！
extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    if let Some((x, y)) = MOUSE.lock().process_interrupt() {
        *crate::MOUSE_X.lock() = x;
        *crate::MOUSE_Y.lock() = y;
    }
    unsafe { PICS.lock().notify_end_of_interrupt(MOUSE_INTERRUPT_ID); }
}

// ---------------------------------------------------------
// マウスドライバの構造体と処理
// ---------------------------------------------------------
pub struct Mouse {
    data_port: Port<u8>,
    status_port: Port<u8>,
    cycle: u8,
    packet: [u8; 3],
    pub x: i32,
    pub y: i32,
}

impl Mouse {
    pub fn new() -> Self {
        Self { data_port: Port::new(0x60), status_port: Port::new(0x64), cycle: 0, packet: [0; 3], x: 400, y: 300 }
    }

    pub fn init(&mut self) {
        unsafe {
            let wait = || { for _ in 0..100000 { if (Port::<u8>::new(0x64).read() & 2) == 0 { break; } } };
            wait(); Port::new(0x64).write(0xA8u8); 
            wait(); Port::new(0x64).write(0x20u8); 
            wait(); let status: u8 = Port::new(0x60).read() | 2; 
            wait(); Port::new(0x64).write(0x60u8); 
            wait(); Port::new(0x60).write(status);
            wait(); Port::new(0x64).write(0xD4u8); 
            wait(); Port::new(0x60).write(0xF4u8); 
            wait(); let _ack: u8 = Port::new(0x60).read();
        }
    }

    pub fn process_interrupt(&mut self) -> Option<(i32, i32)> {
        let byte: u8 = unsafe { self.data_port.read() };
        if self.cycle == 0 && (byte & 0x08) == 0 { return None; }
        self.packet[self.cycle as usize] = byte;
        self.cycle = (self.cycle + 1) % 3;

        if self.cycle == 0 {
            let status = self.packet[0];
            let dx = self.packet[1] as i32 - if (status & 0x10) != 0 { 256 } else { 0 };
            let dy = self.packet[2] as i32 - if (status & 0x20) != 0 { 256 } else { 0 };
            
            self.x += dx;
            self.y -= dy;

            if self.x < 0 { self.x = 0; }
            if self.x > 795 { self.x = 795; }
            if self.y < 0 { self.y = 0; }
            if self.y > 595 { self.y = 595; }

            return Some((self.x, self.y));
        }
        None
    }
}