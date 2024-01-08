use core::mem;

use crate::allocator::align_up;

struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct LinkedListAllocator {
    head: ListNode,
}

impl LinkedListAllocator {
    /// 空のアロケータを新規作成
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0),
        }
    }

    /// 与えられたヒープ境界でアロケータを初期化する。
    ///
    /// ヒープ領域が未使用であることは呼び出し元が保証しなければならない。
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_region(heap_start, heap_size);
    }

    /// 与えられたメモリ領域をフリーリストの先頭に追加する
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
        assert!(size >= mem::size_of::<ListNode>());

        let mut node = ListNode::new(size);
        node.next = self.head.next.take();

        let node_ptr = addr as *mut ListNode;
        node_ptr.write(node);
        self.head.next = Some(&mut *node_ptr);
    }

    /// 与えられたサイズのフリー領域を探し、リストからそれを取り除く
    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
        let mut current = &mut self.head;

        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
                // 割り当てできたのでフリーリストから除く
                let next = region.next.take();
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            } else {
                // 割り当てに失敗したので次の領域を調査
                current = current.next.as_mut().unwrap();
            }
        }

        // 適した領域が見つからなかった
        None
    }

    fn alloc_from_region(region: &ListNode, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // 領域が小さすぎる
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            // 領域の残りが小さすぎて新しい ListNode を格納できない
            return Err(());
        }

        Ok(alloc_start)
    }
}
