# Daikon backlog

Bugs, fixes and refactors found so far, grouped by the part of the code they touch so you can finish one area before moving to the next. Each `###` entry is meant to become one GitHub issue.

Locations are given as file plus function or type, never line numbers, so they stay valid while the code changes. The descriptions reflect the working tree as of 2026-11-13, including the uncommitted byte-migration changes.

---

## How to read this file

### Issue format

```
### [ ] <ID> <title>
Labels: <comma-separated labels>
<body: context, location, done-when, refs>
```

- Change `[ ]` to `[x]` when you finish an item. Once it's an issue, add the issue number after the ID, e.g. `PROTO-1 (#12)`. `scripts/backlog_issues.py --apply` does this for you.
- IDs are stable. Use them in commit messages (`PROTO-1: cap array length`) so you can find the history later.
- `Depends on:` means do that item first. Usually the other item changes the same code.

### Labels

Every issue gets one label from each of these groups. Labels marked `(optional)` are only added when they apply.

| Group | Labels | Meaning |
|---|---|---|
| Priority | `p0` | The build is broken, or **any client can crash the connection or the server** |
| | `p1` | Implemented commands behave differently from Redis |
| | `p2` | Design or refactor work that gets more expensive the longer you wait |
| | `p3` | Polish, performance, nice-to-have |
| Type | `bug`, `refactor`, `feature`, `test`, `chore`, `docs` | |
| Area | `area:connection` | `src/lib.rs`, `src/main.rs` |
| | `area:protocol` | `src/parser/input.rs`: RESP framing |
| | `area:commands-parser` | `src/parser/commands.rs`, `src/data/commands.rs` |
| | `area:commands` | `src/commands/*.rs` |
| | `area:lists` | `src/data/redis_data.rs`, list logic in `rpush.rs` and `lrange.rs` |
| | `area:errors` | `src/data/errors.rs` and how errors reach the client |
| | `area:storage` | `src/storage.rs` |
| | `area:tests`, `area:docs`, `area:deps` | |
| Size | `size:S` (< 1h), `size:M` (an evening), `size:L` (several sessions) | Rough effort |
| Extra (optional) | `client-crash` | A client can trigger a panic with one request |
| | `redis-compat` | Reply text or behavior differs from real Redis |

### Suggested order

1. **WIP-1**: get the build compiling again. Everything else assumes it compiles.
2. The `client-crash` items: PROTO-1 to PROTO-4, CMDP-1, CONN-1.
3. **ERR-1**, then **CMDP-2** to **CMDP-5**. That's error replies plus `LRANGE` parsing.
4. The `area:lists` items, which finish `LRANGE`.
5. **TEST-1**, to lock in the above.
6. Everything else, by priority.

---

## 0. Work in progress

### [ ] WIP-1 (#5) (#5) Finish the migration to byte-based keys and values
Labels: `p0`, `refactor`, `area:commands-parser`, `area:storage`, `area:commands`, `size:L`

**Context:** Redis keys and values are binary-safe, so they should be bytes, not `str`/`String`. The migration is half done: `BulkString` holds `Bytes` in the parser, but `RespValue` still has a lifetime parameter, keys are still `Arc<str>`, and `cargo build` currently fails with about 25 errors.

**What's left:**
- **`RespValue`:** remove the `<'a>` lifetime and have `BulkString` own `Bytes`. A borrowed reply can't escape the storage mutex guard, so the lifetime can't work for replies like `LRANGE`.
- **`Command`:** remove the `<'a>` lifetime (`src/data/commands.rs`).
- **Keys:** switch `Arc<str>` to `Bytes` in `Storage` (both maps), in the `Command` variants, in every `invoke` signature, and in the rpush tests. `Bytes` is already reference-counted, so don't wrap it in `Arc`.
- **`Bytes` has no `to_uppercase` or `parse`:**
  - Command names and `EX`/`PX` (`parse_command_array` and `parse_ttl` in `src/parser/commands.rs`): compare as bytes, ASCII case-insensitive.
  - The TTL number (`parse_ttl`): parse via UTF-8, then an integer.
