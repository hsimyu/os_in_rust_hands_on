use x86_64::registers::control::Cr3;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::{structures::paging::PageTable, VirtAddr};

/// 新しい OffsetPageTable を初期化する。
///
/// この関数は、全物理メモリが、physical_memory_offset から始まる仮想アドレス空間上に
/// 完全にマップされていることを前提としている。
/// また &mut 参照が複数の名称を持ってしまう可能性があるため、この関数は一度しか呼び出してはならない。
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// 有効なレベル4テーブルへの可変な参照を返す。
///
/// この関数は、全物理メモリが、physical_memory_offset から始まる仮想アドレス空間上に
/// 完全にマップされていることを前提としている。
/// また &mut 参照が複数の名称を持ってしまう可能性があるため、この関数は一度しか呼び出してはならない。
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();

    // 仮想アドレス空間の一部に全物理メモリがマップされていることを前提として、
    // 物理アドレスオフセットから仮想アドレスへ変換する
    let virt = physical_memory_offset + phys.as_u64();

    // 仮想アドレスを通してページテーブル自体のポインタを生成
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}
