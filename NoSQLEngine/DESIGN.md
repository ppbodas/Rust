# Design Document — NoSQLEngine

This document explains the internal implementation of the three core storage components: MemTable, SSTable, and Bloom Filter.

---

## 1. MemTable (`src/memtable.rs`)

### What it is

The MemTable is the in-memory write buffer. Every `put` and `delete` lands here first (after the WAL). It sits between the caller and the disk — fast to write, fast to read, lost on crash (which is why the WAL exists).

### Data structure: `BTreeMap`

```
MemTable
└── data: BTreeMap<String, Vec<u8>>
```

A `BTreeMap` is chosen over `HashMap` for one reason: **iteration order**. When the MemTable is flushed to an SSTable, entries must be written in sorted key order so the SSTable index is meaningful. `BTreeMap` gives sorted order for free on every `.iter()` call.

### Capacity and flush trigger

```rust
pub fn is_full(&self) -> bool {
    self.data.len() >= self.capacity
}
```

Capacity is set in `DbConfig::memtable_capacity` (default: 20 records). The Engine checks `is_full()` after every `put`. When it returns `true`, the Engine calls `flush()` which passes the MemTable to `SsTableWriter`, then calls `clear()`.

### Deletes as tombstones

Deletes are not real removals. Instead, the key is inserted with a sentinel value:

```rust
pub const TOMBSTONE: &[u8] = b"__TOMBSTONE__";

pub fn delete(&mut self, key: String) {
    self.data.insert(key, TOMBSTONE.to_vec());
}
```

This means a deleted key **still occupies a slot** in the MemTable and will be written to the SSTable on flush. The Engine's `get` path checks for the tombstone and returns `None`:

```rust
if value == TOMBSTONE { return Ok(None); }
```

The tombstone is necessary because a key may exist in an older SSTable. Without writing it to disk, a restart would "undelete" the key.

### State diagram

```
              put(k, v)           is_full() == true
  EMPTY ──────────────► FILLING ──────────────────► FLUSHING ──► EMPTY
                         │  ▲                           │
                         └──┘                           │
                       more puts                    clear() resets
```

---

## 2. SSTable (`src/sstable.rs`)

### What it is

An SSTable (Sorted String Table) is an **immutable, append-only file** written once when the MemTable is full. It is never modified after creation. Multiple SSTables accumulate on disk over time.

### File layout

Every SSTable file has four sections written sequentially:

```
┌──────────────────────────────────────────────────────┐
│  DATA SECTION                                        │
│  ┌────────────┬──────────┬─────────────┬──────────┐  │
│  │ key_len u32│ key bytes│value_len u32│val bytes │  │  ← one block per entry
│  └────────────┴──────────┴─────────────┴──────────┘  │
│  (repeated for every key, in sorted order)           │
├──────────────────────────────────────────────────────┤ ← index_offset
│  INDEX SECTION                                       │
│  ┌────────────┬──────────┬──────────────────┐        │
│  │ key_len u32│ key bytes│ byte_offset u64  │        │  ← one entry per key
│  └────────────┴──────────┴──────────────────┘        │
│  (maps every key to its exact byte offset in data)   │
├──────────────────────────────────────────────────────┤ ← bloom_offset
│  BLOOM FILTER SECTION                                │
│  num_bits u32 | num_hashes u32 | bit array bytes     │
├──────────────────────────────────────────────────────┤ ← file_size - 20
│  FOOTER (always 20 bytes at end of file)             │
│  index_offset u64 | bloom_offset u64 | num_entries u32│
└──────────────────────────────────────────────────────┘
```

All integers are little-endian.

### Write path (`SsTableWriter::flush`)

```
MemTable.iter()   ← sorted by key (BTreeMap guarantee)
    │
    ├── for each (key, value):
    │       ├── bloom.insert(key)
    │       ├── index.push((key, current_offset))
    │       └── write: key_len | key | value_len | value
    │
    ├── write index section
    │       └── for each (key, offset): key_len | key | offset
    │
    ├── write bloom section
    │       └── bloom.to_bytes()
    │
    └── write footer
            └── index_offset | bloom_offset | num_entries
```

The byte offset of each entry is tracked manually with a running `offset: u64` counter, incremented by `4 + key_len + 4 + value_len` per entry.

### Read path (`SsTableReader::open` + `get`)

Opening an SSTable does **not** read the data section. It only loads the footer, index, and bloom filter into memory:

```
open(path)
  1. Seek to end - 20  → read footer (index_offset, bloom_offset, num_entries)
  2. Seek to index_offset → read entire index into BTreeMap<String, u64>
  3. Seek to bloom_offset → read bloom bytes, deserialise BloomFilter
```

A `get(key)` then works in three steps:

