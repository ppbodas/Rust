mod bloom;
mod config;
mod engine;
mod memtable;
mod sstable;
mod user;
mod wal;

use config::DbConfig;
use engine::Engine;
use std::io::{self, Write};
use std::path::PathBuf;
use user::User;

fn main() {
    let (data_dir, capacity) = parse_args();

    let config = DbConfig {
        memtable_capacity: capacity,
        data_dir: data_dir.clone(),
    };

    println!("NoSQL KV Store");
    println!("  data dir : {}", data_dir.display());
    println!("  capacity : {} records per MemTable", capacity);
    println!("  type 'help' for commands\n");

    let mut db = match Engine::open(config) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to open database: {}", e);
            std::process::exit(1);
        }
    };

    let stdin = io::stdin();
    loop {
        print!("kv> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        }

        let tokens = tokenize(line.trim());
        if tokens.is_empty() {
            continue;
        }

        match tokens[0].as_str() {
            "put" => cmd_put(&mut db, &tokens),
            "get" => cmd_get(&mut db, &tokens),
            "delete" | "del" => cmd_delete(&mut db, &tokens),
            "flush" => match db.flush() {
                Ok(_) => {}
                Err(e) => eprintln!("Error: {}", e),
            },
            "seed" => cmd_seed(&mut db, &tokens),
            "purge" => cmd_purge(&mut db),
            "list" => cmd_list(&mut db, &tokens),
            "stats" => db.stats(),
            "help" => print_help(),
            "exit" | "quit" | "q" => {
                println!("Bye!");
                break;
            }
            other => eprintln!("Unknown command '{}'. Type 'help' for commands.", other),
        }
    }
}

// seed <n>
fn cmd_seed(db: &mut Engine, tokens: &[String]) {
    let n: usize = match tokens.get(1).and_then(|s| s.parse().ok()) {
        Some(n) if n > 0 => n,
        _ => {
            eprintln!("Usage: seed <n>  (n must be a positive integer)");
            return;
        }
    };

    let names = ["Alice", "Bob", "Carol", "Dave", "Eve", "Frank", "Grace", "Hank"];
    let streets = ["Main St", "Oak Ave", "Pine Rd", "Birch Ln", "Cedar Dr", "Elm Blvd"];

    let mut ok = 0;
    for i in 1..=n {
        let name = format!("{} User{}", names[(i - 1) % names.len()], i);
        let phone = format!("555-{:04}", i);
        let address = format!("{} {}", i * 10, streets[(i - 1) % streets.len()]);
        let key = format!("user_{:04}", i);

        println!(
            "[SEED #{:>4}] id={} | name={} | phone={} | address={}",
            i, key, name, phone, address
        );

        let user = User::new(&key, &name, &phone, &address);
        match db.put(key, user.to_bytes()) {
            Ok(_) => ok += 1,
            Err(e) => {
                eprintln!("Error at record {}: {}", i, e);
                break;
            }
        }
    }

    println!("Seeded {} record(s)", ok);
}

// purge — wipes all SSTables, MemTable, and WAL
fn cmd_purge(db: &mut Engine) {
    match db.purge() {
        Ok(_) => {}
        Err(e) => eprintln!("Error: {}", e),
    }
}

// put <id> <name> <phone> <address>
fn cmd_put(db: &mut Engine, tokens: &[String]) {
    if tokens.len() != 5 {
        eprintln!("Usage: put <id> <name> <phone> <address>");
        eprintln!("  Wrap multi-word values in quotes: put u1 \"John Doe\" 555-0001 \"1 Main St\"");
        return;
    }
    let user = User::new(&tokens[1], &tokens[2], &tokens[3], &tokens[4]);
    let key = user.id.clone();
    match db.put(key.clone(), user.to_bytes()) {
        Ok(_) => println!("OK — stored {}", key),
        Err(e) => eprintln!("Error: {}", e),
    }
}

// get <id>
fn cmd_get(db: &mut Engine, tokens: &[String]) {
    if tokens.len() != 2 {
        eprintln!("Usage: get <id>");
        return;
    }
    match db.get(&tokens[1]) {
        Ok(Some(bytes)) => {
            let u = User::from_bytes(&bytes);
            println!("  id      : {}", u.id);
            println!("  name    : {}", u.name);
            println!("  phone   : {}", u.phone);
            println!("  address : {}", u.address);
        }
        Ok(None) => println!("Not found: {}", tokens[1]),
        Err(e) => eprintln!("Error: {}", e),
    }
}

// list [prefix]
fn cmd_list(db: &mut Engine, tokens: &[String]) {
    let prefix = tokens.get(1).map(|s| s.as_str()).unwrap_or("");
    match db.list_keys() {
        Ok(keys) => {
            let filtered: Vec<_> = keys.iter().filter(|k| k.starts_with(prefix)).collect();
            if filtered.is_empty() {
                println!("(no records)");
            } else {
                for k in &filtered {
                    println!("  {}", k);
                }
                println!("  — {} record(s)", filtered.len());
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

// delete <id>
fn cmd_delete(db: &mut Engine, tokens: &[String]) {
    if tokens.len() != 2 {
        eprintln!("Usage: delete <id>");
        return;
    }
    match db.delete(tokens[1].clone()) {
        Ok(_) => println!("OK — deleted {}", tokens[1]),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn print_help() {
    println!("Commands:");
    println!("  put <id> <name> <phone> <address>   Insert or update a user");
    println!("  get <id>                             Look up a user by id");
    println!("  delete <id>  (alias: del)            Delete a user");
    println!("  seed <n>                             Insert n generated users (user_0001…user_000n), prints each");
    println!("  purge                                Delete all SSTables, clear MemTable and WAL");
    println!("  list [prefix]                        List all live keys (optional prefix filter)");
    println!("  flush                                Force MemTable → SSTable flush now");
    println!("  stats                                Show MemTable / SSTable counts");
    println!("  help                                 Show this message");
    println!("  exit  (aliases: quit, q)             Exit");
    println!();
    println!("Wrap values that contain spaces in double or single quotes:");
    println!("  put u1 \"Alice Smith\" 555-0001 \"42 Elm Street, NY\"");
}

// Shell-style tokenizer: splits on whitespace, respects "double" and 'single' quotes
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quote: Option<char> = None;

    for ch in input.chars() {
        match (ch, in_quote) {
            ('"' | '\'', None) => in_quote = Some(ch),
            (c, Some(q)) if c == q => in_quote = None,
            (' ' | '\t', None) => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

// Accepts: --data-dir <path>  --capacity <n>
fn parse_args() -> (PathBuf, usize) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut data_dir = PathBuf::from("./data");
    let mut capacity: usize = 20;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--data-dir" if i + 1 < args.len() => {
                data_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--capacity" if i + 1 < args.len() => {
                capacity = args[i + 1].parse().unwrap_or_else(|_| {
                    eprintln!("Invalid capacity '{}', using 20", args[i + 1]);
                    20
                });
                i += 2;
            }
            other => {
                eprintln!("Unknown flag '{}' — ignored", other);
                i += 1;
            }
        }
    }
    (data_dir, capacity)
}