- **Logging:** log keys and values with `{:?}` (prints `b"..."`) instead of `{}`.
- **`RedisData::to_resp_value`** (`src/data/redis_data.rs`): drop the `String::from_utf8(...).unwrap()`.

**Done when:** `cargo build` and `cargo test` pass, and the code no longer contains `Arc<str>` or a lifetime on `RespValue`/`Command`.

**Refs:** REDIS_NOTES.md §3 "String" (binary-safe); RESP bulk strings: https://redis.io/docs/latest/develop/reference/protocol-spec/#bulk-strings

---

## 1. Protocol parser: `src/parser/input.rs`

Any client can trigger every item in this section with a single request, so do these first.

### [ ] PROTO-1 (#6) (#6) Cap array and bulk-string lengths before allocating
Labels: `p0`, `bug`, `area:protocol`, `client-crash`, `size:S`

**Context:** In the array branch of `parse_data_bytes`, `VecDeque::with_capacity(length)` uses the array length straight from the client. Sending `*9999999999999\r\n` makes the allocation panic or abort the whole process, which takes down every client, not just the one connection. Bulk lengths have no upper bound either: `$999999999\r\n` makes the server wait and buffer forever.

**Fix:** reject lengths above a limit before allocating anything. Redis allows at most 1024×1024 elements per command and `proto-max-bulk-len` (512 MB by default) per bulk string, and replies `-ERR Protocol error: invalid multibulk length` / `invalid bulk length`.

**Done when:** an oversized `*N` or `$N` gets an error reply, the server stays up, and a test covers each case.

**Refs:** https://redis.io/docs/latest/operate/oss_and_stack/management/config-file/ (`proto-max-bulk-len`)

### [ ] PROTO-2 (#7) (#7) `unwrap()` on UTF-8 in length prefixes
Labels: `p0`, `bug`, `area:protocol`, `client-crash`, `size:S`

**Context:** In `parse_data_bytes`, `str::from_utf8(chars.as_slice()).unwrap()` in both the bulk-string and array branches panics if the bytes after `$` or `*` aren't valid UTF-8, e.g. `$\xff\r\n`.

**Fix:** return `MalformedRequest` instead of unwrapping. Lengths are ASCII digits, so parsing them directly as digits is also fine.

**Done when:** the input above produces an error reply, and a test covers it.

### [ ] PROTO-3 (#8) (#8) Slicing past the end of a partially received bulk string
Labels: `p0`, `bug`, `area:protocol`, `client-crash`, `size:S`

**Context:** the bulk-string branch of `parse_data_bytes` checks `rest.len() < length`, then slices `&rest[(length + 2)..]`. If the body has arrived but the trailing `\r\n` hasn't (a TCP read split at the wrong place), the slice panics. The code also never checks that those 2 bytes really are `\r\n`.

**Fix:** require `length + 2` bytes, return `IncompleteRequest` if fewer have arrived, and return `MalformedRequest` if the 2 bytes aren't `\r\n`.

**Done when:** `$3\r\nfoo` and `$3\r\nfoo\r` return incomplete, `$3\r\nfooXY` returns an error, and each has a test.

### [ ] PROTO-4 (#9) (#9) `assert_ne!` in `next_resp_separator` panics on client input
Labels: `p0`, `bug`, `area:protocol`, `client-crash`, `size:S`

**Context:** `next_resp_separator` ends with an `assert_ne!` that checks the byte after `\r\n` isn't `\r`. Any request containing `\r\n\r` panics the connection task. Assertions are for invariants of your own code, not for checking input.

**Fix:** remove the assertion, or turn it into a `MalformedRequest`.

### [ ] PROTO-5 (#10) (#10) Bulk strings containing `\r` are rejected
Labels: `p1`, `bug`, `area:protocol`, `redis-compat`, `size:S`

**Context:** In the bulk-string branch of `parse_data_bytes`, `if content.contains(&b'\r')` rejects valid values. A bulk string is length-prefixed so that it can contain any bytes, including `\r\n`. `SET k "a\r\nb"` must work.

**Fix:** delete the check. The length prefix already gives the boundaries.

