# NoSQL KV Store

A minimal key-value database built in Rust, implementing the core of an LSM-tree storage engine.

## Architecture

```
Write path:   PUT ──► WAL (disk) ──► MemTable (RAM) ──► [full?] ──► SSTable (disk)
Read path:    GET ──► MemTable ──► SSTable[newest…oldest]
```

### Components

| Component | File | Role |
|---|---|---|
| **WAL** | `wal.rs` | Append-only write-ahead log. Every write is persisted here first so data survives crashes. Each entry has a CRC32 checksum. |
| **MemTable** | `memtable.rs` | In-memory `BTreeMap`. Keeps keys sorted (required for SSTable flush). Writes land here after the WAL. |
| **SSTable** | `sstable.rs` | Immutable on-disk file produced when MemTable hits capacity. Contains a data section, a full index (key → byte offset), and a bloom filter. |
| **Bloom Filter** | `bloom.rs` | Per-SSTable probabilistic filter. Eliminates disk reads for keys that definitely don't exist in that file. |
| **Engine** | `engine.rs` | Coordinates all layers. Handles WAL recovery on startup, triggers flushes, and queries layers in the right order. |

### SSTable file layout

```
┌─────────────────────────────┐
│  Data section               │  key_len(u32) | key | value_len(u32) | value
│  (one entry per record,     │
│   sorted by key)            │
├─────────────────────────────┤  ← index_offset (stored in footer)
│  Index section              │  key_len(u32) | key | byte_offset(u64)
│  (one entry per key)        │
├─────────────────────────────┤  ← bloom_offset (stored in footer)
│  Bloom filter               │  num_bits(u32) | num_hashes(u32) | bit array
├─────────────────────────────┤  ← file_size - 20
│  Footer (20 bytes)          │  index_offset(u64) | bloom_offset(u64) | num_entries(u32)
└─────────────────────────────┘
```

### Read path detail

```
GET "user_007"
  1. Check MemTable  → found? return it
  2. For each SSTable (newest → oldest):
       a. Bloom filter says NO?  → skip file entirely (no disk read)
       b. Index lookup → byte offset
       c. Seek to offset, read value
       d. Value == TOMBSTONE?  → key was deleted, return None
  3. Not found anywhere → return None
```

---

## Build

```bash
git clone <repo>
cd NoSQLEngine
cargo build --release
```

Binary location: `./target/release/NoSQLEngine`

---

## CLI Usage

### Start the REPL

```bash
cargo run
```

With custom options:

```bash
cargo run -- --data-dir /tmp/mydb --capacity 20
```

| Flag | Default | Description |
|---|---|---|
| `--data-dir <path>` | `./data` | Directory where WAL and SSTable files are stored |
| `--capacity <n>` | `20` | Number of records in MemTable before flushing to disk |

---

### Commands

#### `put <id> <name> <phone> <address>`

Insert or update a user. Wrap multi-word values in quotes.

```
kv> put user_001 "Alice Smith" 555-0001 "42 Elm Street, New York"
OK — stored user_001

kv> put user_002 Bob 555-0002 "7 Oak Ave"
OK — stored user_002
```

#### `get <id>`

Look up a user by id.

```
kv> get user_001
  id      : user_001
  name    : Alice Smith
  phone   : 555-0001
  address : 42 Elm Street, New York

kv> get user_999
Not found: user_999
```

#### `delete <id>`

Delete a user (writes a tombstone — the key is logically removed).

```
kv> delete user_001
OK — deleted user_001

kv> get user_001
Not found: user_001
```

#### `list [prefix]`

List all live keys across MemTable and all SSTables. Pass an optional prefix to filter.

```
kv> list
  user_001
  user_002
  user_003
  — 3 record(s)

kv> list user_00
  user_001
  user_002
  — 2 record(s)
```

#### `flush`

Force the current MemTable to be written to a new SSTable immediately, without waiting for it to fill up. Useful for inspecting on-disk files during development.

```
kv> flush
[Engine] Flushing 3 records → ./data/sst_0000.sst

kv> flush
[Engine] MemTable is empty — nothing to flush
```

#### `stats`

Show current MemTable fill level and number of SSTable files on disk.

```
kv> stats
[Stats] MemTable: 3/20 records | SSTables on disk: 2
```

#### `help`

Print command reference.

#### `exit` / `quit` / `q`

Exit the REPL.

---

## Example session

```
$ cargo run -- --capacity 5

NoSQL KV Store
  data dir : data
  capacity : 5 records per MemTable
  type 'help' for commands

kv> put u1 "Alice Smith" 555-0001 "10 Main St"
OK — stored u1
kv> put u2 "Bob Jones" 555-0002 "20 Oak Ave"
OK — stored u2
kv> put u3 "Carol White" 555-0003 "30 Pine Rd"
OK — stored u3
kv> put u4 "Dave Brown" 555-0004 "40 Birch Ln"
OK — stored u4
kv> put u5 "Eve Black" 555-0005 "50 Cedar Dr"
[Engine] Flushing 5 records → data/sst_0000.sst
OK — stored u5
kv> stats
[Stats] MemTable: 0/5 records | SSTables on disk: 1
kv> get u3
  id      : u3
  name    : Carol White
  phone   : 555-0003
  address : 30 Pine Rd
kv> list
  u1
  u2
  u3
  u4
  u5
  — 5 record(s)
kv> flush
[Engine] Flushing 5 records → data/sst_0000.sst
kv> stats
[Stats] MemTable: 0/5 records | SSTables on disk: 1
kv> delete u3
OK — deleted u3
kv> get u3
Not found: u3
kv> list
  u1
  u2
  u4
  u5
  — 4 record(s)
kv> exit
Bye!
```

---

## Data files

After running, the `./data` directory contains:

```
data/
  wal.log        — active write-ahead log (truncated after each SSTable flush)
  sst_0000.sst   — first SSTable (immutable)
  sst_0001.sst   — second SSTable (immutable)
  ...
```

On restart the engine replays `wal.log` to recover any writes that had not yet been flushed to an SSTable.

---

## Limitations (by design — this is a learning project)

- No compaction: SSTable files accumulate and are never merged
- No concurrent access: single-threaded only
- Keys and values are always loaded fully into memory during reads
- No range scans: point lookups only
