use std::collections::VecDeque;
#[allow(unused_imports)]
use std::sync::{Arc, Mutex};
use std::time::Instant;
#[allow(unused_imports)]
use std::{env, process, thread};

/// Determines whether a number is prime. This function is taken from CS 110 factor.py.
///
/// You don't need to read or understand this code.
#[allow(dead_code)]
fn is_prime(num: u32) -> bool {
    if num <= 1 {
        return false;
    }
    for factor in 2..((num as f64).sqrt().floor() as u32) {
        if num % factor == 0 {
            return false;
        }
    }
    true
}

/// Determines the prime factors of a number and prints them to stdout. This function is taken
/// from CS 110 factor.py.
///
/// You don't need to read or understand this code.
#[allow(dead_code)]
fn factor_number(num: u32) {
    let start = Instant::now();

    if num == 1 || is_prime(num) {
        println!("{} = {} [time: {:?}]", num, num, start.elapsed());
        return;
    }

    let mut factors = Vec::new();
    let mut curr_num = num;
    for factor in 2..num {
        while curr_num % factor == 0 {
            factors.push(factor);
            curr_num /= factor;
        }
    }
    factors.sort();
    let factors_str = factors
        .into_iter()
        .map(|f| f.to_string())
        .collect::<Vec<String>>()
        .join(" * ");
    println!("{} = {} [time: {:?}]", num, factors_str, start.elapsed());
}

/// Returns a list of numbers supplied via argv.
#[allow(dead_code)]
fn get_input_numbers() -> VecDeque<u32> {
    let mut numbers = VecDeque::new();
    for arg in env::args().skip(1) {
        if let Ok(val) = arg.parse::<u32>() {
            numbers.push_back(val);
        } else {
            println!("{} is not a valid number", arg);
            process::exit(1);
        }
    }
    numbers
}

fn main() {
    let num_threads = num_cpus::get();
    println!("Farm starting on {} CPUs", num_threads);
    let start = Instant::now();

    // TODO: call get_input_numbers() and store a queue of numbers to factor
    let que = get_input_numbers();
    //Arc for share ownership between different threads.
    //Mutex for lock when accessing.
    let queue = Arc::new(Mutex::new(que));

    // TODO: spawn `num_threads` threads, each of which pops numbers off the queue and calls
    let mut threads = Vec::new();
    for i in 0..num_threads {
        //create a new handle pointing to exacly the same memory space.
        let queue_handle = queue.clone();
        threads.push(thread::spawn(move || {
            worker(i, queue_handle);
        }))
    }
    // factor_number() until the queue is empty

    // TODO: join all the threads you created
    for handle in threads {
        handle.join().expect("Panic occured in thread!");
    }

    println!("Total execution time: {:?}", start.elapsed());
}

fn worker(i: usize, queue: Arc<Mutex<VecDeque<u32>>>) {
    //println!("Hello world!");
    //a loop to get more item
    loop {
        //a helper function to avoid lock holding when processing data.
        let val = que_helper(&queue);
        match val {
            Some(num) => {
                println!("{}: {}", i, num);
                factor_number(num);
            }
            None => {
                return;
            }
        }
    }
}
fn que_helper(que: &Arc<Mutex<VecDeque<u32>>>) -> Option<u32> {
    let mut que_ref = que.lock().unwrap();
    return que_ref.pop_back();
}