**Depends on:** PROTO-3, which validates the trailing CRLF so that framing errors are still caught.

### [ ] PROTO-6 (#11) (#11) Inline commands (`PING\r\n` without `*`/`$`)
Labels: `p3`, `feature`, `area:protocol`, `size:M`

**Context:** real Redis accepts space-separated plain-text commands, which is what `nc` and `telnet` send. Right now they fall into `_ => Ok(None)` and get a generic error. This is optional, but it makes manual testing easier.

**Refs:** REDIS_NOTES.md §2 "Inline commands".

---

## 2. Connection handling: `src/lib.rs`, `src/main.rs`

### [ ] CONN-1 (#12) (#12) `execute_command(...).unwrap()` kills the connection on any command error
Labels: `p0`, `bug`, `area:connection`, `client-crash`, `size:S`

**Context:** in `handle_connection` (`src/lib.rs`), the result of `execute_command` is unwrapped. Any `Err(StorageError)` panics the task, including normal errors like `WrongType` from `RPUSH` on a string key. The client sees the connection drop instead of a `-WRONGTYPE ...` reply.

**Fix:** turn the error into a RESP error reply and keep the connection open.

**Depends on:** ERR-1, which provides the conversion.

### [ ] CONN-2 (#13) (#13) Pipelining: commands after the first in one read are dropped
Labels: `p1`, `bug`, `area:connection`, `area:protocol`, `size:M`

**Context:** `parse_command_array` ignores the `rest` that `parse_data_bytes` returns, and `handle_connection` calls `buf.clear()` after every command and every parse error. Two commands in one TCP write (`PING` + `PING`) get one reply. Clients and benchmarks such as `redis-benchmark -P` pipeline by default.

**Fix:** after parsing a command, drop only the bytes it used (e.g. `BytesMut::advance`) and keep parsing until the buffer is empty or incomplete. On `IncompleteRequest`, keep the buffer and read more, as happens now.

**Done when:** a test that writes `PING` twice in one write receives `+PONG` twice.

**Refs:** https://redis.io/docs/latest/develop/use/pipelining/

### [ ] CONN-3 (#14) (#14) `accept().unwrap()` kills the server on transient errors
Labels: `p2`, `bug`, `area:connection`, `size:S`

**Context:** `main` (`src/main.rs`) calls `listener.accept().await.unwrap()`. `accept` fails temporarily under load (e.g. running out of file descriptors). Right now that panics `main` and stops the server.

**Fix:** log the error and keep looping. Optionally sleep briefly first so the loop doesn't spin.

### [ ] CONN-4 (#15) (#15) Logging is always on and runs inside the lock
Labels: `p3`, `chore`, `area:connection`, `size:S`

**Context:** `DEBUG` and `TRACE` are hard-coded to `true` in `src/lib.rs`, so every request writes several lines to stderr, including full values. `set.rs` and `del.rs` log while holding the storage mutex, which slows down every other client.

**Fix:** add a `--log-level` (or `-v`) flag to the existing `clap` args, and log after the lock is released.

---

## 3. Errors: `src/data/errors.rs`

### [ ] ERR-1 (#16) (#16) Convert errors to RESP replies with Redis's exact text in one place
Labels: `p1`, `bug`, `area:errors`, `area:connection`, `redis-compat`, `size:M`

**Context:** every parse error currently replies `-Unknown command` (the parse-error branch of `handle_connection`), whether the problem was a wrong argument count, a non-integer, or malformed input. Clients read the first word of an error as its kind, so they all see `Unknown`. Error texts are also inconsistent: `WrongType` displays only `WRONGTYPE`.

**Fix:** add one conversion from `ParsingError` and `StorageError` to an error `RespValue`, used by `handle_connection` for both parse and execute errors. It must produce the exact texts below.

| Error | Exact reply |
|---|---|
| Unknown command | `-ERR unknown command 'foo', with args beginning with: 'a' 'b'` |
| Wrong argument count (`InvalidCommand` already formats this) | `-ERR wrong number of arguments for 'get' command` |
| Malformed or unknown option | `-ERR syntax error` |
| Bad integer | `-ERR value is not an integer or out of range` |
| Wrong type | `-WRONGTYPE Operation against a key holding the wrong kind of value` |

