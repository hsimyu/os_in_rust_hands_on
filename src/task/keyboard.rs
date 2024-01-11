use core::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::println;

use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::{task::AtomicWaker, Stream};

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

/// キーボード割り込みハンドラから呼ばれる
/// スキャンコードを内部キューに追加します。
/// 割り込み中に実行されるため、この関数は処理をブロックしたり、アロケートしたりしてはいけない
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!("WARNING: scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake();
        }
    } else {
        println!("WARNING: scancode queue uninitialized");
    }
}

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let queue = SCANCODE_QUEUE.try_get().expect("not initialized");

        if let Ok(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        // 取り出せなかったので待つ
        WAKER.register(&cx.waker());

        match queue.pop() {
            Ok(scannode) => {
                // 登録解除
                WAKER.take();
                Poll::Ready(Some(scannode))
            }
            Err(crossbeam_queue::PopError) => Poll::Pending,
        }
    }
}
