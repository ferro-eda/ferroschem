# 4. String Slices, `char_indices()`, and State-Machine Lexing in `ferroschem-format`

In **M1 — Teaching Phase 2**, we built the foundation of our xschem codec in [`crates/format/src/xschem/tokens.rs`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/tokens.rs):
- [`read_braced_string`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/tokens.rs#L25-L64) and [`write_braced_string`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/tokens.rs#L71-L82) (clean-room Rust port of xschem's `load_ascii_string` and `save_ascii_string` from `src/save.c`).
- [`parse_properties`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/tokens.rs#L100-L181) and [`format_properties`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/tokens.rs#L190-L224) (clean-room Rust port of xschem's `get_tok_value` state machine from `src/token.c`).

Let's unpack the four Rust patterns that make this lexer safe, fast, and zero-copy where possible.

---

## 1. Returning `(String, &str)` — Zero-Copy Cursor Advancing

Look at the signature of [`read_braced_string`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/tokens.rs#L25):

```rust
pub fn read_braced_string(input: &str) -> Result<(String, &str), FormatError>
```

Why does it take `input: &str` and return a tuple `(String, &str)`?
- The first element (`String`) is the **newly allocated, unescaped text** inside `{...}` (because we had to strip `\` escape characters and `\r` carriage returns).
- The second element (`&str`) is a **borrowed string slice pointing to the remainder of `input`** right after the closing `}`!
- Copying a `&str` slice in Rust is just copying a `(pointer, length)` pair (16 bytes on 64-bit machines)—it never copies the underlying file text. When our record parser reads `T {@name} 15 -18.75 ...`, `read_braced_string` hands back `("@name", " 15 -18.75 ...")` so the caller can immediately parse the numbers from `rest`.

---

## 2. UTF-8 Safety with `.char_indices()` and `.len_utf8()`

In C, strings are just arrays of bytes (`char *`), and advancing a pointer by `1` moves to the next byte.
In Rust, `&str` is guaranteed to be valid **UTF-8**, where a single Unicode character (`char`) can be 1, 2, 3, or 4 bytes long.

If you slice a `&str` at a byte index that lands in the middle of a multi-byte character, Rust will panic! To slice safely without ever panicking, [`read_braced_string`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/tokens.rs#L41-L58) uses `.char_indices()`:

```rust
let mut chars = after_open.char_indices();
let mut escape = false;

for (idx, ch) in &mut chars {
    if ch == '\r' {
        continue;
    }
    if !escape {
        if ch == '}' {
            let rest = &after_open[idx + ch.len_utf8()..];
            return Ok((out, rest));
        }
        if ch == '\\' {
            escape = true;
            continue;
        }
    }
    out.push(ch);
    escape = false;
}
```
- `after_open.char_indices()` yields `(idx, ch)` where `idx` is the exact **UTF-8 byte offset** where `ch` starts.
- `idx + ch.len_utf8()` is guaranteed to be the exact byte offset where the **next** character begins, so `&after_open[idx + ch.len_utf8()..]` is always a valid UTF-8 boundary!

---

## 3. Concise Pattern Matching with `let Some(...) = ... else` and `matches!`

Notice two modern Rust idioms in [`tokens.rs`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/tokens.rs):

### A. `let Some(after_open) = trimmed.strip_prefix('{') else { ... }`
`str::strip_prefix('{')` checks if `trimmed` starts with `'{'`.
- If it does, it returns `Some(&str)` with the `'{'` removed.
- If it doesn't, it returns `None`, and the `else` block immediately returns `Err(FormatError::Syntax(...))`. No nested `if`/`match` pyramid needed!

### B. The `matches!` Macro
Instead of chaining `ch == '\\' || ch == '{' || ch == '}'`, Rust's `matches!` macro lets us use `match`-style `|` patterns in a boolean expression:

```rust
if matches!(ch, '\\' | '{' | '}') {
    out.push('\\');
}
```

---

## 4. `std::mem::take` inside State Machines

In [`parse_properties`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/tokens.rs#L108-L162), we port xschem's `TOK_BEGIN` → `TOK_TOKEN` → `TOK_SEP` → `TOK_VALUE` state machine using a local Rust `enum State`.

When a `key=value` pair finishes at a whitespace separator, look at line 138:

```rust
props.set(std::mem::take(&mut key), std::mem::take(&mut val));
```
What does `std::mem::take(&mut key)` do?
- It **moves** the finished `String` out of `key` and into `props.set(...)`, and leaves `String::default()` (an empty, zero-allocation `String::new()`) in `key` ready for the next token!
- Compared to `.clone()` + `.clear()`, `std::mem::take` transfers ownership of the heap buffer directly into [`Properties`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/core/src/props.rs#L19-L21) without copying a single byte.