**Done when:** every error in REDIS_NOTES.md §2 "Common error messages" can be produced, and CONN-1 uses the conversion.

**Refs:** REDIS_NOTES.md §2.

### [ ] ERR-2 (#17) (#17) Commands handle errors two different ways
Labels: `p2`, `refactor`, `area:errors`, `area:commands`, `size:S`

**Context:**
- A poisoned lock is reported two ways:
  - `get.rs`, `del.rs` and `set.rs` return `Ok(RespValue::Err("Internal error occured"))`.
  - `rpush.rs` and `lrange.rs` return `Err(StorageError::InaccessibleStorage)`.
- `set.rs` and `echo.rs` also return `Ok(Err(...))` for bad input.

**Fix:** decide that commands return `Err(...)` for every failure and that only ERR-1 produces RESP errors.

**Depends on:** ERR-1.

### [ ] ERR-3 (#18) (#18) Reconsider `BadEncoding` after the byte migration
Labels: `p3`, `refactor`, `area:errors`, `size:S`

**Context:** once keys and values are bytes, UTF-8 matters only for numeric arguments, and Redis replies "value is not an integer or out of range" for those. `BadEncoding` may no longer be needed.

**Depends on:** WIP-1.

---

## 4. Command parsing: `src/parser/commands.rs`, `src/data/commands.rs`

### [ ] CMDP-1 (#19) (#19) `SET k v EX <huge>` panics on `Instant` overflow
Labels: `p0`, `bug`, `area:commands-parser`, `area:commands`, `client-crash`, `size:S`

**Context:** `parse_ttl` accepts any `u64`, then `invoke` in `set.rs` computes `Instant::now() + ttl`, which panics on overflow. `SET k v EX 18446744073709551615` crashes the connection task.

**Fix:** use a checked add and reply `-ERR invalid expire time in 'set' command` on overflow. That also covers CMDP-6.

### [ ] CMDP-2 (#20) (#20) `LRANGE` can't be called: missing from `CommandName::from_str`
Labels: `p0`, `bug`, `area:commands-parser`, `size:S`

**Context:** `CommandName::from_str` (`src/data/commands.rs`) has no `"LRANGE"` arm, so `LRANGE` gets `UnknownCommand` and the parser and command code behind it never run.

**Done when:** `LRANGE` reaches `commands::lrange::invoke`.

### [ ] CMDP-3 (#21) (#21) `parse_index` unwraps the integer parse
Labels: `p0`, `bug`, `area:commands-parser`, `client-crash`, `size:S`

**Context:** `parse_index` (`src/parser/commands.rs`) unwraps the result of `parse`, so `LRANGE k abc 1` panics. `ParsingError::NotAnInt` has `#[from] ParseIntError`, so `?` works there. You don't need `unwrap`.

### [ ] CMDP-4 (#22) (#22) The `stop < start` check rejects `LRANGE k 0 -1`
Labels: `p1`, `bug`, `area:commands-parser`, `redis-compat`, `size:S`

**Context:** The `CommandName::Lrange` branch of `parse_command_array` rejects the request when `stop < start`. Negative indexes count from the end, so the most common call, `LRANGE k 0 -1`, is rejected. The order only makes sense after resolving indexes against the list length, and Redis returns an empty array for an inverted range, not an error.

**Fix:** delete the check. The normalization algorithm in LIST-1 handles it.

### [ ] CMDP-5 (#23) (#23) Use `i64` for list indexes, not `i16`
Labels: `p1`, `bug`, `area:commands-parser`, `area:lists`, `redis-compat`, `size:S`

**Context:** `parse_index` returns `i16` and `Command::Lrange` stores `i16`, so `LRANGE k 0 40000` is rejected even though the list may be much longer. Redis uses signed 64-bit indexes.

**Fix:** use `i64` end to end, and clamp when converting to `usize` (see LIST-1).

### [ ] CMDP-6 (#24) (#24) Validate `SET` options the way Redis does
Labels: `p1`, `bug`, `area:commands-parser`, `redis-compat`, `size:M`

