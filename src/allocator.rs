pub mod bump;
pub mod fixed_size_block;
pub mod linked_list;

use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

use self::fixed_size_block::FixedSizeBlockAllocator;

// Trait 実装用のラッパー
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}

pub const HEAP_START: usize = 0x_4444_4444_0000; // 適当な仮想アドレス
pub const HEAP_SIZE: usize = 100 * 1024;

fn align_up(addr: usize, align: usize) -> usize {
    // align - 1 は align を満たす bit よりも下位がすべて 1 である数値
    // つまり、!(align - 1) は align を満たす bit より上位がすべて 1 である数値
    //
    // addr & !(align - 1) で、addr 以下で最も近いアラインメントを満たす下向きのアラインができる
    // 予め対象を (addr + (align - 1)) と加算しておくことで、1アラインメント分ズラした上で計算する
    // -> 上向きのアラインを得る
    (addr + align - 1) & !(align - 1)
}

#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::new());

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        // NOTE: heap_end をヒープの最後の有効なバイトのアドレスにしたいので、-1 する
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    // 作成したページ範囲内にフレームをマッピングする
    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;

        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        // NOTE: flush() することで TLB を明示的に更新する
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    // アロケータを指定した仮想アドレス範囲で初期化する
    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}
