use x86_64::registers::control::Cr3;
use x86_64::{structures::paging::PageTable, PhysAddr, VirtAddr};

/// 有効なレベル4テーブルへの可変な参照を返す。
///
/// この関数は、全物理メモリが、physical_memory_offset から始まる仮想アドレス空間上に
/// 完全にマップされていることを前提としている。
/// また &mut 参照が複数の名称を持ってしまう可能性があるため、この関数は一度しか呼び出してはならない。
pub unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();

    // 仮想アドレス空間の一部に全物理メモリがマップされていることを前提として、
    // 物理アドレスオフセットから仮想アドレスへ変換する
    let virt = physical_memory_offset + phys.as_u64();

    // 仮想アドレスを通してページテーブル自体のポインタを生成
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

/// 与えられた仮想アドレスを対応する物理アドレスに変換します。
/// 指定されたアドレスがマップされていないなら `None` を返します。
///
/// この関数は、全物理メモリが、physical_memory_offset から始まる仮想アドレス空間上に
/// 完全にマップされていることを前提としている。
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    translate_addr_inner(addr, physical_memory_offset)
}

fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    use x86_64::structures::paging::page_table::FrameError;
    let (level_4_table_frame, _) = Cr3::read();

    let table_indexes = [
        addr.p4_index(),
        addr.p3_index(),
        addr.p2_index(),
        addr.p1_index(),
    ];
    let mut frame = level_4_table_frame;

    // 複数層のページテーブルを辿る
    for &index in &table_indexes {
        // フレームをページテーブルの参照に変換
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe { &*table_ptr };

        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge pages not supported"),
        };
    }

    // この時点で frame は L1 から取得したフレームを指している
    // フレームの開始アドレスにページオフセットを足すことで、
    // 目的の物理アドレスを計算する
    Some(frame.start_address() + u64::from(addr.page_offset()))
}