**Context:** REDIS_NOTES.md §1 marks this done, but these still don't match Redis:

| Input | Current behavior | Redis reply |
|---|---|---|
| `SET k v EX 0`, or a negative value | Accepted | `-ERR invalid expire time in 'set' command` |
| `SET k v FOO 10` | `parse_ttl` returns `None`, leftover arguments give "wrong number of arguments" | `-ERR syntax error` |
| `SET k v EX abc` | | `-ERR value is not an integer or out of range` |

Check that the last row still holds after WIP-1.

**Refs:** https://redis.io/docs/latest/commands/set/

### [ ] CMDP-7 (#25) (#25) Use one strict integer parser for every numeric argument
Labels: `p2`, `refactor`, `area:commands-parser`, `redis-compat`, `size:S`

**Context:** `parse_index` and `parse_ttl` parse integers separately, and later `INCR`, `EXPIRE` and `LPOP count` will need the same thing. Rust's `parse` accepts `"+1"`, which Redis rejects. Redis also rejects `" 1"`, `"01"` and `"1.0"`.

**Fix:** write one helper that parses bytes into an integer with Redis's rules, and use it everywhere.

**Refs:** REDIS_NOTES.md §4.3 "INCR" edge cases.

### [ ] CMDP-8 (#26) (#26) Match command names on bytes, case-insensitively
Labels: `p2`, `refactor`, `area:commands-parser`, `size:S`

**Context:** the command name is uppercased into a new `String` and matched with `FromStr`. Comparing bytes ASCII-case-insensitively avoids the allocation and the UTF-8 step. Part of this is required for WIP-1 to compile.

---

## 5. Commands: `src/commands/`

### [ ] CMD-1 (#27) (#27) `ECHO` replies with a simple string instead of a bulk string
Labels: `p1`, `bug`, `area:commands`, `redis-compat`, `size:S`

**Context:** `invoke` in `src/commands/echo.rs` turns the bulk string into `RespValue::SimpleString`. REDIS_NOTES.md §1 marks this done, but it's still there. A simple string can't hold `\r` or `\n`, and `ECHO ""` must reply `$0\r\n\r\n`.

**Done when:** `ECHO hi` replies `$2\r\nhi\r\n`.

### [ ] CMD-2 (#28) (#28) `GET` on a list replies with an empty error instead of `WRONGTYPE`
Labels: `p1`, `bug`, `area:commands`, `redis-compat`, `size:S`

**Context:** `RedisData::to_resp_value` returns `RespValue::Err("")` for anything that isn't a string, so the client receives `-\r\n`.

**Fix:** `GET` should check the type and return `StorageError::WrongType`. Consider removing `to_resp_value` from `RedisData`, since "how to reply" depends on the command, not just the type.

---

## 6. Lists: `src/data/redis_data.rs`, `src/commands/lrange.rs`, `src/commands/rpush.rs`

### [ ] LIST-1 (#29) (#29) Finish `LRANGE` with Redis's index normalization
Labels: `p1`, `feature`, `area:lists`, `area:commands`, `size:M`

**Context:** `lrange.rs` is unfinished and doesn't compile. Bugs already in it:
- **Wrong condition for `to`:** it checks `start < 0` instead of `stop < 0` when computing `to`.
- **Underflow panic:** `*length - start.unsigned_abs()` (and the same for `stop`) panics in debug builds, or wraps around in release builds, when a negative index reaches past the head (`LRANGE k -100 -1` on a 5-element list). Redis clamps to 0.
- **No missing-key reply:** a missing key must reply `*0\r\n`. The function currently ends with `NullBulkString`.

**Fix:** follow the normalization algorithm in REDIS_NOTES.md §4.8 step by step, then take `skip`/`take` from the list iterator (LIST-3).

**Depends on:** CMDP-2, CMDP-4, CMDP-5, LIST-3.

**Done when:** every example in REDIS_NOTES.md §4.8 gives the documented reply.

**Refs:** https://redis.io/docs/latest/commands/lrange/

### [ ] LIST-2 (#30) (#30) `list_index_to_list_pack_index` divides by the number of packs
Labels: `p1`, `bug`, `area:lists`, `size:S`

