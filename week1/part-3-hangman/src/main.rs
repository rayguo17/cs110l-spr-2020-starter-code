// Simple Hangman Program
// User gets five incorrect guesses
// Word chosen randomly from words.txt
// Inspiration from: https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html
// This assignment will introduce you to some fundamental syntax in Rust:
// - variable declaration
// - string manipulation
// - conditional statements
// - loops
// - vectors
// - files
// - user input
// We've tried to limit/hide Rust's quirks since we'll discuss those details
// more in depth in the coming lectures.
extern crate rand;
use rand::Rng;
use std::collections::HashMap;
use std::fs;
use std::hash::Hash;
use std::io;
use std::io::Write;
use std::ops::Index;

const NUM_INCORRECT_GUESSES: u32 = 5;
const WORDS_PATH: &str = "words.txt";

fn pick_a_random_word() -> String {
    let file_string = fs::read_to_string(WORDS_PATH).expect("Unable to read file.");
    let words: Vec<&str> = file_string.split('\n').collect();
    String::from(words[rand::thread_rng().gen_range(0, words.len())].trim())
}

fn main() {
    let secret_word = pick_a_random_word();
    // Note: given what you know about Rust so far, it's easier to pull characters out of a
    // vector than it is to pull them out of a string. You can get the ith character of
    // secret_word by doing secret_word_chars[i].
    let secret_word_chars: Vec<char> = secret_word.chars().collect();
    // Uncomment for debugging:
    println!("random word: {}", secret_word);
    let mut secret_map = HashMap::new();
    for i in secret_word_chars.iter() {
        *secret_map.entry(i).or_insert(0) += 1;
    }
    let mut cur_map: HashMap<&char, i32> = HashMap::new();

    // Your code here! :)
    let mut chance = 5;
    let mut guess_word: Vec<char> = Vec::new();
    let mut correct_num = 0;
    while !(correct_num == secret_word_chars.len()) && chance > 0 {
        //build the string first,
        let mut sdot: String = String::new();

        let mut sjoin = String::new();
        for i in guess_word.iter() {
            sjoin.push(i.clone());
        }
        concat_word(&cur_map, &secret_word_chars, &mut sdot);
        println!("The word so far is {}", sdot);
        println!("You have guessed the following letters: {}", sjoin);
        println!("You have {} guesses left", chance);
        print!("Please guess a letter: ");
        std::io::stdout().flush().unwrap();
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer);
        println!("");
        //println!("buffer: {}", buffer);
        let c_vec: Vec<char> = buffer.chars().collect();
        let c = c_vec[0].clone();
        if is_required(&mut cur_map, &mut secret_map, &c) {
            correct_num += 1;
            for i in secret_word_chars.iter() {
                if *i == c {
                    *cur_map.entry(i).or_insert(0) += 1;
                    break;
                }
            }
        } else {
            chance -= 1;
            println!("Sorry, that letter is not in the word")
        }
    }
    if correct_num == secret_word_chars.len() {
        println!(
            "Congratulations you guessed the secret word: {}!",
            secret_word
        );
    } else {
        println!("Sorry, you ran out of guesses!");
    }
}

fn concat_word(cur_map: &HashMap<&char, i32>, word_vec: &Vec<char>, res: &mut String) {
    let mut tmp_map = cur_map.clone();
    for i in word_vec.iter() {
        println!("i: {}", i);
        print_map(&tmp_map);
        if tmp_map.contains_key(i) && *tmp_map.get(i).unwrap() > 0 {
            res.push(*i);
            *tmp_map.get_mut(i).unwrap() -= 1;
        } else {
            res.push('-');
        }
    }
}
fn print_map(m: &HashMap<&char, i32>) {
    for (key, value) in m.iter() {
        println!("{}, {}", key, value);
    }
}

fn is_required(
    cur_map: &mut HashMap<&char, i32>,
    ans_map: &mut HashMap<&char, i32>,
    c: &char,
) -> bool {
    if !ans_map.contains_key(c) {
        return false;
    } else {
        if !cur_map.contains_key(c) {
            return true;
        } else {
            let ans_entry = ans_map.get(c).unwrap();
            let cur_entry = cur_map.get(c).unwrap();
            if ans_entry > cur_entry {
                true
            } else {
                false
            }
        }
    }
}
