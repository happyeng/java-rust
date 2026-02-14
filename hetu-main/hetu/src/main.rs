mod simulator;
mod util;
mod verifier;
use crate::simulator::Simulator;
use mimalloc::MiMalloc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
#[macro_use]
extern crate lazy_static;
lazy_static! {
    pub static ref EXIST_COUNT: AtomicUsize = AtomicUsize::new(0);
    pub static ref NONEXIST_COUNT: AtomicUsize = AtomicUsize::new(0);
    pub static ref TRAVERSAL_COUNT: AtomicUsize = AtomicUsize::new(0);
}
fn main() {
    println!("Starting the application");
    let filedir = "../data/fattree/fattree10";
    let mut simulator: Simulator = Simulator::new(144);
    let start: Instant = Instant::now();
    simulator.set_file_dir(filedir);
    simulator.build();
    let duration = start.elapsed();
    println!("Build time: {:?}", duration);
    simulator.verify_reachability_with_npnet();
    let duration: std::time::Duration = start.elapsed();
    println!("Total execution time: {:?}", duration);
    println!(
        "Reachable node pair count: {}",
        EXIST_COUNT.load(Ordering::SeqCst)
    );
    println!(
        "Unreachable node pair count: {}",
        NONEXIST_COUNT.load(Ordering::SeqCst)
    );
    println!(
        "Total node pair count: {}",
        EXIST_COUNT.load(Ordering::SeqCst) + NONEXIST_COUNT.load(Ordering::SeqCst)
    );
}
