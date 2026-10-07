# Daikon — offline Redis notes

A self-contained reference for continuing the Redis clone without internet access.

## Contents

1. [Where you are now](#1-where-you-are-now)
2. [RESP protocol cheat sheet](#2-resp-protocol-cheat-sheet)
3. [Redis data types](#3-redis-data-types)
4. [Next 10 commands to implement](#4-next-10-commands-to-implement)
5. [Stretch goals](#5-stretch-goals)
6. [Testing without redis-cli](#6-testing-without-redis-cli)

---

## 1. Where you are now

### Implemented commands

| Command | Notes |
|---|---|
| `PING` | Replies `+PONG` |
| `ECHO message` | Echoes the argument back as a bulk string |
| `SET key value [EX seconds \| PX milliseconds]` | Overwrites any existing value and TTL |
| `GET key` | Lazy expiry: checks the TTL on read and deletes the key if it has expired |
| `RPUSH` | Parsed into `Command::RPush`, but not dispatched yet, so it replies `-Unknown command` |

### Code layout

- `src/parser/commands.rs`: `parse_command_array` turns the RESP array into a `CommandArray { command, key, value, ttl }`.
- `src/lib.rs`: `execute_command` dispatches on `Command`.
- `src/commands/<cmd>.rs`: one `invoke(storage, command_array)` per command (`get.rs`, `set.rs`).
- `src/data/types.rs`: the `Value` RESP type and `StoredValue`.

### Implemented RESP types (`src/data/types.rs`)

`Array`, `SimpleString`, `BulkString`, `NullBulkString`, `Err` (encodes as `-{msg}\r\n`).

### Things the next commands will need

Hints only, not solutions:

- **RESP Integers (`:`)** — `DEL`, `EXISTS`, `INCR`, `TTL`, `RPUSH` and `HSET` all reply with integers. `Value` doesn't have an integer variant yet.
- **`GET` and `ECHO` reply with the wrong type** — `parse_value` turns the incoming bulk string into a `Value::SimpleString`, and that's what gets stored, so `GET foo` replies `+bar\r\n` instead of `$3\r\nbar\r\n`. `ECHO hi` replies `+hi\r\n` for the same reason. Both should be bulk strings: a simple string can't hold `\r` or `\n`, and `GET` on an empty string must reply `$0\r\n\r\n`.
- **Wire format vs. stored data** — the bug above is a symptom of this one. `StoredValue.value` is a `Value`, which is a *protocol* type. Lists and hashes aren't RESP types, so you'll probably want a separate enum for what's stored, e.g. `enum RedisData { String(..), List(..), Hash(..) }`. Encode to RESP only when you send the reply.
- **`CommandArray` has one slot per argument** — `key`, `value` and `ttl` fit `GET`/`SET`, but `RPUSH key a b c`, `DEL a b c` and `HSET k f1 v1 f2 v2` take a variable number of arguments. Decide whether `CommandArray` grows a `Vec` of arguments, or whether each command parses its own arguments.
- **Error replies** — `Value::Err` now encodes correctly, but every parse failure replies `-Unknown command`, whether the command is unknown, has too few arguments, or the input is malformed. By convention the first word is the error kind, so clients see the kind as `Unknown`. Redis replies:
  - unknown command: `-ERR unknown command 'foo', with args beginning with: …`
  - wrong argument count: `-ERR wrong number of arguments for 'get' command`. Extra arguments are an error too, but `GET a b` currently returns `a`.
- **Bad `SET` options are ignored** — `parse_ttl` uppercases the option now, so `px` works. But `SET k v EX abc`, `SET k v EX 0` and `SET k v FOO 10` all reply `+OK` without a TTL. Redis replies `-ERR value is not an integer or out of range`, `-ERR invalid expire time in 'set' command` and `-ERR syntax error`.
- **`unwrap()`s that kill the connection** — `execute_command(...).unwrap()` in `lib.rs` panics the task if a command returns `Err`. Pick a rule: errors the client should see become `Value::Err`, and `anyhow` errors are bugs. The parser also unwraps `parse_data_bytes(...)` and calls `split_to`/`split_off`, which panic when the buffer is too short.
- **Pipelining and partial reads** — `parse_command_array` parses one command and drops the `rest` that `parse_data_bytes` returns, so a second command in the same TCP read gets no reply (`PING PING` in one write gets one `+PONG`). The opposite case, a command split across two reads, hits the panics above. You need to keep unparsed bytes in `buf` and parse in a loop until it's empty or incomplete.
- **Expiry everywhere** — every command that reads a key must treat an expired key as missing. `StoredValue::is_expired()` exists, but `GET` checks the expiry inline instead. Build a helper such as `get_live(&storage, key)` on top of `is_expired()`.
- **Expiry race in `GET`** — `GET` checks the expiry, drops the read guard and then calls `storage.remove(&key).unwrap()`. If another client deletes the key in between, the `unwrap()` panics. If another client `SET`s a fresh value in between, `GET` deletes the fresh value. `DashMap::remove_if` checks and removes under a single lock.
- **CRLF check** — `next_resp_chunk` checks `bytes.get(chunk_sep_start)`, which is the `\r` it just found, so the test is always true. The comment says it meant to check for `\n`, which is at `chunk_sep_start + 1`.
- **`SystemTime` vs `Instant`** — `SystemTime` can jump backwards (NTP, manual clock changes). `std::time::Instant` is monotonic and is the usual choice for TTLs. (Redis itself uses wall-clock ms because it persists absolute expiry times to disk.)

---

## 2. RESP protocol cheat sheet

Every element ends with CRLF (`\r\n`). Clients always send commands as an **array of bulk strings**.

### RESP2 types (what redis-cli speaks by default)

| Type | Prefix | Example on the wire | Meaning |
|---|---|---|---|
| Simple string | `+` | `+OK\r\n` | Short, non-binary status reply |
| Error | `-` | `-ERR unknown command 'FOO'\r\n` | First word is the error kind by convention (`ERR`, `WRONGTYPE`) |
| Integer | `:` | `:1000\r\n`, `:-2\r\n` | Signed 64-bit |
| Bulk string | `$` | `$5\r\nhello\r\n` | Length in **bytes**, then the payload. Binary-safe |
| Null bulk string | `$` | `$-1\r\n` | "nil", e.g. `GET` on a missing key |
| Empty bulk string | `$` | `$0\r\n\r\n` | Empty string, which is not the same as nil |
| Array | `*` | `*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n` | Count, then each element |
| Empty array | `*` | `*0\r\n` | E.g. `LRANGE` on a missing key |
| Null array | `*` | `*-1\r\n` | Rare (e.g. `BLPOP` timeout) |

Arrays can be nested and can mix types: `*3\r\n:1\r\n$-1\r\n+OK\r\n` is valid.

### RESP3 additions (only after a client sends `HELLO 3`)

| Type | Prefix | Example |
|---|---|---|
| Null | `_` | `_\r\n` |
| Boolean | `#` | `#t\r\n` / `#f\r\n` |
| Double | `,` | `,3.14\r\n`, `,inf\r\n` |
| Big number | `(` | `(3492890328409238509324850943850943825024385\r\n` |
| Bulk error | `!` | `!21\r\nSYNTAX invalid syntax\r\n` |
| Verbatim string | `=` | `=15\r\ntxt:Some string\r\n` |
| Map | `%` | `%1\r\n+key\r\n:1\r\n` (count = number of pairs) |
| Set | `~` | `~2\r\n+a\r\n+b\r\n` |
| Push | `>` | Out-of-band messages (pub/sub) |

You can ignore RESP3 and reply in RESP2 throughout.

### Inline commands

Redis also accepts plain text commands such as `PING\r\n` or `SET foo bar\r\n` (space-separated, no `*`/`$`). This is what you're sending when you type into `telnet`/`nc`. Your parser doesn't support it yet, which is fine; see [section 6](#6-testing-without-redis-cli).

### Common error messages (exact text)

```
-ERR wrong number of arguments for 'get' command      (command name in lowercase)
-ERR unknown command 'foo', with args beginning with: 'a' 'b'
-ERR syntax error
-ERR value is not an integer or out of range
-ERR increment or decrement would overflow
-ERR invalid expire time in 'set' command
-WRONGTYPE Operation against a key holding the wrong kind of value
```

---

## 3. Redis data types

Every Redis key maps to exactly **one** value of **one** type. Commands are type-specific: `LPUSH` on a key holding a string fails with `WRONGTYPE`. The `TYPE key` command returns the type name shown in brackets below. Any key of any type can have a TTL.

### String (`string`)

- **What:** A binary-safe byte sequence, up to 512 MB. "Binary-safe" means it can contain any bytes, including `\0` and invalid UTF-8, such as a JPEG.
- **Integers and floats are also strings.** `INCR` parses the string as a signed 64-bit integer, adds one and stores it back as a string. Internally Redis has three encodings: `int` (the value fits in an i64), `embstr` (≤ 44 bytes, allocated together with the object header) and `raw`. `OBJECT ENCODING key` shows which one is in use.
- **Commands:** `SET`, `GET`, `GETSET`/`SET … GET`, `MSET`, `MGET`, `SETNX`, `SETEX`, `APPEND`, `STRLEN`, `GETRANGE`, `SETRANGE`, `INCR`, `INCRBY`, `INCRBYFLOAT`, `DECR`, `DECRBY`, `GETDEL`, `GETEX`.
- **Used for:** caching, counters, rate limiters, session tokens, distributed locks (`SET key token NX PX 30000`).
- **In Rust:** `Vec<u8>` or `bytes::Bytes`. `String` works for learning but isn't binary-safe.

### List (`list`)

- **What:** An ordered sequence of strings in insertion order, allowing duplicates. Fast push/pop at both ends; access by index is O(N).
- **Internals:** a *quicklist*, i.e. a doubly linked list of *listpacks* (compact contiguous byte arrays). Small lists are a single listpack.
- **Indexing:** 0-based. Negative indexes count from the tail: `-1` is the last element, `-2` the one before it.
- **Empty lists don't exist.** When the last element is popped, the key is deleted. This holds for every collection type (list, hash, set, zset).
- **Commands:** `LPUSH`, `RPUSH`, `LPOP`, `RPOP`, `LLEN`, `LRANGE`, `LINDEX`, `LSET`, `LINSERT`, `LREM`, `LTRIM`, `LMOVE`, `LPOS`, and blocking versions `BLPOP`, `BRPOP`, `BLMOVE`.
- **Used for:** queues (`LPUSH` + `BRPOP`), stacks, "latest N items" feeds (`LPUSH` + `LTRIM 0 99`), activity logs.
- **In Rust:** `std::collections::VecDeque<Bytes>`.

#### List vs. array

A list is its own data type (`TYPE mylist` → `list`). Redis has **no array data type** for storing data. "Array" only refers to the **RESP array** (`*3\r\n...`), which is a reply format. Many replies are sent as RESP arrays even though no list is involved:

| Command | What's being returned | Sent as |
|---|---|---|
| `LRANGE mylist 0 -1` | A list's elements | RESP array |
| `SMEMBERS myset` | A set's members | RESP array |
| `HGETALL myhash` | A hash's field/value pairs, flattened | RESP array |
| `MGET a b c` | Three separate string keys | RESP array |

So a list is something **stored** under a key, and an array is how a reply is **formatted**. In your code, the stored list belongs in your storage type (e.g. `RedisData::List(VecDeque<…>)`), and the reply is `Value::Array(Vec<Value>)`.

How a Redis list differs from what you'd think of as an array:

- **Elements can only be strings.** A list can't contain other lists, hashes or integers (numbers are stored as strings).
- **It's built for the ends, not the middle.** Push and pop at the head or tail are O(1), but `LINDEX`/`LSET` are O(N) because the list is a chain of small packed blocks, not one contiguous array.
- **It has no fixed size.** It grows as you push, and you never declare a capacity.
- **Empty lists don't exist.** Popping the last element deletes the key, and pushing to a missing key creates it.
- **Duplicates are allowed and insertion order is kept.** That's what sets it apart from a set.

### Hash (`hash`)

- **What:** A map of field → value, both strings, stored under one key. It's like a small object or row: `user:42 → {name: "Ana", age: "31"}`. Hashes are flat (values can't be nested hashes).
- **Internals:** a listpack while small (≤ 128 fields and each value ≤ 64 bytes by default), then a real hash table.
- **Per-field TTL:** since Redis 7.4, `HEXPIRE`/`HTTL` can expire individual fields.
- **Commands:** `HSET`, `HGET`, `HMGET`, `HGETALL`, `HDEL`, `HEXISTS`, `HLEN`, `HKEYS`, `HVALS`, `HINCRBY`, `HINCRBYFLOAT`, `HSETNX`, `HSCAN`, `HRANDFIELD`.
- **Used for:** objects (user profiles, sessions), grouping many small counters under one key.
- **In Rust:** `HashMap<Bytes, Bytes>`.

### Set (`set`)

- **What:** An unordered collection of **unique** strings. Membership test, add and remove are O(1). Supports set algebra.
- **Internals:** an *intset* (sorted array of integers) if every member is an integer and there are few of them, a listpack for small sets, otherwise a hash table.
- **Commands:** `SADD`, `SREM`, `SISMEMBER`, `SMISMEMBER`, `SMEMBERS`, `SCARD`, `SPOP`, `SRANDMEMBER`, `SINTER`, `SUNION`, `SDIFF` (plus `…STORE` variants), `SINTERCARD`, `SMOVE`, `SSCAN`.
- **Used for:** tags, unique visitors, "who liked this post", tracking seen IDs, mutual friends via `SINTER`.
- **In Rust:** `HashSet<Bytes>`.

### Sorted set (`zset`)

- **What:** Like a set, but each unique member has a floating-point **score**. Members are kept ordered by score; ties are broken lexicographically by member. You can query by rank (position) or by score range in O(log N).
- **Internals:** a listpack when small, otherwise a **skip list** (for ordering and range queries) combined with a hash table (member → score, for O(1) `ZSCORE`).
- **Commands:** `ZADD` (options `NX`, `XX`, `GT`, `LT`, `CH`, `INCR`), `ZREM`, `ZSCORE`, `ZINCRBY`, `ZCARD`, `ZCOUNT`, `ZRANK`, `ZREVRANK`, `ZRANGE` (with `BYSCORE`, `BYLEX`, `REV`, `LIMIT`, `WITHSCORES`), `ZRANGEBYSCORE`, `ZPOPMIN`, `ZPOPMAX`, `BZPOPMIN`, `ZUNIONSTORE`, `ZINTERSTORE`.
- **Score ranges:** `-inf`/`+inf` are allowed, and `(` makes a bound exclusive: `ZRANGEBYSCORE k (1 5`.
- **Used for:** leaderboards, priority queues, time-ordered indexes (score = timestamp), sliding-window rate limiters.
- **In Rust:** a `HashMap<Bytes, f64>` plus a `BTreeSet<(OrderedF64, Bytes)>`. You'll need an `Ord` wrapper for `f64`. Writing your own skip list is a good exercise. Note that a `BTreeSet` can't return "the rank of X" in O(log N); a skip list storing span widths can.

### Stream (`stream`)

- **What:** An append-only log of entries. Each entry has a unique, monotonically increasing ID `<ms-timestamp>-<sequence>` (e.g. `1526919030474-0`) and a small set of field/value pairs. Supports **consumer groups**: several consumers share the work, with per-message acknowledgement, similar to Kafka.
- **IDs:** `*` asks the server to generate one. `<ms>-*` auto-generates the sequence. Every new ID must be greater than the last; otherwise: `-ERR The ID specified in XADD is equal or smaller than the target stream top item`. `0-0` is never valid: `-ERR The ID specified in XADD must be greater than 0-0`.
- **Internals:** a radix tree whose nodes are listpacks of entries.
- **Commands:** `XADD`, `XRANGE`, `XREVRANGE`, `XREAD` (optionally `BLOCK ms`, with ID `$` meaning "only new entries"), `XLEN`, `XTRIM`, `XDEL`, `XGROUP CREATE`, `XREADGROUP`, `XACK`, `XPENDING`, `XCLAIM`, `XAUTOCLAIM`, `XINFO`.
- **Used for:** event sourcing, activity feeds, sensor data, durable job queues.
- **In Rust:** `BTreeMap<(u64, u64), Vec<(Bytes, Bytes)>>` makes range queries easy.

### Bitmap (not a separate type; `TYPE` says `string`)

- **What:** String commands that treat the value as an array of bits. `SETBIT key 7 1` sets bit 7 and grows the string as needed. Bit 0 is the most significant bit of the first byte.
- **Commands:** `SETBIT`, `GETBIT`, `BITCOUNT`, `BITPOS`, `BITOP AND|OR|XOR|NOT`, `BITFIELD` (read/write arbitrary-width integers at bit offsets).
- **Used for:** compact per-user flags ("user N was active today" means bit N is 1), bloom-filter-like structures.

### HyperLogLog (not a separate type; `TYPE` says `string`)

- **What:** A probabilistic structure that estimates the number of **distinct** elements (cardinality) using at most 12 KB, with about 0.81% standard error, no matter how many elements are added. Individual elements can't be retrieved.
- **Commands:** `PFADD`, `PFCOUNT`, `PFMERGE`.
- **Used for:** unique visitors per day or unique search queries, where an exact set would be too big.

### Geospatial index (not a separate type; `TYPE` says `zset`)

- **What:** Longitude/latitude points stored in a **sorted set**, with the score set to a 52-bit geohash of the position. Nearby points get nearby scores, so a radius search becomes a few score-range scans.
- **Commands:** `GEOADD`, `GEOPOS`, `GEODIST`, `GEOSEARCH`, `GEOSEARCHSTORE`, `GEOHASH`. Because it's a zset, `ZRANGE` and `ZREM` also work on it.
- **Used for:** "find drivers/stores within 5 km".

### Pub/Sub channels (not a key type)

- **What:** Fire-and-forget messaging. `SUBSCRIBE channel` puts the connection into a mode where it only receives pushed messages. `PUBLISH channel msg` delivers to everyone subscribed *at that moment*; nothing is stored. `PSUBSCRIBE` matches glob patterns.
- **Wire format:** subscribers receive arrays like `*3\r\n$7\r\nmessage\r\n$4\r\nnews\r\n$5\r\nhello\r\n`.
- **In Rust:** `tokio::sync::broadcast` per channel.

### Module and newer types (for reference)

- **JSON** (`ReJSON-RL`): nested JSON documents queried with JSONPath (`JSON.SET`, `JSON.GET`). Formerly part of Redis Stack; built in since Redis 8.
- **Vector set** (`vectorset`, Redis 8): stores embedding vectors and supports approximate nearest-neighbour similarity search (`VADD`, `VSIM`).
- **Probabilistic types** (Bloom/Cuckoo filters, Count-Min Sketch, Top-K, t-digest): `BF.ADD` and similar.
- **Time series** (`TS.ADD`, `TS.RANGE`).

### Big-O summary

| Type | Typical read | Typical write | Notes |
|---|---|---|---|
| String | O(1) | O(1) | `GETRANGE`/`SETRANGE` are O(len) |
| List | O(1) at ends, O(N) by index | O(1) at ends | `LRANGE` is O(S+N) |
| Hash | O(1) per field | O(1) per field | `HGETALL` is O(N) |
| Set | O(1) membership | O(1) | `SINTER` is O(N·M) worst case |
| Sorted set | O(log N) | O(log N) | Ranges are O(log N + M) |
| Stream | O(log N) range seek | O(1) append | |

---

## 4. Next 10 commands to implement

The order is deliberate. Each step adds one new concept:

| # | Command | New concept it forces |
|---|---|---|
| 1 | `DEL` | RESP Integer replies, variadic args |
| 2 | `EXISTS` | Shared "is this key alive?" expiry helper |
| 3 | `INCR` | RESP Error replies, parsing, overflow |
| 4 | `EXPIRE` | Changing a TTL on an existing key |
| 5 | `TTL` | Reading a TTL, sentinel return values |
| 6 | `TYPE` | Separating stored data types from RESP types |
| 7 | `RPUSH` | First collection type, `WRONGTYPE` errors |
| 8 | `LRANGE` | Index arithmetic, negative indexes, array replies |
| 9 | `HSET` | Second collection type, pair arguments |
| 10 | `HGET` | Nested lookups |

For every command:
- Command names are case-insensitive (`del`, `DEL`, `Del`).
- An **expired key behaves exactly like a missing key**.
- Wrong argument count: `-ERR wrong number of arguments for '<cmd>' command`.

---

### 1. `DEL key [key ...]`

Removes the given keys. Keys that don't exist are ignored.

**Reply:** Integer, the number of keys actually removed.

```
> SET a 1
+OK
> SET b 2
+OK
> DEL a b nosuchkey
:2
> DEL a
:0
```

Wire format:
```
→ *4\r\n$3\r\nDEL\r\n$1\r\na\r\n$1\r\nb\r\n$9\r\nnosuchkey\r\n
← :2\r\n
```

**Edge cases:**
- An expired-but-not-yet-cleaned-up key must **not** be counted.
- `DEL a a` with `a` present returns `:1`: the second delete finds nothing.
- Works on keys of any type.

**Learning point:** add `Value::Integer(i64)` and encode it as `:{n}\r\n`.

---

### 2. `EXISTS key [key ...]`

**Reply:** Integer, how many of the given keys exist.

```
> SET a 1
+OK
> EXISTS a
:1
> EXISTS a nosuch
:1
> EXISTS a a a
:3
```

**Edge cases:**
- Repeated keys are counted **each time** (`EXISTS a a` → `:2`). This differs from `DEL`.
- Expired keys count as non-existent. A good time to write a `fn get_live(...)` helper that every command uses.

---

### 3. `INCR key`

Parses the value as a signed 64-bit integer, adds 1, stores the result (still as a string) and returns it.

**Reply:** Integer, the new value.

```
> SET counter 10
+OK
> INCR counter
:11
> GET counter
$2\r\n11            ← still a string
> INCR newkey
:1                   ← missing key is treated as 0
> SET name ana
+OK
> INCR name
-ERR value is not an integer or out of range
> SET big 9223372036854775807
+OK
> INCR big
-ERR increment or decrement would overflow
```

**Edge cases:**
- Missing key: start from 0, so the result is 1.
- Strict parsing: `" 1"`, `"1 "`, `"+1"`, `"01"` and `"1.0"` are **all rejected** by real Redis. Rust's `str::parse::<i64>()` accepts `"+1"`, so be aware.
- Overflow: use `i64::checked_add`.
- **`INCR` keeps the key's existing TTL** (whereas `SET` clears it).
- On a list or hash: `-WRONGTYPE …`.

**Learning point:** RESP error replies. `Value::Err` already encodes correctly; the work is choosing the exact `ERR …` text for each failure.

**Easy follow-ups using the same code:** `DECR key`, `INCRBY key n`, `DECRBY key n`.

---

### 4. `EXPIRE key seconds [NX | XX | GT | LT]`

Sets a timeout on an existing key.

**Reply:** Integer, `1` if the timeout was set, `0` if the key doesn't exist or the condition wasn't met.

```
> SET a 1
+OK
> EXPIRE a 100
:1
> EXPIRE nosuch 100
:0
> EXPIRE a abc
-ERR value is not an integer or out of range
```

**Options** (Redis 7.0+; optional for you):
- `NX`: only set if the key currently has **no** expiry.
- `XX`: only set if the key **already has** an expiry.
- `GT`: only set if the new expiry is greater than the current one. A key without a TTL counts as infinite, so `GT` never applies to it.
- `LT`: only set if the new expiry is less than the current one. A key without a TTL counts as infinite, so `LT` always applies.

**Edge cases:**
- `seconds <= 0`: the key is **deleted immediately** and the reply is `:1`.
- Calling `EXPIRE` again overwrites the previous TTL.
- `SET` on the key afterwards clears the TTL (unless `SET … KEEPTTL`). Your current `SET` already does this because it replaces the whole `StoredValue`.

**Follow-ups:** `PEXPIRE` (ms), `EXPIREAT` / `PEXPIREAT` (absolute Unix timestamp), `PERSIST key` (remove the TTL; replies `1` if one was removed).

---

### 5. `TTL key`

**Reply:** Integer:
- remaining time to live in **seconds**
- `-1` if the key exists but has no expiry
- `-2` if the key doesn't exist (or has expired)

```
> SET a 1 EX 100
+OK
> TTL a
:100
> SET b 1
+OK
> TTL b
:-1
> TTL nosuch
:-2
```

**Edge cases:**
- Rounding: Redis computes `(remaining_ms + 500) / 1000`, i.e. it rounds to the nearest second, so right after `EX 100` it says `100`, not `99`.
- `PTTL key` is the same in milliseconds, without rounding.

---

### 6. `TYPE key`

**Reply:** Simple string, one of `string`, `list`, `set`, `zset`, `hash`, `stream`, or `none` if the key is missing.

```
> SET a 1
+OK
> TYPE a
+string
> RPUSH l x
:1
> TYPE l
+list
> TYPE nosuch
+none
```

**Learning point:** this is where `StoredValue { value: Value }` stops working. `Value` describes *how bytes go over the wire*, but `TYPE` needs to know *what kind of data is stored*. Introduce something like:

```rust
enum RedisData {
    String(Bytes),
    List(VecDeque<Bytes>),
    Hash(HashMap<Bytes, Bytes>),
    // Set, ZSet, Stream later
}
```

You can implement `TYPE` before `RPUSH` (only `string`/`none`) and extend it as you go.

---

### 7. `RPUSH key element [element ...]`

Appends one or more elements to the tail of the list. Creates the list if the key doesn't exist.

**Prerequisites if you do this before 1–6:** a `Value::Integer` for the reply (from `DEL`), a stored-data enum that can hold a list (from `TYPE`), and a `CommandArray` that can carry several elements (see [section 1](#1-where-you-are-now)).

**Reply:** Integer, the list length **after** the push.

```
> RPUSH l a
:1
> RPUSH l b c
:3
> SET s x
+OK
> RPUSH s y
-WRONGTYPE Operation against a key holding the wrong kind of value
```

**Edge cases:**
- At least one element is required: `RPUSH l` → wrong number of arguments.
- Pushing to an expired key creates a fresh list with **no** TTL.
- An existing list keeps its TTL.

**Twin command:** `LPUSH key element [element ...]` pushes each element to the **head** in order, so `LPUSH l a b c` produces `[c, b, a]`.

**Concurrency note:** with `DashMap`, `get_mut`/`entry` lock the shard. Do the read-check-modify inside a single `entry()` call so that two clients pushing concurrently can't lose an update.

---

### 8. `LRANGE key start stop`

Returns the elements from index `start` to `stop`, **both inclusive**.

**Reply:** Array of bulk strings. `*0\r\n` for a missing key or an empty range.

```
> RPUSH l a b c d e
:5
> LRANGE l 0 -1
*5  a b c d e
> LRANGE l 1 2
*2  b c
> LRANGE l -2 -1
*2  d e
> LRANGE l 3 100
*2  d e          ← stop is clamped to the end
> LRANGE l 4 1
*0               ← start > stop
> LRANGE l 10 20
*0               ← start beyond the end
> LRANGE nosuch 0 -1
*0
```

**Index normalisation algorithm** (this is what Redis does):

```
len = list length
if start < 0: start = len + start
if stop  < 0: stop  = len + stop
if start < 0: start = 0
if start > stop or start >= len: return empty
if stop >= len: stop = len - 1
return list[start..=stop]
```

**Edge cases:**
- `start`/`stop` that aren't integers: `-ERR value is not an integer or out of range`.
- On a string key: `WRONGTYPE`.

**Follow-ups:** `LLEN`, `LPOP key [count]`, `RPOP`, `LINDEX`. `BLPOP key timeout` is a great async exercise because it blocks until another client pushes; use `tokio::sync::Notify`.

---

### 9. `HSET key field value [field value ...]`

Sets one or more field/value pairs in a hash. Creates the hash if needed.

**Reply:** Integer, the number of fields that were **newly added**. Fields that already existed and were only updated don't count.

```
> HSET user:1 name ana age 31
:2
> HSET user:1 age 32 city oslo
:1            ← age was updated, only city is new
> HSET user:1 name
-ERR wrong number of arguments for 'hset' command
```

**Edge cases:**
- The argument count after the key must be even and at least 2.
- On a non-hash key: `WRONGTYPE`.
- The legacy `HMSET` is the same command but replies `+OK`.

---

### 10. `HGET key field`

**Reply:** Bulk string with the value, or `$-1\r\n` if the key or the field doesn't exist.

```
> HSET user:1 name ana
:1
> HGET user:1 name
$3\r\nana
> HGET user:1 nosuchfield
$-1
> HGET nosuchkey name
$-1
> SET s x
+OK
> HGET s name
-WRONGTYPE Operation against a key holding the wrong kind of value
```

**Follow-ups:** `HGETALL key` returns a flat array `[field1, value1, field2, value2, …]`, `*0` if the key is missing. Also `HDEL` (delete the key when the hash becomes empty), `HLEN`, `HEXISTS`, `HKEYS`, `HVALS`, `HINCRBY`.

---

## 5. Stretch goals

Roughly in increasing difficulty:

**Strings and keys**
- `MSET k v [k v …]` → `+OK`; `MGET k [k …]` → array with `$-1` for missing keys
- `APPEND key value` → new length; `STRLEN key` → length, `0` if missing
- `SET` options: `NX` (only if missing, otherwise nil reply), `XX` (only if present), `KEEPTTL`, `GET` (return the old value), `EXAT`/`PXAT`
- `KEYS pattern`: glob matching with `*`, `?`, `[abc]`, `[^a]`, `[a-z]`, `\` escape. Writing the matcher is good practice.
- `DBSIZE` → integer; `FLUSHALL` → `+OK`
- `RENAME src dst`, `COPY`, `RANDOMKEY`
- `SCAN cursor [MATCH pattern] [COUNT n]`: cursor-based iteration (reply `*2` = `[next_cursor, [keys…]]`, cursor `0` means done)

**Sets**
- `SADD`, `SREM`, `SMEMBERS`, `SISMEMBER`, `SCARD`, `SINTER`, `SUNION`

**Sorted sets**
- `ZADD`, `ZSCORE` (score as bulk string, e.g. `$3\r\n1.5`), `ZRANK`, `ZRANGE … WITHSCORES`, `ZCARD`, `ZREM`

**Server / infrastructure**
- **Active expiry:** Redis also expires keys in the background, about 10 times per second. It samples 20 random keys that have a TTL, deletes the expired ones and repeats if more than 25% were expired. A `tokio::spawn` with `tokio::time::interval` is enough for a simple version.
- `INFO`, `CONFIG GET dir`, `CLIENT SETNAME`, `COMMAND DOCS` (redis-cli sends this on connect; reply `*0`)
- `MULTI` / `EXEC` / `DISCARD`: transactions. Queue commands per connection, reply `+QUEUED` to each, then run them all and return an array of results.
- `SUBSCRIBE` / `PUBLISH`: pub/sub with `tokio::sync::broadcast`
- `BLPOP`: blocking pop with a timeout
- `XADD` / `XRANGE` / `XREAD`: streams
- **Persistence:** RDB snapshot (binary format) or AOF (append every write command, replay on startup; this is the simpler one)
- **Replication:** `REPLCONF`, `PSYNC`, `WAIT`

---

## 6. Testing without redis-cli

`redis-cli` isn't installed on this machine. If you get a connection before you go offline, `brew install redis` installs both `redis-cli` and a real `redis-server` to compare against.

Otherwise, `nc` is available. Your parser only understands RESP arrays, so you have to send properly framed commands. Add this helper to `~/.zshrc`:

```zsh
# usage: resp SET foo bar EX 10
resp() {
  local out="*$#\r\n"
  for arg in "$@"; do
    out+="\$${#arg}\r\n${arg}\r\n"
  done
  printf "$out" | nc -w 1 127.0.0.1 6379
}
```

```
$ resp PING
+PONG
$ resp SET foo bar
+OK
$ resp GET foo
$3
bar
```

That last output is what real Redis sends. Daikon currently prints `+bar` (see [section 1](#1-where-you-are-now)).

`${#arg}` counts characters, not bytes, so stick to ASCII. To see the exact bytes, including `\r\n`, pipe through `| od -c`.

**Rust integration tests** also work offline. Create `tests/commands.rs`, start the server on a random port, connect with `tokio::net::TcpStream`, write raw RESP and assert on the raw reply bytes. This is how you'll catch regressions in the 10 commands above.