**Context:** `list_index_to_list_pack_index` (`src/data/redis_data.rs`) computes `i / list.len()`, where `list.len()` is the number of packs, not the entries per pack. Even with `LP_MAX_SIZE`, the math assumes every pack except the last is full. That's true with only `RPUSH`, but stops being true once `LPUSH`, pops or `LREM` exist.

**Fix:** to find index `i`, skip whole packs by subtracting each pack's stored `size` until `i` falls inside one. That's O(number of packs) and works whatever the pack sizes are. Or delete the function if `LRANGE` only uses the iterator.

### [ ] LIST-3 (#31) (#31) Iterator over list entries
Labels: `p1`, `feature`, `area:lists`, `size:M`

**Context:** `ListPack::iter` returns `self.data.iter()`, which yields individual raw bytes, length prefixes included, not entries. `RedisListIter` mixes two designs: it has offset fields as if it implemented `next()` itself, plus an `iter()` method using `flat_map`.

**Fix:**
- **Pack iterator:** borrows the pack's bytes. Each `next()` reads the 4-byte LE length (`split_first_chunk`), slices out the entry, skips the trailing length, and yields `&[u8]`.
- **List iterator:** `flat_map` over the packs, in an `iter()` method on the list type (LIST-4). Delete `RedisListIter` and its offsets.
- **Reverse iteration (optional):** implement `DoubleEndedIterator` by reading the trailing length (`split_last_chunk`), so that `.rev()` works. That makes negative indexes and `RPOP` cheap.
- **Errors:** read with checked slicing, so a corrupted length returns `None` or an error instead of panicking.

**Done when:** iterating over a list built with `RPUSH` gives back exactly the pushed values, in order, across several packs, and a test covers it.

### [ ] LIST-4 (#32) (#32) Move list logic into its own struct inside `RedisData`
Labels: `p2`, `refactor`, `area:lists`, `size:M`

**Context:** `RedisData::List { packs, length }` exposes its fields. `rpush.rs` updates `packs` and `length` separately (filling the last pack, `pack()`, the `length` bookkeeping), and `lrange.rs` reads them directly. Any command can get them out of sync, and every new list command would repeat the same logic.

**Fix:**
- Make the variant hold a list struct with private fields. Methods: `push_back`, `len`, `iter`, and later `push_front`/`pop_*`.
- Add an accessor on `RedisData` that returns the list or `WrongType`, so that each command does one call plus `?`.
- Keep `RedisData` an enum, not a trait: the set of types is closed, and commands need the concrete type.

**Done when:** no command touches `packs` or `length` directly.

### [ ] LIST-5 (#33) (#33) `value.len() as u32` truncates silently
Labels: `p3`, `bug`, `area:lists`, `size:S`

**Context:** `ListPack::push` casts the length to `u32`. A value over 4 GB would get a wrong prefix and corrupt the pack. Capping bulk length in PROTO-1 makes this impossible in practice, but a `try_from` or a debug assertion documents the assumption.

**Depends on:** PROTO-1.

### [ ] LIST-6 (#34) (#34) More compact listpack encoding
Labels: `p3`, `feature`, `area:lists`, `size:L`

**Context:** every entry carries 8 bytes of length prefixes (4 before, 4 after). That's more than the data for short values like `"one"`. Packs are also capped by entry count (`LP_MAX_SIZE = 10`), while Redis caps them by bytes (`list-max-listpack-size -2` means 8 KB).

**Fix:** use a variable-width entry header and back-length like real listpacks, and limit packs by size in bytes. Don't change the iterator's interface (LIST-3), so nothing else has to change.

**Refs:** listpack spec: https://github.com/antirez/listpack/blob/master/listpack.md; REDIS_NOTES.md §3 "List".

---

## 7. Storage: `src/storage.rs`

### [ ] STOR-1 (#35) (#35) Active expiry
Labels: `p3`, `feature`, `area:storage`, `size:M`

**Context:** keys expire only when something reads them (`get_live`/`get_mut_live`). A key with a TTL that's never read again stays in memory forever.

