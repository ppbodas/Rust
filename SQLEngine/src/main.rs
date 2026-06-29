mod config;
mod record;
mod page;
mod pager;
mod btree;
mod engine;

use std::io::{self, BufRead, Write};
use std::time::Instant;
use engine::Engine;
use record::User;

const DB_PATH: &str = "users.db";

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("seed") => cmd_seed(),
        Some("help") | Some("--help") | Some("-h") => { print_help(); Ok(()) }
        _            => cmd_repl(),
    }
}

// ── Seed command ──────────────────────────────────────────────────────────────

fn cmd_seed() -> std::io::Result<()> {
    let _ = std::fs::remove_file(DB_PATH);
    let mut db = Engine::open(DB_PATH)?;

    println!("Seeding 10,000 records into {}...", DB_PATH);
    let start = Instant::now();

    for i in 1u64..=10_000 {
        let user = User::new(
            i,
            &format!("User_{i}"),
            (20 + (i % 60)) as u8,
            &format!("+1-555-{i:04}"),
            &format!("{i} Main St, City {}", i % 100),
        );
        db.insert(&user)?;
    }

    db.close()?;
    println!("Done in {:.2?}. Database: {}", start.elapsed(), DB_PATH);
    Ok(())
}

// ── Interactive REPL ──────────────────────────────────────────────────────────

fn cmd_repl() -> std::io::Result<()> {
    let mut db = Engine::open(DB_PATH)?;
    println!("SQLEngine — database: {}", DB_PATH);
    println!("Type HELP for available commands.\n");

    let stdin = io::stdin();
    loop {
        print!("sql> ");
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break; // EOF
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // splitn(6) so the last token (address) captures everything including spaces
        let tokens: Vec<&str> = line.splitn(6, ' ').collect();
        let cmd = tokens[0].to_uppercase();

        match cmd.as_str() {
            "INSERT" => handle_insert(&mut db, &tokens),
            "UPDATE" => handle_update(&mut db, &tokens),
            "DELETE" => handle_delete(&mut db, &tokens),
            "FIND"   => handle_find(&mut db, &tokens),
            "RANGE"  => handle_range(&mut db, &tokens),
            "COUNT"  => handle_count(&mut db),
            "HELP"   => print_help(),
            "EXIT" | "QUIT" => {
                db.close()?;
                println!("Bye.");
                break;
            }
            _ => println!("Unknown command '{}'. Type HELP.", tokens[0]),
        }
    }

    Ok(())
}

// ── Command handlers ──────────────────────────────────────────────────────────

fn handle_insert(db: &mut Engine, tokens: &[&str]) {
    // INSERT <id> <name> <age> <phone> <address>
    if tokens.len() < 6 {
        println!("Usage: INSERT <id> <name> <age> <phone> <address>");
        println!("  e.g: INSERT 1 Alice 30 +1-555-0001 \"123 Main St\"");
        return;
    }

    let id = match tokens[1].parse::<u64>() {
        Ok(v) => v,
        Err(_) => { println!("Error: id must be a positive integer."); return; }
    };
    let name    = tokens[2].trim_matches('"');
    let age = match tokens[3].parse::<u8>() {
        Ok(v) => v,
        Err(_) => { println!("Error: age must be 0–255."); return; }
    };
    let phone   = tokens[4].trim_matches('"');
    let address = tokens[5].trim_matches('"');

    if name.len() > 32 {
        println!("Error: name too long (max 32 chars).");
        return;
    }
    if phone.len() > 16 {
        println!("Error: phone too long (max 16 chars).");
        return;
    }
    if address.len() > 63 {
        println!("Error: address too long (max 63 chars).");
        return;
    }

    let user = User::new(id, name, age, phone, address);
    let start = Instant::now();
    match db.insert_unique(&user) {
        Ok(Ok(_))    => println!("Inserted id={} in {:.2?}", id, start.elapsed()),
        Ok(Err(msg)) => println!("Error: {msg}"),
        Err(e)       => println!("Error: {e}"),
    }
}

fn handle_delete(db: &mut Engine, tokens: &[&str]) {
    // DELETE <id>
    if tokens.len() < 2 {
        println!("Usage: DELETE <id>");
        return;
    }
    let id = match tokens[1].parse::<u64>() {
        Ok(v) => v,
        Err(_) => { println!("Error: id must be a positive integer."); return; }
    };
    let start = Instant::now();
    match db.delete(id, true) {
        Ok(true)  => println!("Deleted id={} in {:.2?}", id, start.elapsed()),
        Ok(false) => println!("No record with id={}", id),
        Err(e)    => println!("Error: {e}"),
    }
}

