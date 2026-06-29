# SQLEngine — Simple SQL Engine in Rust

A from-scratch SQL storage engine built in Rust to understand how databases store and retrieve data internally. Uses a **B+ tree** on a **page-structured binary file** for disk-persistent, indexed storage.

---

## Table of Contents

1. [Overview](#overview)
2. [Source Layout](#source-layout)
3. [The Database File](#the-database-file)
4. [Page Layout](#page-layout)
   - [Page Header](#page-header)
   - [Metadata Page (Page 0)](#metadata-page-page-0)
   - [Leaf Page](#leaf-page)
   - [Internal Page](#internal-page)
5. [User Record Layout](#user-record-layout)
6. [B+ Tree Structure](#b-tree-structure)
   - [Internal Nodes](#internal-nodes)
   - [Leaf Nodes](#leaf-nodes)
   - [Insert and Page Splitting](#insert-and-page-splitting)
   - [Point Lookup](#point-lookup)
   - [Range Query](#range-query)
7. [Sample Pages with Real Data](#sample-pages-with-real-data)
   - [Metadata Page (Page 0)](#sample-metadata-page)
   - [Leaf Page (Page 1)](#sample-leaf-page)
   - [Level-2 Internal Page (Page 3)](#sample-level-2-internal-page)
   - [Root Internal Page (Page 344)](#sample-root-page)
   - [Full Tree Picture](#full-tree-picture)
8. [Real Numbers from 10,000 Records](#real-numbers-from-10000-records)
9. [Running](#running)
10. [Commands and Sample Inputs](#commands-and-sample-inputs)
11. [Traversal Logs](#traversal-logs)
    - [INSERT logs](#insert-logs)
    - [UPDATE logs](#update-logs)
    - [DELETE logs](#delete-logs)
    - [FIND logs](#find-logs)
    - [RANGE logs](#range-logs)

---

## Overview

The engine stores `User` records indexed by `id` (u64) in a B+ tree written directly to a binary file (`users.db`). There is no query parser — the engine exposes these operations via an interactive REPL:

| Command | Description |
|---|---|
| `INSERT <id> <name> <age> <phone> <address>` | Add a record (rejects duplicate ids) |
| `UPDATE <id> <name> <age> <phone> <address>` | Overwrite non-key fields in-place |
| `DELETE <id>` | Remove a record, compact the leaf page |
| `FIND <id>` | Point lookup via B+ tree traversal |
| `RANGE <start> <end>` | Scan all records in id range |
| `COUNT` | Count all records by scanning leaf chain |

Every mutating and read command prints **step-by-step traversal logs** showing exactly which pages were read, which slots shifted, and what bytes were written.

---

## Source Layout

```
src/
├── config.rs   — compile-time constants (page size, capacities)
├── record.rs   — User struct and fixed-size byte serialization
├── page.rs     — page header, leaf helpers, internal node helpers
├── pager.rs    — file I/O: read/write pages by page_id
├── btree.rs    — B+ tree: insert, point lookup, range scan
├── engine.rs   — public query API wrapping the tree
└── main.rs     — demo: 10k inserts + queries
```

---

## The Database File

Everything lives in a single binary file: `users.db`.

The file is divided into **fixed-size pages** of `PAGE_SIZE = 4096` bytes each. A page is addressed by its **page_id** (u32). The byte offset of page N in the file is:

```
offset = page_id × 4096
```

This means reading any page is a single `seek + read` with no scanning.

```
users.db
┌─────────────────────────────────────────────────────────┐
│ Page 0  │ Page 1  │ Page 2  │ Page 3  │ ... │ Page 629 │
│ 4096 B  │ 4096 B  │ 4096 B  │ 4096 B  │     │ 4096 B   │
│ META    │ LEAF    │ LEAF    │ INTERNAL│     │ LEAF     │
└─────────────────────────────────────────────────────────┘
         ↑
    always the metadata page
```

Total file size for 10,000 records: **2,520 KB** (630 pages × 4096 bytes).

---

## Page Layout

Every page starts with a **24-byte header**, followed by a body whose interpretation depends on the page type.

### Page Header

```
Byte offset   Size    Field
───────────────────────────────────────────────────────
0             1 B     page_type   (0=Meta, 1=Internal, 2=Leaf)
1–3           3 B     padding
4–7           4 B     num_slots   (records in leaf / keys in internal)
8–11          4 B     next_page_id
                        • Leaf    → page_id of next leaf in chain
                        • Internal→ unused (0xFFFFFFFF)
                        • Meta    → unused
12–23        12 B     reserved (zeros)
───────────────────────────────────────────────────────
Total: 24 bytes
```

### Metadata Page (Page 0)

Page 0 is written on every `close()` and read on `open()`. It holds only two values:

```
Byte offset   Field
──────────────────────────────────
0–3           root_page_id  (u32)
4–7           num_pages     (u32)
──────────────────────────────────
```

Raw hex example (root=344=0x158, num_pages=630=0x276):

```
00000000: 58 01 00 00  76 02 00 00  00 00 …
          └─────────┘  └─────────┘
          root=0x158   pages=0x276
          = 344        = 630
```

### Leaf Page

After the 24-byte header, leaf pages contain back-to-back fixed-size **128-byte records**, sorted by `id` in ascending order.

```
┌──────────────────────────────────────────────────────────────┐
│ HEADER (24 B)                                                │
│   page_type=2, num_slots=16, next_page_id=2                  │
├──────────────┬──────────────┬──────────────┬─────────────────┤
│ Record  [0]  │ Record  [1]  │ Record  [2]  │  …  Record [30] │
│  128 bytes   │  128 bytes   │  128 bytes   │     128 bytes   │
│  id=1        │  id=2        │  id=3        │     id=31       │
└──────────────┴──────────────┴──────────────┴─────────────────┘
         ↑
    up to 31 records per page  (4096 - 24) / 128 = 31
```

The `next_page_id` in the header forms a **singly-linked list** across all leaf pages in sorted order. This chain is what enables range scans without touching any internal nodes after the first.

```
Page 1 → Page 2 → Page 4 → Page 5 → … → Page 629 → 0xFFFFFFFF (end)
id 1–16  id 17–31  id 32–47                           id ~9985–10000
```

### Internal Page

After the 24-byte header, internal pages store a **sorted array of (key, right_child_page_id) pairs**, preceded by a single **leftmost child pointer**.

```
┌──────────────────────────────────────────────────────────────────┐
│ HEADER (24 B)                                                    │
│   page_type=1, num_slots=N (number of keys)                      │
├────────────┬──────────────────────┬──────────────────────┬───────┤
│ leftmost   │ key[0]  child[0]     │ key[1]  child[1]     │  …   │
│ child (4B) │ (8B)    (4B) = 12B  │ (8B)    (4B) = 12B  │       │
└────────────┴──────────────────────┴──────────────────────┴───────┘

Byte offsets (from page start):
  24        → leftmost child (u32, 4 bytes)
  28 + i×12 → key[i]   (u64, 8 bytes)
  36 + i×12 → child[i] (u32, 4 bytes)   ← right child of key[i]
```

Maximum keys per internal page: `(4096 - 24 - 4) / 12 = 339`

---

## User Record Layout

Every `User` is serialized to exactly **128 bytes**:

```
Byte range    Size    Field        Type
──────────────────────────────────────────────
[0   .. 7]    8 B     id           u64 (little-endian)
[8   .. 39]  32 B     name         UTF-8, zero-padded
[40]          1 B     age          u8
[41  .. 56]  16 B     phone        UTF-8, zero-padded
[57  .. 119] 63 B     address      UTF-8, zero-padded
[120 .. 127]  8 B     padding      zeros
──────────────────────────────────────────────
Total: 128 bytes
```

Raw hex of the record for `id=1, name="User_1", age=21, phone="+1-555-0001", address="1 Main St, City 1"`:

```
[  0]  01 00 00 00 00 00 00 00  ← id=1 (u64 LE)
[  8]  55 73 65 72 5f 31 00 00  ← "User_1\0\0"
[ 16]  00 00 00 00 00 00 00 00  ← name padding
[ 24]  00 00 00 00 00 00 00 00
[ 32]  00 00 00 00 00 00 00 00
[ 40]  15                       ← age=21 (0x15)
[ 41]  2b 31 2d 35 35 35 2d 30  ← "+1-555-0"
[ 49]  30 30 31 00 00 00 00 00  ← "001\0\0\0\0\0"
[ 57]  31 20 4d 61 69 6e 20 53  ← "1 Main S"
[ 65]  74 2c 20 43 69 74 79 20  ← "t, City "
[ 73]  31 00 00 00 00 00 00 00  ← "1\0..."
[ 81]  00 … 00                  ← address + record padding
```

Fixed-size records mean any slot can be read with a single index calculation — no variable-length scanning needed.

---

## B+ Tree Structure

The B+ tree is the core data structure. It provides O(log N) insert and lookup, and O(log N + K) range scan for K results.

### Internal Nodes

Internal nodes store **separator keys** that guide traversal. A node with N keys has N+1 children:

```
         [key0=2737 | key1=5473]
         /          |            \
   page_3       page_343       page_516
  (ids < 2737) (2737..5472)  (5473..10000)
```

To find which child to follow for a search key `k`:
- If `k < key[0]` → follow leftmost child
- If `key[i] <= k < key[i+1]` → follow child[i]
- If `k >= key[last]` → follow rightmost child

### Leaf Nodes

Leaf nodes hold the actual `User` records in sorted order by `id`. They are linked in a chain via `next_page_id`:

```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│ Page 1       │───▶│ Page 2       │───▶│ Page 4       │
│ ids 1..16    │    │ ids 17..31   │    │ ids 32..47   │
└──────────────┘    └──────────────┘    └──────────────┘
```

Unlike internal nodes, leaf nodes contain the **full record data**, not just keys.

### Insert and Page Splitting

When a leaf is full (31 records), it **splits** into two pages:

```
Before split (leaf full, 31 records):
┌──────────────────────────────────────────┐
│ id=1 id=2 id=3 … id=31                  │
└──────────────────────────────────────────┘

After split (inserting id=32):
┌──────────────────────┐  ┌──────────────────────┐
│ id=1 … id=16         │  │ id=17 … id=32         │
│ next_page_id=new_page│  │ next_page_id=old_next │
└──────────────────────┘  └──────────────────────┘
        ↑ pushed-up key = 17 goes to parent internal node
```

The **pushed-up key** (first key of the right half) is inserted into the parent internal node. If the parent is also full, it splits too and pushes a key up to its parent — recursively up to the root. When the root splits, a new root is created, increasing the tree height by 1.

Internal node splits work the same way: the middle key is pushed up and the node splits left/right.

### Point Lookup

```
find_by_id(7777):

Root (page 344) → internal, keys=[2737, 5473]
  7777 >= 5473 → follow right child (page 516)

Page 516 → internal
  binary search on keys → follow child (some leaf page)

Leaf page → binary search on records → return User{id=7777, …}

Total: 3 page reads = 3 × disk seeks
```

### Range Query

```
range_query(500, 510):

1. Traverse tree to find the leaf containing id=500  (3 page reads)
2. Scan forward through records, collecting ids 500..510
3. Follow next_page_id if the range spans multiple leaves
4. Stop when a record id > 510 is seen

Total disk reads: tree height + number of pages spanning the range
```

---

## Sample Pages with Real Data

These are actual pages dumped from `users.db` after inserting 10,000 records.

---

### Sample Metadata Page

**Page 0** — always at byte offset 0 in the file.

```
┌─────────────────────────────────────────────────────────┐
│  PAGE 0 — METADATA                    (4096 bytes)      │
├─────────────────────────────────────────────────────────┤
│  Bytes [0..3]   root_page_id = 344                      │
│  Bytes [4..7]   num_pages    = 630                      │
│  Bytes [8..4095] zeros                                  │
└─────────────────────────────────────────────────────────┘

Raw hex (first 8 bytes):
  58 01 00 00  76 02 00 00
  └─────────┘  └─────────┘
  344 (LE)     630 (LE)
```

---

### Sample Leaf Page

**Page 1** — byte offset 4,096 in the file. The very first leaf ever created.

```
┌─────────────────────────────────────────────────────────────────────┐
│  PAGE 1 — LEAF                                     (4096 bytes)     │
├─────────────────────────────────────────────────────────────────────┤
│  HEADER (24 bytes)                                                  │
│    page_type    = 2  (Leaf)                                         │
│    num_slots    = 16                                                │
│    next_page_id = 2  ──────────────────────────────► Page 2        │
├─────────────────────────────────────────────────────────────────────┤
│  slot[ 0]  id=1    name="User_1"    age=21  phone="+1-555-0001"    │
│            address="1 Main St, City 1"                              │
├─────────────────────────────────────────────────────────────────────┤
│  slot[ 1]  id=2    name="User_2"    age=22  phone="+1-555-0002"    │
│            address="2 Main St, City 2"                              │
├─────────────────────────────────────────────────────────────────────┤
│  slot[ 2]  id=3    name="User_3"    age=23  phone="+1-555-0003"    │
│            address="3 Main St, City 3"                              │
├─────────────────────────────────────────────────────────────────────┤
│  slot[ 3]  id=4    name="User_4"    age=24  phone="+1-555-0004"    │
│            address="4 Main St, City 4"                              │
├─────────────────────────────────────────────────────────────────────┤
│  slot[ 4]  id=5    name="User_5"    age=25  phone="+1-555-0005"    │
│            address="5 Main St, City 5"                              │
├─────────────────────────────────────────────────────────────────────┤
│  slot[ 5]  id=6    name="User_6"    age=26  phone="+1-555-0006"    │
│            address="6 Main St, City 6"                              │
├─────────────────────────────────────────────────────────────────────┤
│  slot[ 6]  id=7    name="User_7"    age=27  phone="+1-555-0007"    │
│            address="7 Main St, City 7"                              │
├─────────────────────────────────────────────────────────────────────┤
│  slot[ 7]  id=8    name="User_8"    age=28  phone="+1-555-0008"    │
│            address="8 Main St, City 8"                              │
├─────────────────────────────────────────────────────────────────────┤
│  slot[ 8]  id=9    name="User_9"    age=29  phone="+1-555-0009"    │
│            address="9 Main St, City 9"                              │
├─────────────────────────────────────────────────────────────────────┤
│  slot[ 9]  id=10   name="User_10"   age=30  phone="+1-555-0010"   │
│            address="10 Main St, City 10"                            │
├─────────────────────────────────────────────────────────────────────┤
│  slot[10]  id=11   name="User_11"   age=31  phone="+1-555-0011"   │
│            address="11 Main St, City 11"                            │
├─────────────────────────────────────────────────────────────────────┤
│  slot[11]  id=12   name="User_12"   age=32  phone="+1-555-0012"   │
│            address="12 Main St, City 12"                            │
├─────────────────────────────────────────────────────────────────────┤
│  slot[12]  id=13   name="User_13"   age=33  phone="+1-555-0013"   │
│            address="13 Main St, City 13"                            │
├─────────────────────────────────────────────────────────────────────┤
│  slot[13]  id=14   name="User_14"   age=34  phone="+1-555-0014"   │
│            address="14 Main St, City 14"                            │
├─────────────────────────────────────────────────────────────────────┤
│  slot[14]  id=15   name="User_15"   age=35  phone="+1-555-0015"   │
│            address="15 Main St, City 15"                            │
├─────────────────────────────────────────────────────────────────────┤
│  slot[15]  id=16   name="User_16"   age=36  phone="+1-555-0016"   │
│            address="16 Main St, City 16"                            │
├─────────────────────────────────────────────────────────────────────┤
│  [unused free space — 0 bytes, page is partially filled]            │
└─────────────────────────────────────────────────────────────────────┘
```

Each slot is 128 bytes. With `num_slots=16`, used body = 16 × 128 = 2,048 bytes.
Remaining 2,024 bytes are zeroed free space (this page was split early and never refilled).

---

### Sample Level-2 Internal Page

**Page 3** — the internal node directly below the root, covering ids 1..2736.

```
┌─────────────────────────────────────────────────────────────────────┐
│  PAGE 3 — INTERNAL (level 2)                       (4096 bytes)     │
├─────────────────────────────────────────────────────────────────────┤
│  HEADER (24 bytes)                                                  │
│    page_type    = 1  (Internal)                                     │
│    num_slots    = 170  (170 separator keys → 171 children)          │
│    next_page_id = 0xFFFFFFFF  (unused for internal nodes)           │
├─────────────────────────────────────────────────────────────────────┤
│  leftmost_child = page_1   (covers ids < 17)                        │
├──────────────┬──────────────────────────────────────────────────────┤
│  key[  0]=17 │ right_child = page_2   (covers ids 17..32)           │
│  key[  1]=33 │ right_child = page_4   (covers ids 33..48)           │
│  key[  2]=49 │ right_child = page_5   (covers ids 49..64)           │
│  key[  3]=65 │ right_child = page_6   (covers ids 65..80)           │
│  key[  4]=81 │ right_child = page_7   (covers ids 81..96)           │
│  key[  5]=97 │ right_child = page_8   (covers ids 97..112)          │
│     …        │  …  (164 more keys, one per leaf page)               │
│  key[169]=…  │ right_child = page_…   (last leaf in this subtree)   │
└──────────────┴──────────────────────────────────────────────────────┘

Body layout in bytes (from offset 24):
  [24..27]   leftmost child page_id  (4 bytes)
  [28..35]   key[0] = 17             (8 bytes, u64 LE)
  [36..39]   right child = page_2    (4 bytes, u32 LE)
  [40..47]   key[1] = 33             (8 bytes)
  [48..51]   right child = page_4    (4 bytes)
  … repeating every 12 bytes …
```

---

### Sample Root Page

**Page 344** — the current root, created when the tree grew to 3 levels.

```
┌─────────────────────────────────────────────────────────────────────┐
│  PAGE 344 — INTERNAL (ROOT, level 1)               (4096 bytes)     │
├─────────────────────────────────────────────────────────────────────┤
│  HEADER (24 bytes)                                                  │
│    page_type    = 1  (Internal)                                     │
│    num_slots    = 2  (2 separator keys → 3 children/subtrees)       │
│    next_page_id = 0xFFFFFFFF                                        │
├─────────────────────────────────────────────────────────────────────┤
│  leftmost_child = page_3                                            │
│                   └─► subtree covering ids 1 .. 2736               │
├───────────────────┬─────────────────────────────────────────────────┤
│  key[0] = 2737    │ right_child = page_343                          │
│                   │ └─► subtree covering ids 2737 .. 5472           │
├───────────────────┼─────────────────────────────────────────────────┤
│  key[1] = 5473    │ right_child = page_516                          │
│                   │ └─► subtree covering ids 5473 .. 10000          │
└───────────────────┴─────────────────────────────────────────────────┘
```

Only 2 × 12 = 24 bytes of body used. The remaining 4,048 bytes are free space —
internal nodes rarely get close to their 339-key capacity.

---

### Full Tree Picture

The complete tree with real page numbers from the 10,000-record database:

```
                         ┌──────────────────────────────────┐
                         │  ROOT: Page 344  (Internal)      │
                         │  keys: [2737, 5473]              │
                         └──────────────────────────────────┘
                          /            |              \
                 page_3            page_343          page_516
              (ids 1–2736)      (ids 2737–5472)   (ids 5473–10000)
              170 keys           170 keys           169 keys
                 |
         ┌───────┼───────┐──  …  ──┐
       page_1  page_2  page_4    page_…
      (1–16)  (17–32) (33–48)
      16 recs  16 recs 16 recs
         │       │
         └───────┘
      linked via next_page_id
      for range scans
```

A lookup for any id follows exactly **2 internal node reads + 1 leaf read = 3 page I/Os**,
regardless of where in the 10,000-record dataset the record lives.

---

## Real Numbers from 10,000 Records

All measurements from an unoptimized debug build on macOS.

| Metric | Value |
|---|---|
| Total records | 10,000 |
| Page size | 4,096 bytes |
| Record size | 128 bytes |
| Records per leaf page | 31 |
| Keys per internal page | 339 |
| Total pages | 630 |
| Leaf pages | 625 |
| Internal pages | 4 |
| Metadata pages | 1 |
| B+ tree height | 3 |
| Database file size | 2,520 KB |
| Insert 10k records | ~213 ms |
| Point lookup | ~12 µs |
| Range query (11 records) | ~16 µs |

Tree height of 3 means every lookup touches exactly **3 pages** regardless of which record is searched — from id=1 to id=10000.

With 339 keys per internal node and 31 records per leaf:
- Height 2 can hold up to 339 × 31 = **10,509 records**
- Height 3 can hold up to 339² × 31 = **3,562,569 records**
- Height 4 can hold up to 339³ × 31 ≈ **1.2 billion records**

---

## Running

### Build

```bash
cargo build
```

### Start the interactive shell

```bash
cargo run
```

Opens a `sql>` prompt against `users.db` (created fresh if it does not exist).

### Seed with 10,000 records

```bash
cargo run -- seed
```

Wipes any existing `users.db` and inserts 10,000 generated records. Use this to
pre-populate the database before querying.

---

## Commands and Sample Inputs

### INSERT

Add a single user record.

```
INSERT <id> <name> <age> <phone> <address>
```

Rules:
- `id` — positive integer, must be unique
- `name` — max 32 characters
- `age` — 0–255
- `phone` — max 16 characters
- `address` — max 63 characters (address may contain spaces)

**Examples:**

```
sql> INSERT 1 Alice 30 +1-555-0001 123 Main St, New York
Inserted id=1 in 11µs

sql> INSERT 2 Bob 25 +1-555-0002 456 Oak Ave, Los Angeles
Inserted id=2 in 8µs

sql> INSERT 3 Charlie 35 +1-555-0003 789 Pine Rd, Chicago
Inserted id=3 in 9µs

sql> INSERT 1001 Diana 28 +44-20-7946 10 Downing St, London
Inserted id=1001 in 9µs
```

---

### FIND

Point lookup by id. Traverses the B+ tree (3 page reads for 10k records).

```
FIND <id>
```

**Examples:**

```
sql> FIND 3
Found in 5µs:
  id=3      name=Charlie         age=35   phone=+1-555-0003     address=789 Pine Rd, Chicago

sql> FIND 1001
Found in 5µs:
  id=1001   name=Diana           age=28   phone=+44-20-7946     address=10 Downing St, London

sql> FIND 9999
Found in 12µs:
  id=9999   name=User_9999       age=59   phone=+1-555-9999     address=9999 Main St, City 99

sql> FIND 42
No record with id=42
```

---

### RANGE

Fetch all records where `start_id <= id <= end_id`.
Finds the start leaf via the tree, then follows the leaf linked list.

```
RANGE <start_id> <end_id>
```

**Examples:**

```
sql> RANGE 1 3
3 record(s) found in 12µs:
  id=1      name=Alice           age=30   phone=+1-555-0001     address=123 Main St, New York
  id=2      name=Bob             age=25   phone=+1-555-0002     address=456 Oak Ave, Los Angeles
  id=3      name=Charlie         age=35   phone=+1-555-0003     address=789 Pine Rd, Chicago

sql> RANGE 500 510
11 record(s) found in 15µs:
  id=500    name=User_500        age=40   phone=+1-555-0500     address=500 Main St, City 0
  id=501    name=User_501        age=41   phone=+1-555-0501     address=501 Main St, City 1
  ...
  id=510    name=User_510        age=50   phone=+1-555-0510     address=510 Main St, City 10

sql> RANGE 9998 10000
3 record(s) found in 18µs:
  id=9998   name=User_9998       age=58   phone=+1-555-9998     address=9998 Main St, City 98
  id=9999   name=User_9999       age=59   phone=+1-555-9999     address=9999 Main St, City 99
  id=10000  name=User_10000      age=60   phone=+1-555-10000    address=10000 Main St, City 0
```

---

### COUNT

Count total records by scanning all leaf pages.

```
COUNT
```

**Example:**

```
sql> COUNT
Total records: 10000 (scanned in 12ms)
```

---

### EXIT / QUIT

Flush metadata to disk and close the database.

```
sql> EXIT
Bye.
```

---

### UPDATE

Update non-key fields (name, age, phone, address) in-place for an existing id.
The record is overwritten at the same slot and offset — no slots shift, no page split.
To change the `id` itself, use `DELETE` + `INSERT`.

```
UPDATE <id> <name> <age> <phone> <address>
```

**Examples:**

```
sql> UPDATE 100 Alice 45 +1-999-0100 456 New Ave Boston
Updated id=100 in 16µs

sql> UPDATE 99999 Ghost 20 +1-000-0000 Nowhere
No record with id=99999. Use INSERT to add it.
```

---

### DELETE

Remove a record by id. Shifts all subsequent slots in the leaf one position left
to compact the page. No underflow merging — partially filled pages remain as-is.

```
DELETE <id>
```

**Examples:**

```
sql> DELETE 100
Deleted id=100 in 185µs

sql> DELETE 99999
No record with id=99999
```

---

### HELP

Print all available commands.

```
sql> HELP
Commands:
  INSERT <id> <name> <age> <phone> <address>   Insert a user (error if id exists)
  UPDATE <id> <name> <age> <phone> <address>   Update non-key fields in-place
  DELETE <id>                                   Delete a record by id
  FIND   <id>                                   Lookup by id
  RANGE  <start_id> <end_id>                   Fetch all ids in range
  COUNT                                         Count all records
  HELP                                          Show this message
  EXIT                                          Close and quit

Field limits:  name ≤ 32 chars  |  phone ≤ 16 chars  |  address ≤ 63 chars
Database file: users.db
```

---

### Full session example

```bash
$ cargo run -- seed          # load 10k records
Seeding 10,000 records into users.db...
Done in 213ms. Database: users.db

$ cargo run                  # open interactive shell
SQLEngine — database: users.db
Type HELP for available commands.

sql> FIND 7777
Found in 12µs:
  id=7777   name=User_7777   age=57   phone=+1-555-7777   address=7777 Main St, City 77

sql> UPDATE 7777 Zara 29 +1-999-7777 99 Harbor Blvd Miami
Updated id=7777 in 16µs

sql> INSERT 7777 Dup 99 +1-000-0000 Nowhere
Error: id=7777 already exists. Use UPDATE to modify it.

sql> DELETE 7777
Deleted id=7777 in 185µs

sql> FIND 7777
No record with id=7777

sql> RANGE 9998 10000
3 record(s) found in 18µs:
  id=9998    name=User_9998    age=58   phone=+1-555-9998    address=9998 Main St, City 98
  id=9999    name=User_9999    age=59   phone=+1-555-9999    address=9999 Main St, City 99
  id=10000   name=User_10000   age=60   phone=+1-555-10000   address=10000 Main St, City 0

sql> COUNT
Total records: 9999 (scanned in 12ms)

sql> EXIT
Bye.
```

To start completely fresh, delete `users.db`:

```bash
rm users.db && cargo run
```

---

## Traversal Logs

Every command prints step-by-step logs showing exactly which pages were read,
which slots moved, and what bytes were written. This makes the engine a learning
tool — you can watch the B+ tree work in real time.

---

### INSERT logs

**Case 1 — room available, no split:**

```
sql> INSERT 100 Alice 30 +1-555-0100 100 Main St NY
  [insert] starting at root page 344
  [insert] page  344 [INTERNAL,   2 keys] → id 100 < key[0]=2737 → leftmost child → page 3
  [insert] page    3 [INTERNAL, 170 keys] → key[5]=97 ≤ id 100 < key[6]=113 → page 8
  [insert] page    8 [LEAF,      15 records, ids 97..112] → inserting here
  [insert] page has room (15/31), no split needed
  [insert] inserting id=100 at slot[3]  offset=408 (24+3×128)
  [insert] shifting slots 3..14 one position RIGHT to make room:
           id=101   offset 408 → 536  (slot[3] → slot[4])
           id=102   offset 536 → 664  (slot[4] → slot[5])
           ...
  [insert] wrote id=100 at slot[3]  offset=408
  [insert] num_slots: 15 → 16
  [insert] writing page 8 to disk
Inserted id=100 in 104µs
```

**Case 2 — leaf full, split required:**

```
sql> INSERT 32 User32 52 +1-555-0032 32 Main St
  [insert] page    1 [LEAF, 31 records, ids 1..31] → inserting here
  [insert] page 1 is FULL (31/31) — SPLIT required
  [insert] merging 31 existing + 1 new = 32 records, sorted
  [insert] split at midpoint 16:
           LEFT  page  1 ← ids 1..16  (16 records)
           RIGHT page  2 ← ids 17..32 (16 records)
  [insert] pushed-up key=17 → parent must add (key=17, right_child=page_2)
  [insert] writing left  half to page 1 (next_leaf=page_2)
  [insert] writing right half to page 2 (next_leaf=none)
  [insert] ROOT SPLIT — new root page 3 created
           left subtree  = old root page 1
           right subtree = page 2
           separator key = 17
           tree height increased by 1
Inserted id=32 in 38µs
```

---

### UPDATE logs

UPDATE overwrites exactly 128 bytes in-place. No slots shift, no tree changes.

```
sql> UPDATE 200 NewAlice 45 +1-999-0200 789 New Ave Boston
  [update] searching for id=200
  [update] page   14 [LEAF, 16 records, ids 193..208] → start scanning here
  [update] arrived at page 14 [LEAF, 16 records, ids 193..208]
  [update] found id=200 at slot[7]  offset=920 (24+7×128)
  [update] overwriting in-place (no slot shift needed):
           field     OLD value            NEW value
           ─────────────────────────────────────────────────────
           id        200                  200 (unchanged — key field)
           name      User_200             NewAlice
           age       40                   45
           phone     +1-555-0200          +1-999-0200
           address   200 Main St, City 0  789 New Ave Boston
  [update] 128 bytes written at offset 920..1048 in page 14
  [update] no other slots moved — all offsets unchanged
  [update] writing page 14 to disk
Updated id=200 in 69µs
```

---

### DELETE logs

DELETE removes the slot and shifts all subsequent slots left to compact the page.

```
sql> DELETE 100
  [delete] searching for id=100
  [delete] arrived at page 8 [LEAF, 16 records, ids 97..112]
  [delete] page 8 slot layout BEFORE delete:
           slot[ 0]  offset=24     id=97
           slot[ 1]  offset=152    id=98
           slot[ 2]  offset=280    id=99
           slot[ 3]  offset=408    id=100 ← DELETE THIS
           slot[ 4]  offset=536    id=101
           ...
           slot[15]  offset=1944   id=112
  [delete] removed slot[3] at offset 408 (24+3×128)
  [delete] shifting slots 4..15 one position LEFT:
           id=101   offset 536 → 408  (slot[4] → slot[3])
           id=102   offset 664 → 536  (slot[5] → slot[4])
           ...
           id=112   offset 1944 → 1816  (slot[15] → slot[14])
  [delete] slot[15] at offset 1944 zeroed (free space)
  [delete] num_slots: 16 → 15
  [delete] page 8 slot layout AFTER delete:
           slot[ 0]  offset=24     id=97
           ...
           slot[ 3]  offset=408    id=101
           ...
           slot[14]  offset=1816   id=112
           slot[15]  offset=1944   [empty / zeroed]
  [delete] writing page 8 to disk
Deleted id=100 in 185µs
```

---

### FIND logs

```
sql> FIND 7777
  [traversal] starting at root page 344
  [traversal] page  344 [INTERNAL,   2 keys] → id 7777 ≥ key[1]=5473 → rightmost child → page 516
  [traversal] page  516 [INTERNAL, 282 keys] → key[143]=7777 ≤ id 7777 < key[144]=7793 → page 490
  [traversal] page  490 [LEAF,      16 records, ids 7777..7792] → binary search for id=7777
  [traversal] → FOUND at page 490
Found in 54µs:
  id=7777   name=User_7777   age=57   phone=+1-555-7777   address=7777 Main St, City 77
```

---

### RANGE logs

```
sql> RANGE 500 510
  [traversal] locating start leaf for id=500
  [traversal] page  344 [INTERNAL,  2 keys] → id 500 < key[0]=2737 → leftmost child → page 3
  [traversal] page    3 [INTERNAL, 170 keys] → key[30]=497 ≤ id 500 < key[31]=513 → page 33
  [traversal] page   33 [LEAF,      16 records, ids 497..512] → start scanning here
  [leaf 0] page 33 │ 16 records (id 497..512) │ collected 11 │ next → page 34 │ RANGE END
11 record(s) found in 16µs:
  id=500    name=User_500   ...
  ...
  id=510    name=User_510   ...
```

The `[leaf N]` lines show each leaf page scanned, how many records were collected
from it, and where the chain continues. `RANGE END` marks the leaf where the scan stopped.
