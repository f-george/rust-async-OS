use std::{
    pin::Pin,
    task::{Context, Poll},
};

use conquer_once::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::{Stream, StreamExt, task::AtomicWaker};
use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1, layouts};

// Wake is used to handle futures. You can notify an executor to poll a future
// using a wake when it is required.

/// Cells are shareable mutable containers
/// If you want mutation across multiple threads use mutex.
///
/// OnceCell is a cell that can be written to only once
///
/// Array queue is a bounded mpmc queue
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

static WAKER: AtomicWaker = AtomicWaker::new();

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

// If stream is empty, poll infinitely unless a waker is used

// Extract waker from context -> Store reference -> invoke wake method

// impl Stream for ScancodeStream {
//     type Item = u8;

//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
//         let queue = SCANCODE_QUEUE.try_get().expect("not initialized");
//         match queue.pop() {
//             Some(scancode) => Poll::Ready(Some(scancode)),
//             None => Poll::Pending,
//         }
//     }
// }

/// Use waker
impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue not initialized");

        // Interrupt handler fills queue im mediately after check
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        // Pause polling
        WAKER.register(&cx.waker());
        match queue.pop() {
            Some(scancode) => {
                // input is ready to be taken from queue.
                // Pop and use waker to activate
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            // Nothing to pop, poll once
            None => Poll::Pending,
        }
    }
}

// NOTE: Executor still polls Scancode stream which is poor performance

// Polls until Poll::Ready(None) signalling the stream is complete.
// pub trait Stream {
//     type Item;

//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>>;
// }

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => print!("{}", character),
                    DecodedKey::RawKey(key) => print!("{:?}", key),
                }
            }
        }
    }
}