**Fix:** a background task (`tokio::time::interval`) samples keys from `expiry` about 10 times per second and deletes the expired ones. Hold the lock briefly each time.

**Refs:** REDIS_NOTES.md §5 "Active expiry".

---

## 8. Tests: `tests/`

### [ ] TEST-1 (#36) (#36) Integration tests over TCP
Labels: `p1`, `test`, `area:tests`, `size:M`

**Context:** there are only unit tests (`input.rs`, `rpush.rs`). Nothing exercises the full path: socket, parser, command, reply. Every `client-crash` item above is a one-request test case.

**Fix:** create `tests/` that starts the server on a random port, writes raw RESP with `tokio::net::TcpStream`, and asserts on the raw reply bytes. Start with these cases:
- Every `client-crash` input
- Pipelined `PING PING`
- A command split across two writes
- `LRANGE k 0 -1`

**Refs:** REDIS_NOTES.md §6.

### [ ] TEST-2 (#37) (#37) Fuzz the RESP parser
Labels: `p3`, `test`, `area:tests`, `area:protocol`, `size:M`

**Context:** the parser takes untrusted bytes and has already had several panics (PROTO-1 to PROTO-4). `cargo fuzz` on `parse_data_bytes` would find the rest.

**Depends on:** PROTO-1 to PROTO-4, so the fuzzer doesn't keep rediscovering them.

---

## 9. Dependencies and docs

### [ ] DEPS-1 (#38) (#38) Remove unused dependencies
Labels: `p3`, `chore`, `area:deps`, `size:S`

**Context:** `dashmap` and `anyhow` are in `Cargo.toml`, but nothing in `src/` uses them anymore: storage is a `Mutex<HashMap>`, and errors use `thiserror`.

### [ ] DOCS-1 (#39) (#39) Update REDIS_NOTES.md §1 and §7
Labels: `p2`, `docs`, `area:docs`, `size:S`

**Context:** several parts of the notes no longer match the code:
- **"Code layout":** mentions `CommandArray`, `src/data/types.rs`, `Value` and `DashMap`, none of which exist anymore.
- **"Implemented commands":** missing `DEL` and `RPUSH`, and `LRANGE` is in progress.
- **Checklist:**
  - ECHO and the SET options are marked done but aren't (see CMD-1 and CMDP-6).
  - "CRLF check" is unchecked but already fixed.
  - "Error replies", "unwraps" and "Pipelining" are tracked here now as ERR-1, CONN-1 and CONN-2.
- **§7 "What this means for Daikon":** describes `Arc<DashMap>`, but the code now uses `Mutex<HashMap>`.

**Fix:** make §1 a short status summary and point to this file for open work.

---

## 10. Commands still to implement

In the order given in REDIS_NOTES.md §4, where each command and its edge cases are described. Each one can become its own `feature` issue labeled `area:commands`. Collection commands also get `area:lists` or a new area label.

- [ ] CMDS-1 (#40) (#40) `EXISTS key [key ...]` (`p2`, `size:S`)
- [ ] CMDS-2 (#41) (#41) `INCR` / `DECR` / `INCRBY` / `DECRBY` (`p2`, `size:M`, depends on CMDP-7)
- [ ] CMDS-3 (#42) (#42) `EXPIRE` / `PEXPIRE` / `PERSIST` (`p2`, `size:M`)
- [ ] CMDS-4 (#43) (#43) `TTL` / `PTTL` (`p2`, `size:S`)
- [ ] CMDS-5 (#44) (#44) `TYPE` (`p2`, `size:S`)
- [ ] CMDS-6 (#45) (#45) `LPUSH` (`p2`, `size:M`, depends on LIST-4)
- [ ] CMDS-7 (#46) (#46) `LLEN`, `LPOP`, `RPOP`, `LINDEX` (`p2`, `size:M`, depends on LIST-3 and LIST-4; popping the last element must delete the key)
- [ ] CMDS-8 (#47) (#47) `HSET` / `HGET` / `HGETALL` (`p2`, `size:M`)
- [ ] CMDS-9 (#48) (#48) Stretch goals: see REDIS_NOTES.md §5 (`p3`)
