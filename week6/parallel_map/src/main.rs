use crossbeam_channel::{self, Receiver};
use std::{thread, time};

struct Item<T> {
    val: T,
    pos: usize,
}

struct Res<T> {
    val: T,
    pos: usize,
}

fn parallel_map<T, U, F>(mut input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + 'static,
    U: Send + 'static + Default,
{
    let mut output_vec: Vec<U> = Vec::with_capacity(input_vec.len());
    // TODO: implement parallel map!
    let (input_sender, input_receiver) = crossbeam_channel::unbounded();
    let (output_sender, output_receiver) = crossbeam_channel::unbounded();
    let mut threads = Vec::new();
    for _ in 0..num_threads {
        let receiver: Receiver<Item<T>> = input_receiver.clone();
        let sender = output_sender.clone();
        threads.push(thread::spawn(move || {
            while let Ok(next_item) = receiver.recv() {
                let res = f(next_item.val);
                sender
                    .send(Item {
                        val: res,
                        pos: next_item.pos,
                    })
                    .unwrap();
            }
        }))
    }
    let reserved = input_vec.len();
    for i in 0..reserved {
        output_vec.push(Default::default());
        input_sender
            .send(Item {
                val: input_vec.pop().unwrap(),
                pos: input_vec.len(),
            })
            .expect("msg");
    }
    drop(input_sender);
    let mut cnt = 0;
    while let Ok(next_res) = output_receiver.recv() {
        output_vec[next_res.pos] = next_res.val;
        cnt += 1;
        if cnt == reserved {
            break;
        }
    }
    for thread in threads {
        thread.join().expect("Panic ocurred in thread");
    }
    output_vec
}

fn storer_worker() {}

fn main() {
    let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
    let squares = parallel_map(v, 10, |num| {
        println!("{} squared is {}", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });
    println!("squares: {:?}", squares);
}
