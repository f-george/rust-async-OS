//!
//! Async tutorial entry point
//!

use task::{self, Task, executor::SimpleExecutor, keyboard};

// #![allow(dead_code)]

async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}

fn main() {
    let mut executor = SimpleExecutor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}