```
get("user_007")
  1. bloom.may_contain("user_007")?
       NO  → return None immediately (no disk read)
       YES → continue
  2. index.get("user_007")?
       None → return None (bloom false positive)
       Some(offset) → continue
  3. file.seek(offset)
       read key_len, skip key bytes
       read value_len, read value bytes
       value == TOMBSTONE? → return None
       else → return Some(value)
```

Step 1 (bloom) eliminates most disk reads for absent keys. Step 2 (index) is an O(log n) in-memory `BTreeMap` lookup. Step 3 is a single `seek` + bounded `read` — no scanning.

### Immutability

Once written, an SSTable file is never opened for writing again. The `SsTableWriter` opens the file with `truncate(true)`, writes everything, and closes. `SsTableReader` only ever calls `File::open` (read-only). This makes SSTables safe to read concurrently and trivial to reason about.

### Multiple SSTables and lookup order

The Engine holds a `Vec<SsTableReader>` ordered by creation time (oldest first). Queries scan **newest → oldest** so a more recent value always shadows an older one:

```rust
for sst in self.sstables.iter().rev() { ... }
```

---

## 3. Bloom Filter (`src/bloom.rs`)

### What it is

A Bloom filter answers "is key X definitely NOT in this SSTable?" with zero false negatives but a tunable false positive rate. When `may_contain` returns `false`, no disk read is needed. When it returns `true`, the key is probably present (but the index lookup confirms it).

### Bit array

The filter is a flat bit array stored as `Vec<u8>`. Each bit is independently set by the hash functions. The array length is derived from the desired capacity and false-positive rate:

```
m = ceil( -n * ln(p) / ln(2)² )
```

Where `n` = number of keys, `p` = target false positive rate (1% in this implementation). With `p = 0.01` and `n = 20`, this gives ~192 bits.

### Hash functions — double hashing

Rather than implementing `k` independent hash functions, the filter uses the **double hashing** trick with two FNV-1a variants:

```rust
h1 = fnv1a(key, FNV_OFFSET)       // standard FNV-1a offset
h2 = fnv1a(key, FNV_PRIME)        // FNV-1a with prime as offset

bit_i = (h1 + i * h2) % num_bits  // for i in 0..k
```

FNV-1a (Fowler–Noll–Vo) is fast, non-cryptographic, and produces well-distributed outputs:

```rust
fn fnv1a(data: &[u8], offset: u64) -> u64 {
    let mut h = offset;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211u64);  // FNV prime
    }
    h
}
```

### Optimal hash count

The number of hash functions `k` is chosen to minimise the false positive rate for the given `m` and `n`:

```
k = ceil( (m/n) * ln(2) )
```

This is clamped to `[1, 8]` to prevent degenerate cases.

### Insert and query

**Insert** sets `k` bits:
```rust
pub fn insert(&mut self, key: &[u8]) {
    for bit in self.hashes(key) {
        self.bits[bit / 8] |= 1 << (bit % 8);
    }
}
```

**Query** checks all `k` bits — if any is 0, the key is definitely absent:
```rust
pub fn may_contain(&self, key: &[u8]) -> bool {
    self.hashes(key).all(|bit| self.bits[bit / 8] & (1 << (bit % 8)) != 0)
}
```

### Serialisation

The filter is serialised into the SSTable file as:

```
num_bits  : u32  (4 bytes)
num_hashes: u32  (4 bytes)
bit array : [u8; ceil(num_bits / 8)]
```

On `SsTableReader::open`, the bloom section is read back and deserialised so the filter is ready in memory before the first query hits.

### False positive behaviour

A false positive means `may_contain` returns `true` for a key that isn't in the SSTable. The index lookup (`index.get(key)`) then returns `None`, and the get returns `None` without reading the data section. The bloom filter only saves the disk seek — the index is the authoritative source of truth.

```
may_contain = true  →  index lookup  →  key absent  →  None   (false positive, no data read)
may_contain = true  →  index lookup  →  offset found →  seek + read value
may_contain = false →  return None immediately                 (no index lookup, no disk read)
```

---

## Component interaction summary

```
PUT "user_007" value
  └── WAL.append()              persisted to disk first
  └── MemTable.put()            stored in BTreeMap
  └── MemTable.is_full()?
        YES → SsTableWriter.flush(memtable, "sst_0001.sst")
                └── iterates BTreeMap (sorted order)
                └── builds BloomFilter + index in one pass
                └── writes data → index → bloom → footer
              MemTable.clear()
              WAL.clear()

GET "user_007"
  └── MemTable.get()            O(log n) BTreeMap lookup
        found → return (may be tombstone)
        not found → check SSTables newest→oldest
          └── BloomFilter.may_contain()   O(k) bit checks, in memory
                false → skip file
                true  → BTreeMap index lookup → seek → read value
```
