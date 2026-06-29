use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use lazy_static::lazy_static;

lazy_static! {
    // GDT（グローバル・ディスクリプタ・テーブル）の本体
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        // カーネル用の「コード」と「データ」の領域を定義
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        (gdt, Selectors { code_selector, data_selector })
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    data_selector: SegmentSelector,
}

/// GDTをCPUに読み込ませて有効化する関数
pub fn init() {
    use x86_64::instructions::segmentation::{CS, Segment};
    
    // GDTをロード
    GDT.0.load();
    unsafe {
        // コードセグメントレジスタ（CS）を新しいものに更新
        CS::set_reg(GDT.1.code_selector);
    }
}