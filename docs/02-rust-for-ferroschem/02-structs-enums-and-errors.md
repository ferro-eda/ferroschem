# 2. Structs, Enums, and Error Handling in Our Scaffold

Let's walk through the actual Rust code in our M0 scaffold so every keyword and symbol makes complete sense.

---

## 1. `struct` and `#[derive(...)]` — Modeling Data
Open [`crates/core/src/lib.rs`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/core/src/lib.rs):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
```
- `pub struct Point` defines a public data structure (accessible outside this crate) with two 64-bit floating-point fields (`f64`).
- `#[derive(...)]` asks the Rust compiler to automatically generate boilerplate code for this struct:
  - `Debug`: Lets us print a point for debugging with `println!("{:?}", pt)`.
  - `Clone`: Lets us explicitly duplicate a value with `pt.clone()`.
  - `Copy`: Only for tiny, simple types like two `f64` numbers—tells Rust that assigning `let b = a;` can just copy the 16 bytes on the stack instead of "moving" ownership.
  - `PartialEq`: Lets us compare two points with `pt1 == pt2`.
  - `Default`: Gives us `Point::default()`, which sets `x: 0.0, y: 0.0`.
  - `Serialize, Deserialize` (from `serde`): Lets us convert `Point` to and from JSON automatically!

---

## 2. `Option<T>` — Replacing `null` Safely
Look at [`Properties::get`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/core/src/props.rs#L31-L37) and [`Wire::label`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/core/src/document.rs#L99-L105):

```rust
impl Properties {
    /// Look up a property value by key (e.g. `props.get("name")` or `props.get("value")`).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.value.as_str())
    }
}

impl Wire {
    /// Net label attached to this wire (`lab=`), if any.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.props.get("lab")
    }
}
```
Rust does **not** have `null` or `nil`. If a value might be absent (for example, an unnamed wire has no `lab=` net label), we wrap its type in `Option<T>`, which is a built-in Rust `enum` with two variants:
- `Some(T)` — a value is present, e.g. `Some("VDD")`.
- `None` — no value is present.

Because `Option<T>` is a distinct type from `T`, the compiler forces you to handle the `None` case—eliminating null-pointer crashes at compile time.

The `#[must_use]` attribute above `label` asks Rust to warn if a caller throws away the returned `Option`:

```rust
wire.label(); // warning: the returned Option is unused

if let Some(label) = wire.label() {
    println!("Wire label: {label}");
}
```

It applies to the method's **return value**, not to the `Wire` object. It does not mean every wire must call `label()`. The warning helps catch cases where the caller may have meant to inspect or use the label but forgot. Warnings can be configured as errors by a project, but `#[must_use]` itself is not a requirement to call the method.

---

## 3. `Result<T, E>` and Why We Ban `unwrap()`
Look at the top of [`crates/core/src/lib.rs`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/core/src/lib.rs) and [`crates/format/src/lib.rs`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/lib.rs):

```rust
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
```
In EDA software, **a crash destroys the user's unsaved circuit design**. In Rust, functions that can fail return a `Result<T, E>`:
- `Ok(value)` when the operation succeeds (producing a `T`).
- `Err(error)` when the operation fails (producing an error `E`).

Beginners often write `.unwrap()` on a `Result` or `Option`, which means *"give me the value inside `Ok`/`Some`, or crash the whole program if it's `Err`/`None`"*. By adding `#![deny(clippy::unwrap_used, ...)]`, our build fails if anyone (human or AI) ever writes `.unwrap()`.

Instead, we define structured errors with `thiserror` in [`crates/format/src/lib.rs`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/lib.rs):
```rust
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("not supported yet: {0}")]
    Unsupported(String),
}

pub fn parse_schematic(_text: &str) -> Result<Schematic, FormatError> {
    Err(FormatError::Unsupported("schematic parsing".into()))
}
```
And when calling a function that returns a `Result`, we use the **`?` operator**:
- Writing `let sch = parse_schematic(input)?;` means: *"If `parse_schematic` returned `Ok(s)`, put `s` in `sch` and keep going; if it returned `Err(e)`, immediately return that error to my caller."*
