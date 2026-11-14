# 5. Lifetimes (`'a`), Turbofish Parsing (`.parse::<T>()`), and Recursive Descent in `parser.rs`

In **M1 — Teaching Phase 3A**, we built the xschem `.sym` and `.sch` record parser in [`crates/format/src/xschem/parser.rs`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/parser.rs).

Let's examine the four Rust patterns that make [`parser.rs`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/parser.rs) concise, zero-copy on numeric tokens, and immune to crashes on malformed files.

---

## 1. Struct Lifetimes: `struct Cursor<'a>`

In [Chapter 4](./04-strings-and-lexing.md), we saw how [`read_braced_string`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/tokens.rs#L21) took `input: &str` and returned `(String, &str)`—the parsed string plus the remaining unparsed slice.

When parsing an entire record with 7 fields in a row—like a `T` (Text) record:
```text
T {@#0:net_name} 10 -28.75 0 0 0.15 0.15 {layer=15 hide=instance}
```
manually threading `(val, rest)` through 7 calls would be noisy. Instead, we wrap the remaining `&str` slice inside a tiny `Cursor<'a>` struct ([`parser.rs`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/parser.rs#L21-L32)):

```rust
struct Cursor<'a> {
    rest: &'a str,
    records_read: usize,
}

impl<'a> Cursor<'a> {
    const fn new(input: &'a str) -> Self {
        Self {
            rest: input,
            records_read: 0,
        }
    }
    // ...
}
```

### What does `'a` mean?
- In Rust, every reference (`&str`) borrows data owned by someone else.
- When a `struct` holds a reference (`rest: &'a str`), Rust requires a **lifetime parameter** (`'a`, pronounced *"tick-a"*) on the struct declaration (`struct Cursor<'a>`).
- `'a` is simply a compile-time label saying: *"This `Cursor` borrows a string slice from an input buffer that lives for at least `'a`. The `Cursor` (and any `&'a str` word slices it hands out) can never outlive that original input string."*
- Because `Cursor<'a>` only holds a 16-byte `(pointer, length)` fat pointer (`&'a str`) plus a `usize` counter, advancing the cursor (`self.rest = &self.rest[end..]`) is **zero-copy**—zero heap allocations are needed to tokenize numbers, rotations, or tags!

---

## 2. The Turbofish (`.parse::<T>()`) and `.map_err(...)`

Look at how [`Cursor::read_f64`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/parser.rs#L76-L88) parses a coordinate from a `&str` word:

```rust
fn read_f64(&mut self) -> Result<f64, FormatError> {
    let word = self.read_word()?;
    let val = word
        .parse::<f64>()
        .map_err(|_| FormatError::Syntax(format!("invalid floating-point number '{word}'")))?;
    if !val.is_finite() {
        return Err(FormatError::Syntax(format!(
            "non-finite coordinate '{word}' is not allowed"
        )));
    }
    Ok(if val == 0.0 { 0.0 } else { val })
}
```

1. **`.parse::<f64>()` (The "Turbofish" `::<T>`):**
   - `str::parse` is a generic method in the Rust standard library that can parse a string into *any* type implementing the `FromStr` trait (`f64`, `u32`, `usize`, `u16`, `IpAddr`, etc.).
   - Writing `::<f64>` tells the compiler which target type we want to parse into.
2. **`.map_err(|_| ...)?`:**
   - `word.parse::<f64>()` returns `Result<f64, ParseFloatError>`.
   - Because our function returns `Result<f64, FormatError>`, `.map_err(|_| ...)` converts the standard library's `ParseFloatError` into our descriptive [`FormatError::Syntax`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/lib.rs#L52), and `?` returns early if parsing failed.
3. **Hostile-Input Guard (`!val.is_finite()`):**
   - In Rust, `"NaN".parse::<f64>()` and `"inf".parse::<f64>()` succeed! Checking `!val.is_finite()` at the parser boundary guarantees that `NaN` or `±Infinity` can never enter [`Point`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/core/src/geometry.rs#L11) and corrupt bounding-box or rendering math downstream.

---

## 3. Rust 2024 `let`-Chains (`if cond && let Some(x) = ...`)

Look at how [`RawRecordSet::into_symbol`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/parser.rs#L174-L181) handles legacy pre-1.2 xschem symbol files (which stored symbol properties in `G {...}` when `K` was empty):

```rust
if self.props.is_empty()
    && let Some(vhdl) = self.netlist_blocks.get("vhdl")
    && vhdl.contains("type=")
    && let Ok(legacy_props) = parse_properties(vhdl)
{
    self.props = legacy_props;
    self.netlist_blocks.remove("vhdl");
}
```

Because FerroSchem uses **Rust Edition 2024**, we can chain regular boolean conditions (`self.props.is_empty()`, `vhdl.contains("type=")`) and pattern-matching `let` bindings (`let Some(vhdl) = ...`, `let Ok(legacy_props) = ...`) with `&&` in a single flat `if` expression instead of nesting four levels of `if` / `match` blocks.

---

## 4. Bounded Recursive Descent for `[` ... `]` Embedded Symbols

In xschem, a component instance `C` in a `.sch` file can optionally be followed by a `[` ... `]` block containing an inline `.sym` definition.

Instead of duplicating the record parser for embedded symbols, [`parse_records`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/parser.rs#L220) takes `depth: usize` (`0` at the top level, `> 0` inside `[...]`) and calls itself recursively when `'['` is encountered ([`parser.rs`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/parser.rs#L431-L449)):

```rust
'[' => {
    let next_depth = depth + 1;
    if next_depth > ParseLimits::DEFAULT.max_nesting_depth {
        return Err(FormatError::InputTooLarge {
            size: next_depth,
            max: ParseLimits::DEFAULT.max_nesting_depth,
        });
    }
    let Some(last_inst) = out.instances.last_mut() else {
        return Err(FormatError::Syntax(
            "encountered '[' embedded symbol block without a preceding 'C' instance"
                .into(),
        ));
    };
    cursor.skip_to_end_of_line();
    let embedded_records = parse_records(cursor, next_depth)?;
    last_inst.embedded_symbol = Some(embedded_records.into_symbol());
    continue;
}
```

When the recursive call sees `']'` (`if depth > 0`), it returns `Ok(embedded_records)`, which is converted via `.into_symbol()` and attached directly to `last_inst.embedded_symbol`.

### Two Security Rules for Recursive Parsers on Hostile Input
1. **Shared Document-Wide Counters Live on `Cursor`, Not Local Variables:** If `records_read` were a local variable inside `parse_records`, each recursive `[` frame would reset the counter back to `0`. Keeping `records_read` on `Cursor<'a>` ensures [`ParseLimits::max_records`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/lib.rs#L26) counts across the entire document.
2. **Always Bound Recursion Depth (`max_nesting_depth`):** Without an explicit depth limit, an attacker-crafted file with thousands of nested `C ...\n[\nC ...\n[\n...` lines would exhaust the call stack and crash the process (violating FerroSchem's *Never crash* rule). Checking `next_depth > ParseLimits::DEFAULT.max_nesting_depth` turns unbounded recursion into a clean [`FormatError::InputTooLarge`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/lib.rs#L54).
