mod builder;
mod error;
mod format;
mod lookup;
mod metadata;

use std::io::{self, BufRead, Write};

use error::DictError;

fn run_build(input: &str, output: &str) -> i32 {
    match builder::build(input, output) {
        Ok(count) => {
            println!("Built dictionary with {count} entries → {output}");
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

fn run_lookup(dict: &str) -> i32 {
    let mut dictionary = match metadata::DictionaryMetadata::open(dict) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error opening dictionary: {e}");
            return 1;
        }
    };

    let stdin = io::stdin();
    println!("Dictionary loaded. Type a word to look it up, or 'quit' to exit.");
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut word = String::new();
        if stdin.lock().read_line(&mut word).is_err() || word.is_empty() {
            break;
        }

        let word = word.trim();
        if word.eq_ignore_ascii_case("quit") || word.eq_ignore_ascii_case("exit") {
            break;
        }
        if word.is_empty() {
            continue;
        }

        match dictionary.lookup(word) {
            Ok(meaning) => println!("{meaning}"),
            Err(DictError::WordNotFound(_)) => println!("Word '{word}' not found."),
            Err(e) => eprintln!("Error: {e}"),
        }
    }
    0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let exit_code = match args.get(1).map(String::as_str) {
        Some("build") if args.len() == 4 => run_build(&args[2], &args[3]),
        Some("lookup") if args.len() == 3 => run_lookup(&args[2]),
        Some("build") | Some("lookup") => {
            eprintln!("Usage:\n  dict build <input.txt> <output.dat>\n  dict lookup <dict.dat>");
            1
        }
        _ => {
            eprintln!("Usage:\n  dict build <input.txt> <output.dat>\n  dict lookup <dict.dat>");
            1
        }
    };
    std::process::exit(exit_code);
}
