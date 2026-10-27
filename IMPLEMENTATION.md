# How Rust-idiomatic daikon is

Review date: 2026-10-27. This covers style and idiom only, not bugs.

**Overall: about 6/10.** The low-level pieces are idiomatic: the parser, the borrowing, and the use of `let-else`. The weak point is the type design. Data shapes are checked at runtime and then `unwrap()`ed, where Rust code would normally let the types guarantee them. Default `cargo clippy` only reports dead code, so this is about design, not lint noise.

## What already looks idiomatic

- **The parser returns `(value, rest)` over `&[u8]`.** This is the zero-copy, nom-style shape Rust parsers use.
- **`let-else`** is used well (`set.rs`, `rpush.rs`, `parser/input.rs`), and so is `is_some_and`.
- **`thiserror` errors with `#[from]`** (`data/errors.rs`), so `?` converts `Utf8Error` and `ParseIntError` for you.
- **`FromStr` for `Command` and `Display` for `CommandName`** reuse standard traits instead of inventing new ones.
- **`Arc<str>` keys and `Bytes` storage** are good choices for shared data that doesn't change.
- **`std::sync::Mutex` inside tokio** is correct for short critical sections that never cross an `.await`, so you don't need tokio's async mutex.
- **Unit tests sit inline under `#[cfg(test)]`.**

## Where it's less idiomatic

### 1. "Parse, don't validate" (the biggest gap)

`CommandArray` carries `Option<Vec<..>>` keys and values, and a runtime `ArgShape` decides which ones are filled. Every handler then calls `.unwrap()` on them. The idiomatic approach is one enum of parsed commands, where each variant holds exactly the fields that command needs (for example: Set has a key, a value and an optional TTL). The parser would build that enum, the handlers would receive typed data, and the `unwrap()`s, `ArgShape` and `KeyType` would all go away. Lots of `unwrap()` on your own data is usually a sign the types aren't encoding an invariant.

### 2. Two error systems, mixed

- The library uses `anyhow`, while `DaikonError` and `errors::Result` are defined but never used. The usual split is `thiserror` inside the library and `anyhow` at the binary edge (`main`).
- Handlers report failures in two ways: some return `Ok(RESPValue::Err(..))`, others return `Err(..)`. Pick one meaning: either a domain error that gets converted into a RESP error at one place, or a reply value, not both.
- `use anyhow::Ok` and importing `RESPValue::Err` hide the prelude's `Ok` and `Err` (`echo.rs`, `set.rs`). Readers will trip over that.
- `main` could return a `Result` and use `?` instead of `expect`/`unwrap`.

### 3. Lock handling

`if let Ok(lock) = mutex.lock() … else "Internal error"` treats a poisoned lock as something to recover from. The convention is to call `.lock()` and `.expect(..)` it (or use `parking_lot`, whose lock can't be poisoned). The `dashmap` dependency is also unused.

### 4. Hand-rolled logging

The `debug!`/`trace!` macros backed by a global `AtomicBool` plus `eprintln!` duplicate what the `tracing` or `log` crates do, along with levels, filtering and spans.

### 5. Encoding API

- `RESPType::encode(&mut self)` doesn't need `&mut`; `&self` is enough.
- Each nested value allocates its own buffer, which is then copied byte by byte with `flat_map` and `put_u8`. The idiomatic shape is "encode into" a buffer the caller passes in.
- `format!(..).as_bytes()` builds a temporary `String`. Writing straight into the buffer with `write!` avoids it.
- A trait with a single implementation probably doesn't need to be a trait yet.

### 6. Binary safety in the types

RESP bulk strings are bytes, but `BulkString(String)` forces UTF-8. That leads to `String::from_utf8(..).unwrap()` in `to_resp_value`. Store `Bytes` and let the type say what the protocol says.

### 7. Naming and API conventions (Rust API Guidelines)

- Acronyms are written as one word: `RespValue`, `RespType`, not `RESPValue`.
- `NullBulkString()` is a variant with zero tuple fields. It should be a plain unit variant.
- In `parser/commands.rs`, names like `_ttl` and `_keys` are used, but a leading underscore means "unused". Reuse the plain name instead (shadowing).
- Use `b'\n'` instead of the magic number `10` plus a comment (`parser/input.rs:98`).
- Functions should accept the most general borrowed type: `parse_command_array` should take `&[u8]`, not `&BytesMut`. Also, `input.iter().as_slice()` is a roundabout way to borrow the bytes as a slice.

### 8. Small things that can be written more simply

- `del.rs`: `fold` with `acc.add(1)` and `use std::ops::Add` reads oddly. Use `+ 1`, or better, `filter` plus `count`.
- `Display for CommandName` formats `Debug` output and then lowercases it, which allocates and silently depends on how `Debug` prints. A `match` returning `&'static str` is the usual way. The `FromStr` arms also repeat the same struct literal for every command; the rules could be a method on `CommandName`.
- `parse_keys` returns a deque that is always empty, so the tuple serves no purpose.
- `Arc::<str>::from(key.to_owned())` clones the string for no reason; `.into()` on the owned `String` is enough.
- In `parse_command_array`, the six `ArgShape` arms repeat the same error branch, nested five levels deep. Early returns and a single helper would flatten it, and point 1 would remove the whole block anyway.

### 9. Visibility and construction

`main` builds `Storage` by filling its public `HashMap` fields directly. Deriving `Default` or adding a `new()`, and keeping the fields private, is the usual encapsulation. Visibility is also a mix of `pub` and `pub(crate)` with no clear rule.

## Where to start

Fixing point 1 gives the most for the effort, because it pulls a lot of points 2 and 8 along with it. Running `cargo clippy -- -W clippy::pedantic` will flag many of the smaller items.

## Not covered

- Bugs were left out on purpose (for example the `"/r/n"` separator and the `unwrap()` calls on client input).
- Not checked: whether `std::assert_matches` in the tests needs a nightly toolchain.
