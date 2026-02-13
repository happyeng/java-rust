mod topo_runner;
mod util;
mod verifier;
use crate::topo_runner::TopoRunner;
use std::time::Instant;
#[macro_use]
extern crate lazy_static;
use std::sync::atomic::{AtomicUsize, Ordering};
lazy_static! {
    pub static ref EXIST_COUNT: AtomicUsize = AtomicUsize::new(0);
    pub static ref NONEXIST_COUNT: AtomicUsize = AtomicUsize::new(0);
}

fn main() {
    let mut topo_runner: TopoRunner = TopoRunner::new(128);
    let filedir = "../data/fattree/fattree10";
    let start: Instant = Instant::now();
    topo_runner.set_file_dir(filedir);
    topo_runner.build();
    topo_runner.verify();
    let duration: std::time::Duration = start.elapsed();
    println!("Total program execution time: {:?}", duration);
    println!(
        "Reachable node pair count:  {}",
        EXIST_COUNT.load(Ordering::SeqCst)
    );
    println!(
        "Unreachable node pair count:  {}",
        NONEXIST_COUNT.load(Ordering::SeqCst)
    );
    println!(
        "Total node pair count:  {}",
        EXIST_COUNT.load(Ordering::SeqCst) + NONEXIST_COUNT.load(Ordering::SeqCst)
    );
}