fn handle_update(db: &mut Engine, tokens: &[&str]) {
    // UPDATE <id> <name> <age> <phone> <address>
    if tokens.len() < 6 {
        println!("Usage: UPDATE <id> <name> <age> <phone> <address>");
        println!("  Updates name/age/phone/address for an existing id.");
        println!("  To change the id itself, delete and re-insert.");
        return;
    }
    let id = match tokens[1].parse::<u64>() {
        Ok(v) => v,
        Err(_) => { println!("Error: id must be a positive integer."); return; }
    };
    let name    = tokens[2].trim_matches('"');
    let age = match tokens[3].parse::<u8>() {
        Ok(v) => v,
        Err(_) => { println!("Error: age must be 0–255."); return; }
    };
    let phone   = tokens[4].trim_matches('"');
    let address = tokens[5].trim_matches('"');

    if name.len() > 32    { println!("Error: name too long (max 32 chars)."); return; }
    if phone.len() > 16   { println!("Error: phone too long (max 16 chars)."); return; }
    if address.len() > 63 { println!("Error: address too long (max 63 chars)."); return; }

    let user = User::new(id, name, age, phone, address);
    let start = Instant::now();
    match db.update(&user, true) {
        Ok(true)  => println!("Updated id={} in {:.2?}", id, start.elapsed()),
        Ok(false) => println!("No record with id={}. Use INSERT to add it.", id),
        Err(e)    => println!("Error: {e}"),
    }
}

fn handle_find(db: &mut Engine, tokens: &[&str]) {
    // FIND <id>
    if tokens.len() < 2 {
        println!("Usage: FIND <id>");
        println!("  e.g: FIND 42");
        return;
    }
    let id = match tokens[1].parse::<u64>() {
        Ok(v) => v,
        Err(_) => { println!("Error: id must be a positive integer."); return; }
    };

    let start = Instant::now();
    match db.find_by_id(id, true) {
        Ok(Some(u)) => {
            println!("Found in {:.2?}:", start.elapsed());
            print_user(&u);
        }
        Ok(None)    => println!("No record with id={}", id),
        Err(e)      => println!("Error: {e}"),
    }
}

fn handle_range(db: &mut Engine, tokens: &[&str]) {
    // RANGE <start_id> <end_id>
    if tokens.len() < 3 {
        println!("Usage: RANGE <start_id> <end_id>");
        println!("  e.g: RANGE 100 110");
        return;
    }
    let start_id = match tokens[1].parse::<u64>() {
        Ok(v) => v,
        Err(_) => { println!("Error: start_id must be a positive integer."); return; }
    };
    let end_id = match tokens[2].parse::<u64>() {
        Ok(v) => v,
        Err(_) => { println!("Error: end_id must be a positive integer."); return; }
    };
    if start_id > end_id {
        println!("Error: start_id must be <= end_id.");
        return;
    }

    let start = Instant::now();
    match db.range_query(start_id, end_id, true) {
        Ok(results) => {
            println!("{} record(s) found in {:.2?}:", results.len(), start.elapsed());
            for u in &results {
                print_user(u);
            }
        }
        Err(e) => println!("Error: {e}"),
    }
}

fn handle_count(db: &mut Engine) {
    let start = Instant::now();
    match db.range_query(0, u64::MAX, false) {
        Ok(results) => println!("Total records: {} (scanned in {:.2?})", results.len(), start.elapsed()),
        Err(e)      => println!("Error: {e}"),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn print_user(u: &record::User) {
    println!(
        "  id={:<6} name={:<15} age={:<4} phone={:<15} address={}",
        u.id, u.name, u.age, u.phone, u.address
    );
}

fn print_help() {
    println!("Commands:");
    println!("  INSERT <id> <name> <age> <phone> <address>   Insert a user (error if id exists)");
    println!("  UPDATE <id> <name> <age> <phone> <address>   Update non-key fields in-place");
    println!("  DELETE <id>                                   Delete a record by id");
    println!("  FIND   <id>                                   Lookup by id");
    println!("  RANGE  <start_id> <end_id>                   Fetch all ids in range");
    println!("  COUNT                                         Count all records");
    println!("  HELP                                          Show this message");
    println!("  EXIT                                          Close and quit");
    println!();
    println!("Field limits:  name ≤ 32 chars  |  phone ≤ 16 chars  |  address ≤ 63 chars");
    println!("Database file: {}", DB_PATH);
}
