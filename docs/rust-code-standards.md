# High-Performance Rust Coding Standards

> If it feels fast at small scale but collapses under load, the problem is design, not the compiler.

Rust enables writing code that's simultaneously safe and fast through zero-cost abstractions, but achieving peak performance requires understanding specific patterns. **The most impactful optimizations come from treating allocations, syscalls, hashing, and scheduler interactions as first-class design constraints** — with proper patterns, Rust code regularly matches or exceeds C++ performance while maintaining memory safety guarantees.

This guide combines authoritative best practices from the Rust Performance Book, core team members, and real-world benchmarks with enforceable standards and practical patterns. Each section provides both the underlying performance rationale and clear MUST/SHOULD/AVOID directives for immediate application.

## Operating Principles

* **MUST** profile before and after changes (CPU + allocations) and record results in PRs
* **MUST** use release builds for performance evaluation
* **MUST** treat allocations, syscalls, hashing, and scheduler interactions as design constraints, not afterthoughts
* **SHOULD** identify "hot paths" (see criteria in Section 15) and apply rigorous standards to them

---

## Quick Reference: Do / Don't Table

| Topic              | Prefer                                | Avoid                                |
| ------------------ | ------------------------------------- | ------------------------------------ |
| Ownership          | `&T`, `&str`, `Cow`, `Arc<str>`       | `.clone()` to "make it compile"      |
| Iteration          | Chained iterators, lazy pipelines     | `collect::<Vec<_>>()` then transform |
| Collections        | `with_capacity`, `SmallVec` where apt | Repeated `push` without sizing       |
| Hashing            | FxHash/AHash for trusted hot paths    | Default hasher for all internal data |
| I/O                | `BufReader`/`BufWriter`, streaming    | Unbuffered I/O, intermediate Strings |
| Errors             | `thiserror` enums + `?`               | `Result<T, String>` + `format!`      |
| Logging            | `tracing` with structured fields      | Unstructured `println!/log!` blobs   |
| Concurrency        | Scoped threads, bounded channels      | Copying large `String`s into tasks   |
| Dispatch           | Static generics in hot paths          | `dyn Trait` where not needed         |
| Build              | LTO + reduced codegen units           | Default release profile              |
| Bench discipline   | Criterion median/p95 + alloc counts   | "Feels faster" anecdotes             |

---

## 1. Ownership & Borrowing

### Understanding the Performance Cliff

The single most impactful performance decision in Rust code involves choosing between borrowing and cloning. **Borrowing is a compile-time zero-cost abstraction taking less than 1 nanosecond, while cloning a Vec with 10,000 elements requires 40-50 microseconds** — a difference of 40,000x. This isn't just about raw speed; cloning allocates heap memory, fragments your address space, and thrashes CPU caches.

### Standards

* **MUST** pass references (`&T`, `&str`, `&[u8]`) through call chains when lifetimes allow
* **MUST** design APIs to accept `&str`/`&[u8]`/`&T` where possible, not owned types
* **SHOULD** use `Cow<'a, str>` when mutation or owned fallback is occasionally required
* **SHOULD** use `Arc<T>`/`Arc<str>` for cross-thread sharing of read-only data
* **SHOULD** use scoped threads (`crossbeam::scope`, `rayon::scope`) to borrow instead of Arc when threads have bounded lifetimes
* **AVOID** `.clone()` to satisfy ownership in hot paths; restructure APIs to accept borrows
* **AVOID** returning owned `String`/`Vec<T>` unless you own the allocation contract

### Anti-pattern: Cloning to satisfy the borrow checker

```rust
fn filter_active_users(users: &[User]) -> Vec<User> {
    users.iter()
        .filter(|u| u.is_active)
        .cloned()  // Clones every user - expensive!
        .collect()
}

fn process_data(data: Vec<String>) -> usize {
    data.len()  // Takes ownership but only needs to read
}

let data = vec!["a".to_string(), "b".to_string()];
let len = process_data(data.clone());  // Expensive clone just to keep data alive
println!("{:?}", data);
```

### Prefer: Borrow by default

```rust
fn filter_active_users(users: &[User]) -> Vec<&User> {
    users.iter()
        .filter(|u| u.is_active)
        .collect()  // Returns references, zero copying
}

fn process_data(data: &[String]) -> usize {
    data.len()  // Borrows instead of taking ownership
}

let data = vec!["a".to_string(), "b".to_string()];
let len = process_data(&data);  // Zero-cost borrow
println!("{:?}", data);  // data still available
```

### When to Clone Strategically

**Clone when you genuinely need independent ownership:**
- Sending data across thread boundaries without Arc
- Returning modified data without affecting the original
- Intentionally simplifying complex lifetime relationships

For read-only operations, borrowing should be your default choice. When you do need owned data, use `Cow` (Clone-on-Write) to defer cloning until modification actually occurs — this pattern achieves 6-7x performance improvements in read-heavy workloads where only 10% of calls require modification.

### Reference Counting: The Middle Ground

Reference counting with `Rc` and `Arc` occupies a middle ground with 2-3 nanosecond overhead for `Rc::clone()` and 10-15 nanoseconds for `Arc::clone()` due to atomic operations. **Start with owned data and add reference counting only when profiling reveals genuine shared ownership requirements** — many architectures that initially seem to need `Arc` can be redesigned around message passing with zero synchronization overhead.

### Pattern: Share Across Threads Without Copying

```rust
use std::sync::Arc;
use std::thread;

// Anti-pattern: clone strings into each thread
fn spawn_tasks_bad(names: Vec<String>) {
    for n in names {
        let n_clone = n.clone();  // Expensive full copy per thread
        thread::spawn(move || {
            println!("{}", n_clone);
        });
    }
}

// Prefer: share with Arc
fn spawn_tasks_good(names: Vec<String>) {
    // One allocation per string; cheap Arc clones per thread
    let shared: Vec<Arc<str>> = names.into_iter()
        .map(|s| Arc::<str>::from(s))
        .collect();

    for s in shared {
        let s2 = Arc::clone(&s);
        thread::spawn(move || {
            println!("{}", s2);
        });
    }
}

// Best: use scoped threads to borrow (zero Arc overhead)
fn spawn_tasks_scoped(names: &[String]) {
    crossbeam::scope(|s| {
        for name in names {
            s.spawn(move |_| {
                println!("{}", name);  // Direct borrow, no Arc needed
            });
        }
    }).unwrap();
}
```

### Pattern: Flexible Borrow-or-Own Parameters

```rust
use std::borrow::Cow;

fn normalize<'a>(s: impl Into<Cow<'a, str>>) -> Cow<'a, str> {
    let mut s = s.into();
    if needs_mutation(&s) {
        let mut owned = s.into_owned();
        owned.make_ascii_lowercase();
        return Cow::Owned(owned);
    }
    s  // Return borrowed if no mutation needed
}

// Works with &str (zero cost), String (takes ownership), or Cow
let result1 = normalize("HELLO");           // Borrows
let result2 = normalize(String::from("World"));  // Owns
```

---

## 2. Pre-allocating Collections

### Understanding Reallocation Costs

A Vec starting with default capacity undergoes approximately 14 reallocations to reach 10,000 elements, with each reallocation involving allocating new memory, copying all existing elements, and deallocating old memory. **Pre-sizing with `Vec::with_capacity()` eliminates these reallocations entirely, yielding 2-3x speedups** for known collection sizes.

### Standards

* **MUST** call `Vec::with_capacity(estimated)` when building a Vec with known or estimatable size
* **MUST** use `HashMap::with_capacity()` and `HashSet::with_capacity()` for large collections
* **SHOULD** use `String::with_capacity()` when building strings in loops or concatenating many parts
* **SHOULD** call `reserve()` before bulk operations to ensure capacity for additional elements
* **SHOULD** reuse collections in loops with `clear()` to keep capacity
* **MUST** consider `SmallVec`, `arrayvec`, or `tinyvec` for collections with small, fixed upper bounds

### Anti-pattern: Repeated Reallocations

```rust
// Multiple reallocations: capacity grows 0 → 4 → 8 → 16 → 32...
let mut numbers = Vec::new();
for i in 0..10000 {
    numbers.push(i);  
}

// HashMap rehashes multiple times as load factor exceeds threshold
let mut map = HashMap::new();
for i in 0..1000 {
    map.insert(i, i * 2);  
}

// String reallocates during concatenation
let mut result = String::new();
for _ in 0..1000 {
    result.push_str("some text");  
}

// Creating new Vec each iteration wastes allocation
for batch in batches {
    let mut buffer = Vec::new();  // Allocates every time
    for item in batch {
        buffer.push(process(item));
    }
    write_to_disk(&buffer);
}
```

### Prefer: Pre-allocated Collections

```rust
// Single allocation, no reallocations
let mut numbers = Vec::with_capacity(10000);
for i in 0..10000 {
    numbers.push(i);
}

// HashMap pre-sizes accounting for load factor (~1.25x needed capacity)
let mut map = HashMap::with_capacity(1000);
for i in 0..1000 {
    map.insert(i, i * 2);
}

// String pre-allocated to exact size needed
let mut result = String::with_capacity(9000);  // 1000 iterations × 9 bytes
for _ in 0..1000 {
    result.push_str("some text");
}

// Reuse allocation across iterations (2-10x improvement in tight loops)
let mut buffer = Vec::with_capacity(1000);
for batch in batches {
    buffer.clear();  // Keeps capacity, resets length to 0
    for item in batch {
        buffer.push(process(item));
    }
    write_to_disk(&buffer);
}
```

### Pattern: Efficient String Building

```rust
// Pre-allocated string building (fastest)
let total_len: usize = strings.iter().map(|s| s.len()).sum();
let mut result = String::with_capacity(total_len);
for s in &strings {
    result.push_str(s);  // Single allocation, no reallocations
}

// Using join() for collections (convenient and fast)
let result = vec!["hello", "world"].join(" ");

// Reusing buffer with write! macro (avoids format! allocation)
use std::fmt::Write;
let mut buffer = String::with_capacity(1024);
for item in items {
    buffer.clear();
    write!(&mut buffer, "Item: {}", item).unwrap();
    log(&buffer);
}
```

---

## 3. Iterator Chains & Lazy Evaluation

### Understanding Zero-Cost Abstractions

Rust iterators represent one of the language's most powerful zero-cost abstractions. **Iterator chains with `map()`, `filter()`, and similar adapters compile to machine code identical to hand-written loops** — sometimes even faster due to better optimization opportunities. The compiler performs iterator fusion, combining multiple operations into a single pass with no intermediate collections.

### Standards

* **MUST** avoid `collect::<Vec<_>>()` followed by immediate `map`/`filter`/`sum`
* **MUST** keep transformations in a single lazy iterator pipeline
* **MUST** only materialize (`collect`) at API boundaries or when random access is required
* **SHOULD** return `impl Iterator<Item = &T>` or `impl Iterator<Item = T>` from producers to keep consumers lazy
* **SHOULD** use terminal operations (`any()`, `all()`, `find()`, `sum()`) directly without collecting
* **AVOID** `.iter().cloned().collect()` when you could use `.into_iter().collect()`

### Anti-pattern: Unnecessary Intermediate Collections

```rust
// Creating unnecessary intermediate collections
let result: Vec<_> = data.iter()
    .filter(|&&x| x > 0)
    .collect::<Vec<_>>()      // Allocates intermediate Vec
    .iter()
    .map(|&x| x * 2)
    .collect::<Vec<_>>()      // Allocates final Vec
    .iter()
    .sum::<i32>();

// Collecting when you only need existence check
let has_match = data.iter()
    .filter(|&&x| x > 100)
    .collect::<Vec<_>>()
    .len() > 0;               // Wastes allocation

// Cloning when not needed
data.iter().cloned().for_each(|x| println!("{}", x));
```

### Prefer: Lazy Iterator Chains

```rust
// Single iterator chain, operations fused into one pass
let result: i32 = data.iter()
    .filter(|&&x| x > 0)
    .map(|&x| x * 2)
    .sum();  // No intermediate allocations

// Use terminal operations directly without collecting
let has_match = data.iter().any(|&x| x > 100);  // Short-circuits
let count = data.iter().filter(pred).count();
let first = data.iter().find(pred);

// Only clone when you actually need owned data
data.iter().for_each(|x| println!("{}", x));

// Move instead of copy for owned types
let owned: Vec<String> = data.into_iter()
    .filter(|s| s.len() > 5)
    .collect();  // Moves Strings, doesn't copy
```

### Pattern: Lazy Transformed Stream

```rust
fn process(nums: &[u32]) -> u64 {
    nums.iter()
        .copied()              // Cheap copy for u32
        .map(u64::from)        // Type conversion
        .filter(|n| n % 3 == 0)
        .take(50_000)          // Early termination
        .sum()                 // All fused into single loop
}

// Return lazy iterator instead of Vec
fn active_users(users: &[User]) -> impl Iterator<Item = &User> {
    users.iter().filter(|u| u.is_active)
    // Caller can further transform, collect, or iterate
}
```

### Understanding Iterator Types

Use `iter()` for reading (returns `&T`), `iter_mut()` for in-place modification (returns `&mut T`), and `into_iter()` when consuming the collection (returns `T`). Iterator methods like `any()`, `all()`, and `find()` provide short-circuit evaluation, processing only as many elements as necessary — this can yield 10-1000x improvements when the sought condition appears early.

---

## 4. Collection Selection

### Understanding Collection Performance Characteristics

Collection choice dramatically impacts performance because different data structures optimize for different operations. **Vec provides O(1) indexed access and cache-friendly contiguous memory, while VecDeque enables O(1) operations at both ends using a ring buffer at the cost of slightly worse cache locality**. HashMap delivers O(1) average-case lookups with ~20-50 nanosecond access times, whereas BTreeMap guarantees O(log n) operations taking ~100-200 nanoseconds but provides sorted iteration and efficient range queries.

### Standards

* **MUST** use Vec as default for sequences (best cache performance)
* **SHOULD** switch to VecDeque when you need efficient `push_front()` or `pop_front()` operations
* **AVOID** `Vec::remove(0)` / front-draining in FIFO loops; use `VecDeque` or a head index + periodic compaction
* **SHOULD** use HashMap for unsorted key-value storage with fast lookup
* **SHOULD** use BTreeMap when you need sorted keys or range queries
* **MUST** consider `SmallVec` for collections typically containing < 16-32 elements (10-100x speedup)
* **SHOULD** use `ArrayVec` for fixed-capacity, no-heap-allocation requirements

### Anti-pattern: Wrong Collection for Access Pattern

```rust
// Using Vec when you need double-ended operations
let mut queue = Vec::new();
queue.push(item);           // O(1) - fine
queue.remove(0);            // O(n) - shifts all elements!

// Using HashMap when you need sorted keys
let mut map = HashMap::new();
map.insert(3, "three");
map.insert(1, "one");
// Iteration order is unpredictable
for (k, v) in &map {
    println!("{}: {}", k, v);  // Could be 3: three, then 1: one
}
```

### Prefer: Collection Matches Usage

```rust
// VecDeque for queue operations
let mut queue = VecDeque::new();
queue.push_back(item);     // O(1)
queue.pop_front();         // O(1) - no shifting!

// BTreeMap when you need sorted keys
let mut map = BTreeMap::new();
map.insert(3, "three");
map.insert(1, "one");
// Iteration always in sorted key order
for (k, v) in &map {
    println!("{}: {}", k, v);  // Always: 1: one, then 3: three
}
// Efficient range queries
let range = map.range(1..5);
```

### Pattern: Small Stack-Allocated Collections

```rust
use smallvec::{SmallVec, smallvec};

// Stack-allocated until 8 elements (10-100x faster for small collections)
let mut vec: SmallVec<[i32; 8]> = smallvec![1, 2, 3];
vec.push(4);  // Still on stack
// Automatically moves to heap if exceeding 8 elements

// ArrayVec for guaranteed no-heap (panics on overflow)
use arrayvec::ArrayVec;
let mut vec: ArrayVec<i32, 8> = ArrayVec::new();
vec.push(1);  // Stack only, panics if > 8
```

---

## 5. String Handling

### Understanding String Types

Strings in Rust come in two fundamental forms: `String` (owned, heap-allocated, growable) and `&str` (borrowed string slice, no allocation). **Function parameters should almost always accept `&str` rather than `String`** because `&str` works with string literals, borrowed Strings, and string slices, while accepting `String` forces callers to allocate when they might only have a `&str`.

### Standards

* **MUST** accept `&str` for function parameters that only read strings
* **MUST** return `String` only when the function creates new string data
* **SHOULD** use `impl Into<Cow<'a, str>>` for flexible APIs that may or may not modify
* **SHOULD** use `String::with_capacity()` when building strings in loops
* **SHOULD** prefer `push_str()` over `+` operator in loops
* **AVOID** `format!()` in hot paths (allocates each call); use `write!()` into a buffer instead
* **AVOID** `.to_string()` when `.to_owned()` or `.into()` is clearer about allocation

### Anti-pattern: Forcing Callers to Allocate

```rust
fn greet(name: String) -> String {  // Forces callers to own a String
    format!("Hello, {}!", name)
}

let greeting = greet("Alice".to_string());  // Unnecessary allocation

fn process_text(text: String) {  // Takes ownership unnecessarily
    println!("{}", text);
}

// Concatenation with + in loop (reallocates every iteration)
let mut result = String::new();
for s in &strings {
    result = result + s;  // New allocation each time!
}
```

### Prefer: Accept Borrowed Strings

```rust
fn greet(name: &str) -> String {  // Accepts any string type
    format!("Hello, {}!", name)
}

let greeting = greet("Alice");  // No extra allocation

fn process_text(text: &str) {  // Borrows, caller keeps ownership
    println!("{}", text);
}

// Pre-allocated string building (10x faster)
let total_len: usize = strings.iter().map(|s| s.len()).sum();
let mut result = String::with_capacity(total_len);
for s in &strings {
    result.push_str(s);  // Single allocation, no reallocations
}

// Using join() for collections
let result = vec!["hello", "world"].join(" ");
```

### Pattern: Avoid format! in Hot Paths

```rust
use std::fmt::Write;

// Anti-pattern: format! allocates every call
for i in 0..10000 {
    let msg = format!("Processing item {}", i);  // 10000 allocations
    log(&msg);
}

// Preferred: reuse buffer with write!
let mut buffer = String::with_capacity(50);
for i in 0..10000 {
    buffer.clear();
    write!(&mut buffer, "Processing item {}", i).unwrap();
    log(&buffer);  // Single allocation, reused 10000 times
}
```

---

## 6. Error Handling

### Understanding Zero-Cost Error Handling

Rust's `Result<T, E>` and `Option<T>` types force explicit error handling at compile time while maintaining zero runtime overhead. **Result compiles to a tagged union that fits in registers, generating machine code identical to C's error codes** but with type-safe, exhaustive handling enforced by the compiler.

### Standards

* **MUST** use small `enum` error types per module with `thiserror`
* **MUST** use `?` operator for propagation and early returns
* **MUST** be explicit when intentionally discarding a `#[must_use]` value (`Result`, `Option`, etc.) via `let _ = ...;` or `drop(...)` (leave a short comment explaining why ignoring is correct)
* **SHOULD** attach context at boundary layers (`anyhow`/`eyre` for application edges)
* **SHOULD** use `#[cold]` and `#[inline(never)]` on error construction paths
* **AVOID** `format!()` in error types on hot paths; structure first, format on log
* **AVOID** `Result<T, String>` or string-based errors
* **AVOID** `unwrap()` in library code; propagate errors to callers
* **AVOID** broad `#[allow(unused_must_use)]` (it hides real bugs); prefer local intent at the callsite

### Anti-pattern: String-Based Errors

```rust
fn load_config(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|e| format!("read failed: {}", e))  // Allocates on error
}

// Using unwrap() in library code
pub fn get_config_value(key: &str) -> String {
    config.get(key).unwrap()  // Panics on missing key!
}
```

### Prefer: Typed Errors with thiserror

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppErr {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("parse error: {0}")]
    Parse(#[from] std::num::ParseIntError),
    
    #[error("invalid config")]
    InvalidCfg,
    
    #[error("not found: {0}")]
    NotFound(String),
}

fn load_config(path: &str) -> Result<String, AppErr> {
    let s = std::fs::read_to_string(path)?;  // Auto-converts via From
    if s.trim().is_empty() {
        return Err(AppErr::InvalidCfg);
    }
    Ok(s)
}

// Propagate errors to caller
pub fn get_config_value(key: &str) -> Result<String, AppErr> {
    config.get(key)
        .ok_or_else(|| AppErr::NotFound(key.to_string()))
}

// Or provide sensible defaults
pub fn get_timeout() -> u64 {
    config.get("timeout")
        .and_then(|s| s.parse().ok())
        .unwrap_or(30)  // Safe default value
}
```


### Pattern: Explicitly Discard Non-Critical `Result`/`Option` (`let _ =` / `drop`)

Rust warns on unused `#[must_use]` values to prevent accidentally swallowing failures. When the outcome is genuinely non-critical (best-effort cleanup, fire-and-forget metrics, shutdown cleanup), be explicit and local about it.

* Use `let _ = expr;` when you want to evaluate `expr` and discard the value.
* Use `drop(value);` when you want to end a value's lifetime **now** (often to release memory or to shorten a borrow).

```rust
use std::fs;

fn best_effort_cleanup() {
    // Best-effort cleanup: missing file / permissions are acceptable here.
    let _ = fs::remove_file("/tmp/temp_cache.dat");
}

fn release_memory_early() {
    // Large temporary allocation we want to free before the next phase.
    let tmp = vec![0u8; 8 * 1024 * 1024];

    // ... use tmp ...

    drop(tmp); // Explicit early drop (also ends borrows)
    // tmp is moved/dropped here; using it again is a compile error.
}
```

For readability, a scoped block is often the cleanest "early drop":

```rust
fn do_work_in_phases() {
    {
        let tmp = vec![0u8; 8 * 1024 * 1024];
        // ... phase 1 ...
        let _ = tmp.len(); // stand-in for real work
    } // tmp dropped here

    // ... phase 2 (runs with lower memory pressure) ...
}
```

### Pattern: Boundary Error Formatting

```rust
// Library/core layer: typed errors
#[derive(thiserror::Error, Debug)]
pub enum DbErr {
    #[error("connect failed")]
    Connect(#[from] sqlx::Error),
    #[error("not found")]
    NotFound,
}

pub async fn fetch_user(db: &Pool, id: i64) -> Result<User, DbErr> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id)
        .fetch_optional(db)
        .await?
        .ok_or(DbErr::NotFound)
}

// Boundary layer (HTTP/controller): format once with context
pub async fn get_user_handler(db: &Pool, id: i64) -> impl IntoResponse {
    match fetch_user(db, id).await {
        Ok(user) => Json(user).into_response(),
        Err(e) => {
            tracing::warn!(error = ?e, user_id = id, "fetch_user failed");
            StatusCode::NOT_FOUND.into_response()
        }
    }
}
```

---

## 7. Structured Logging & Observability

### Understanding Structured Logging Benefits

Structured logging with the `tracing` crate provides zero-cost spans and events with strongly-typed fields that enable powerful querying and analysis in production. **String concatenation in logs destroys queryability and adds allocation overhead** in hot paths.

### Standards

* **MUST** use `tracing` crate for spans and events; prefer fields over string blobs
* **MUST** emit error variants and key IDs as fields (`error = %err, user_id = %id`)
* **SHOULD** use `debug!`/`trace!` for hot loop logging; these compile to no-ops when level is disabled
* **SHOULD** attach spans around operations for automatic timing and context propagation
* **AVOID** logging success paths at `info` level in hot loops
* **AVOID** `format!()` or string concatenation in log messages; use fields
* **AVOID** unstructured `println!()` or `log!()` macros

### Anti-pattern: Unstructured Logging

```rust
// String concatenation destroys queryability
println!("User {} logged in from {}", user_id, ip_addr);

// format! allocates even when log level is disabled
log::info!("{}", format!("Processing {}", item_id));

// Success logging in hot path at info level
for item in items {
    process(item);
    log::info!("Processed item {}", item.id);  // Floods logs
}
```

### Prefer: Structured Fields with tracing

```rust
use tracing::{info, debug, warn, error, instrument};

// Structured fields enable querying: "show all user_id=123 events"
tracing::info!(
    user_id = %user_id,
    ip_addr = %ip_addr,
    "user logged in"
);

// Lazy evaluation - format only if level is enabled
tracing::debug!(item_id = %item_id, "processing item");

// Hot path uses debug/trace, not info
for item in items {
    process(item);
    tracing::debug!(item_id = %item.id, "processed");  // Compiles to no-op if disabled
}

// Spans provide automatic timing and context
#[instrument(skip(db), fields(user_id = %user_id))]
async fn fetch_user_data(db: &Pool, user_id: i64) -> Result<User> {
    // Automatic logging of entry, exit, duration, and errors
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(db)
        .await?;
    Ok(user)
}

// Error logging with structured fields
match operation().await {
    Ok(result) => tracing::info!(count = result.len(), "operation succeeded"),
    Err(e) => tracing::error!(error = %e, context = ?ctx, "operation failed"),
}
```

---

## 8. Static vs Dynamic Dispatch

### Understanding Dispatch Performance

Rust provides two mechanisms for polymorphism: static dispatch through generics and dynamic dispatch through trait objects. **Static dispatch generates specialized code for each concrete type with zero runtime overhead and full inlining**, while **dynamic dispatch uses vtables with ~3-5 nanosecond overhead per call and prevents cross-boundary inlining**.

### Standards

* **MUST** use static dispatch (generics or `impl Trait`) for hot paths called millions of times
* **SHOULD** use `impl Trait` for cleaner syntax when returning single concrete types
* **SHOULD** use dynamic dispatch (`Box<dyn Trait>`, `&dyn Trait`) when you need heterogeneous collections or runtime polymorphism
* **AVOID** trait objects where static dispatch would work
* **AVOID** premature use of `Box<dyn Trait>` for "flexibility"

### Anti-pattern: Unnecessary Dynamic Dispatch

```rust
// Using trait objects when static dispatch would work
fn process_items(iter: &mut dyn Iterator<Item = i32>) -> i32 {
    iter.sum()  // Vtable lookup overhead, no inlining
}

// Boxing when not needed
fn get_iterator() -> Box<dyn Iterator<Item = i32>> {
    Box::new(vec![1, 2, 3].into_iter())  // Always returns Vec's iterator
}
```

### Prefer: Static Dispatch with Generics

```rust
// Static dispatch with generics (zero cost, fully inlined)
fn process_items<I: Iterator<Item = i32>>(iter: I) -> i32 {
    iter.sum()  // Fully inlined, optimized per type
}

// Or using impl Trait (equivalent, cleaner syntax)
fn process_items(iter: impl Iterator<Item = i32>) -> i32 {
    iter.sum()
}

// Return single concrete type
fn get_iterator() -> impl Iterator<Item = i32> {
    vec![1, 2, 3].into_iter()  // Concrete type known at compile time
}
```

### Pattern: `impl Trait` Return Values Hide Implementation Details

Returning concrete iterator adapter types (e.g., `Map<Filter<Range<...>>>`) leaks implementation details into your API surface and can cause churn when internals change.

Prefer returning `impl Trait` to keep signatures stable and readable:

```rust
// Caller doesn't need to know this is a filtered Range.
fn odd_numbers(limit: u32) -> impl Iterator<Item = u32> {
    (0..limit).filter(|x| x % 2 != 0)
}

fn main() {
    for n in odd_numbers(10) {
        println!("Odd: {}", n);
    }
}
```

**Constraint:** `-> impl Trait` is still **one concrete type** per function. If you need to return different concrete types in different branches, use an enum wrapper or `Box<dyn Trait>` (and pay the dynamic dispatch cost).

### When to Use Dynamic Dispatch

```rust
// Use Box<dyn Trait> when returning different types at runtime
fn get_iterator(flag: bool) -> Box<dyn Iterator<Item = i32>> {
    if flag {
        Box::new(vec![1, 2, 3].into_iter())
    } else {
        Box::new(0..10)
    }
}

// Heterogeneous collections require trait objects
let handlers: Vec<Box<dyn Handler>> = vec![
    Box::new(FileHandler::new()),
    Box::new(NetworkHandler::new()),
    Box::new(CacheHandler::new()),
];
```

---

## 9. Concurrency Patterns

### Understanding Concurrency Costs

Rust's concurrency primitives provide memory safety without data races, but different patterns have different costs. **Spawning tasks imposes ~1-2 microsecond overhead** in Tokio for scheduling and synchronization. **Scoped threads eliminate Arc overhead** by borrowing data for bounded lifetimes. **Rayon provides work-stealing parallelism** optimized for CPU-bound tasks.

### Standards

* **MUST** prefer scoped threads (`crossbeam::scope`) when threads have bounded lifetimes — eliminates Arc overhead
* **MUST** use `rayon` for CPU-bound parallel processing (work-stealing scheduler)
* **MUST** use `tokio` or `async-std` for I/O-bound async operations
* **SHOULD** share read-only data via `Arc<T>` for unbounded thread lifetimes
* **SHOULD** use channels for communication over `Arc<Mutex<T>>` when possible
* **SHOULD** use `Arc<RwLock<T>>` only with measured read-heavy contention (10:1 reads:writes)
* **AVOID** cloning large owned data into each task; restructure to borrow or reference shared state
* **AVOID** `Arc<Mutex<T>>` without contention analysis; channels often perform better

### Pattern: Scoped Threads Eliminate Arc

```rust
use crossbeam::thread;

// Anti-pattern: Arc when borrowing would work
fn process_parallel_arc(data: Vec<String>) {
    let data = Arc::new(data);
    let mut handles = vec![];
    
    for i in 0..4 {
        let data = Arc::clone(&data);
        handles.push(std::thread::spawn(move || {
            process(&data[i]);
        }));
    }
    
    for h in handles {
        h.join().unwrap();
    }
}

// Prefer: scoped threads borrow directly
fn process_parallel_scoped(data: &[String]) {
    crossbeam::thread::scope(|s| {
        for i in 0..4 {
            s.spawn(move |_| {
                process(&data[i]);  // Direct borrow, no Arc
            });
        }
    }).unwrap();  // Scope ensures threads complete before returning
}
```

### Pattern: Rayon for Data Parallelism

```rust
use rayon::prelude::*;

// Sequential processing
fn process_items(items: &[Item]) -> Vec<Result> {
    items.iter()
        .map(|item| expensive_computation(item))
        .collect()
}

// Parallel processing with Rayon (trivial change, work-stealing)
fn process_items_parallel(items: &[Item]) -> Vec<Result> {
    items.par_iter()  // Parallel iterator
        .map(|item| expensive_computation(item))
        .collect()  // Automatically parallelized
}

// Parallel fold/reduce
let sum: i64 = (0..1_000_000).into_par_iter()
    .map(|x| x * x)
    .sum();
```

### Pattern: Channels Over Shared Mutable State

```rust
use std::sync::mpsc;

// Anti-pattern: shared mutable state
let counter = Arc::new(Mutex::new(0));
for _ in 0..10 {
    let counter = Arc::clone(&counter);
    thread::spawn(move || {
        for _ in 0..100 {
            *counter.lock().unwrap() += 1;  // Contention on every increment
        }
    });
}

// Prefer: message passing
let (tx, rx) = mpsc::channel();
for _ in 0..10 {
    let tx = tx.clone();
    thread::spawn(move || {
        let mut local_count = 0;
        for _ in 0..100 {
            local_count += 1;  // No contention
        }
        tx.send(local_count).unwrap();
    });
}
drop(tx);

let total: i32 = rx.iter().sum();  // Collect results
```

---

## 10. Async/Await Patterns

### Understanding Async Overhead

Async/await transforms functions into state machines polled by an executor. **The futures themselves are zero-cost abstractions, but spawning tasks imposes ~1-2 microsecond overhead** in Tokio due to scheduling, memory allocation for task storage, and synchronization primitives.

### Standards

* **MUST** use async for I/O-bound operations where blocking would waste thread resources
* **SHOULD** process concurrent operations with `buffer_unordered` or `join!` macros
* **SHOULD** use `spawn_blocking()` for CPU-intensive work to avoid blocking the executor
* **SHOULD** use `tokio::select!` for racing futures or implementing timeouts
* **AVOID** spawning tasks for trivial computation (use direct `.await` instead)
* **AVOID** sequential processing of concurrent operations (use concurrent combinators)

### Anti-pattern: Sequential Async

```rust
// Spawning tasks for trivial computation
async fn process_data(data: Vec<u8>) -> Result<u8> {
    let result = tokio::spawn(async move {
        data.iter().sum::<u8>()  // 1μs overhead for microseconds of work!
    }).await?;
    Ok(result)
}

// Sequential processing of concurrent operations
async fn fetch_all(urls: Vec<String>) -> Vec<Response> {
    let mut responses = Vec::new();
    for url in urls {
        responses.push(fetch(&url).await);  // Waits for each sequentially
    }
    responses
}
```

### Prefer: Concurrent Async

```rust
// Direct await without spawning for simple async operations
async fn process_data(data: &[u8]) -> Result<u8> {
    let result = some_async_computation(data).await;  // No task overhead
    Ok(result)
}

// Concurrent processing with buffer_unordered
use futures::stream::StreamExt;

async fn fetch_all(urls: Vec<String>) -> Vec<Response> {
    futures::stream::iter(urls)
        .map(|url| async move { fetch(&url).await })
        .buffer_unordered(10)  // Process 10 concurrently
        .collect()
        .await
}

// Or using join! for fixed set
async fn fetch_two(url1: String, url2: String) -> (Response, Response) {
    tokio::join!(
        fetch(&url1),
        fetch(&url2)
    )  // Both execute concurrently
}
```

### Pattern: CPU Work in Async Context

```rust
// Anti-pattern: CPU-bound work blocks executor
async fn process() -> Vec<u8> {
    expensive_cpu_computation()  // Blocks executor thread!
}

// Prefer: move to blocking thread pool
async fn process() -> Vec<u8> {
    tokio::task::spawn_blocking(|| {
        expensive_cpu_computation()  // Runs on dedicated thread pool
    })
    .await
    .unwrap()
}
```

### Pattern: Timeouts and Select

```rust
use tokio::time::{timeout, Duration};

// Timeout pattern
async fn fetch_with_timeout(url: &str) -> Result<Response> {
    timeout(Duration::from_secs(5), fetch(url))
        .await
        .map_err(|_| Error::Timeout)?  // Times out after 5s
}

// Select first to complete
async fn fetch_fastest(urls: Vec<String>) -> Response {
    let mut futures: Vec<_> = urls.iter()
        .map(|url| Box::pin(fetch(url)))
        .collect();
    
    loop {
        tokio::select! {
            result = &mut futures[0] => return result,
            result = &mut futures[1] => return result,
            result = &mut futures[2] => return result,
        }
    }
}
```

---

## 11. Control Flow & Pattern Matching

### Understanding Why Flattening Matters

Rust will optimize `match`, `if let`, and `while let` similarly in most cases — this is primarily about reducing indentation, making the "happy path" obvious, and keeping error/None handling close to the callsite.

### Standards

* **MUST** prefer guard clauses / early returns over deeply nested `match` blocks
* **SHOULD** use `if let` when only one pattern is relevant
* **SHOULD** use `while let` for "repeat-until-empty" loops (`next()`, `recv()`, `pop_front()`, etc.)
* **AVOID** `match` with `_ => {}` when only one arm matters
* **AVOID** `while let` for simple iteration — prefer `for x in iter` when you can

### Anti-pattern: Verbose `match` for a Single Arm

```rust
let app_mode: Option<&str> = Some("Production");

match app_mode {
    Some(mode) => println!("Running in: {}", mode),
    None => {}
}
```

### Prefer: `if let` / `while let`

```rust
let app_mode: Option<&str> = Some("Production");

if let Some(mode) = app_mode {
    println!("Running in: {}", mode);
}

// Drain an iterator until it returns None.
let mut tasks = vec!["Task A", "Task B", "Task C"].into_iter();
while let Some(task) = tasks.next() {
    println!("Processing: {}", task);
}
```

### Pattern: Drain a FIFO Queue Until Empty

```rust
use std::collections::VecDeque;

let mut q: VecDeque<&str> = VecDeque::from(vec!["A", "B", "C"]);

while let Some(task) = q.pop_front() {
    println!("Processing: {}", task);
}
```

---

## 12. `const` vs `static`: Compile-Time Constants vs Addressable Globals

### The Distinction That Matters

* `const` is a compile-time constant **value**. It is typically inlined and has no stable memory address contract.
* `static` is a single global **location** with a stable address for the lifetime of the program.

### Standards

* **MUST** use `const` for configuration values, sizes, and math constants
* **MUST** use `static` only for true globals that require a stable address (FFI, atomics, lazily-initialized singletons)
* **MUST** make `static` state thread-safe (`Atomic*`, `OnceLock`, `Mutex`, etc.)
* **AVOID** `static mut` (requires `unsafe` and is easy to misuse); prefer safe concurrency primitives
* **SHOULD** pass state explicitly rather than reaching for globals by default

### Pattern: Global Counter + Compile-Time Limit

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

// Compile-time constant (inlined).
const MAX_CONNECTIONS: u32 = 100;

// Global with a stable address (shared state).
static ACTIVE_USERS: AtomicUsize = AtomicUsize::new(0);

fn try_new_connection() -> bool {
    // Relaxed is often enough for counters; use stronger orderings when correctness needs it.
    let current = ACTIVE_USERS.fetch_add(1, Ordering::Relaxed) + 1;

    if current as u32 <= MAX_CONNECTIONS {
        true
    } else {
        // Roll back if we exceeded the limit.
        ACTIVE_USERS.fetch_sub(1, Ordering::Relaxed);
        false
    }
}
```

### Pattern: Lazy Global Initialization

```rust
use std::sync::OnceLock;

struct Config {
    // ...
}

static CONFIG: OnceLock<Config> = OnceLock::new();

fn config() -> &'static Config {
    CONFIG.get_or_init(|| Config { /* ... */ })
}
```

---

## 13. Build & Release Configuration

### Understanding Build Optimization Tradeoffs

**Default release builds leave significant performance on the table**. LTO (Link-Time Optimization) enables cross-crate inlining and dead code elimination, typically improving performance by 5-15%. Reducing codegen units from 16 to 1 enables better optimization at the cost of longer compile times. CPU-specific builds can unlock vectorization and modern instruction sets but reduce portability.

### Standards

* **MUST** configure production release profiles with LTO and reduced codegen units
* **SHOULD** use `panic = "abort"` for production binaries (slightly better performance, smaller binaries)
* **SHOULD** consider PGO (Profile-Guided Optimization) for stable, performance-critical binaries
* **SHOULD** use `target-cpu=native` only for builds deployed to known CPU architectures
* **AVOID** default release settings for production deployments

### Recommended Production Profile

```toml
# Cargo.toml
[profile.release]
opt-level = 3              # Maximum optimizations
lto = "fat"                # Full cross-crate LTO
codegen-units = 1          # Single codegen unit for maximum optimization
panic = "abort"            # Abort on panic (no unwinding overhead)
strip = true               # Strip symbols from binary

# Optional: CPU-specific builds (when deployment target is known)
# [profile.release]
# [target.'cfg(target_arch = "x86_64")']
# rustflags = ["-C", "target-cpu=native"]
```

### Understanding the Tradeoffs

| Setting          | Benefit                              | Cost                          |
| ---------------- | ------------------------------------ | ----------------------------- |
| `lto = "fat"`    | 5-15% perf, smaller binary           | Much slower builds            |
| `codegen-units=1`| Better optimization                  | Slower builds                 |
| `panic = "abort"`| Small perf/size win                  | No panic unwinding/catching   |
| `target-cpu=native`| Unlock CPU-specific instructions   | Binary not portable           |

### Profile-Guided Optimization (PGO)

PGO trains the compiler on real workload behavior, enabling better inlining and branch prediction decisions. **For stable, performance-critical binaries, PGO can improve throughput by 10-20%**.

```bash
# Using cargo-pgo for simplified PGO workflow
cargo install cargo-pgo

# 1. Build instrumented binary
cargo pgo build

# 2. Run representative workload to generate profile data
cargo pgo run -- --workload production-like-load

# 3. Build optimized binary using profile data
cargo pgo optimize build --release
```

### Build Modes Summary

```toml
# Development: fast compile, debug info
cargo build

# Release (default): balanced
cargo build --release

# Production: maximum performance
[profile.release]
lto = "fat"
codegen-units = 1
panic = "abort"

# With PGO: absolute maximum (for stable hot binaries)
cargo pgo optimize build --release
```

---

## 14. Advanced Patterns

### Const Generics for Stack Allocation

Const generics enable arrays and fixed-size buffers on the stack without heap allocation. **Stack-allocated 4×4 matrices with const generics run 5x faster than heap-allocated equivalents**.

```rust
// Stack-allocated with known size at compile time
fn add_arrays<const N: usize>(a: [i32; N], b: [i32; N]) -> [i32; N] {
    let mut result = [0; N];
    for i in 0..N {
        result[i] = a[i] + b[i];
    }
    result  // No heap allocation, returned by value efficiently
}

let a = [1, 2, 3, 4];
let b = [5, 6, 7, 8];
let result = add_arrays(a, b);  // All on stack
```

A common use of const generics is parameterizing dimensions at compile time (traditional generics parameterize types; const generics parameterize values):

```rust
// Fixed-size matrix with compile-time dimensions (stack allocated).
struct Matrix<T, const ROWS: usize, const COLS: usize> {
    data: [[T; COLS]; ROWS],
}

impl<T: Default + Copy, const ROWS: usize, const COLS: usize> Matrix<T, ROWS, COLS> {
    fn new() -> Self {
        Self { data: [[T::default(); COLS]; ROWS] }
    }

    fn size_info(&self) {
        println!("Matrix size: {}x{}", ROWS, COLS);
    }
}

fn main() {
    let mat = Matrix::<f64, 4, 4>::new();
    mat.size_info();
}
```



### PhantomData: Type System Marker for Type-State and Phantom Ownership

`PhantomData<T>` is a zero-sized marker that tells the compiler your type has a relationship with `T` (or a lifetime) even if you don't store a `T` at runtime. This enables **zero-cost type-state patterns**, safer FFI wrappers, and correct auto-trait/lifetime behavior with no runtime overhead.

### Standards

* **SHOULD** use `PhantomData` for type-state/state-machine APIs (compile-time enforced call ordering)
* **SHOULD** use `PhantomData` in wrappers around raw pointers/FFI handles to model ownership or lifetimes
* **AVOID** using `PhantomData` as a workaround for confusing lifetimes — prefer simpler ownership first

### Pattern: Type-State Client (Zero Runtime Size)

```rust
use std::marker::PhantomData;

struct Connected;
struct Disconnected;

struct Client<State> {
    id: u32,
    _state: PhantomData<State>,
}

impl<State> Client<State> {
    fn new(id: u32) -> Self {
        Self { id, _state: PhantomData }
    }
}

impl Client<Disconnected> {
    fn connect(self) -> Client<Connected> {
        Client { id: self.id, _state: PhantomData }
    }
}

impl Client<Connected> {
    fn send(&self, msg: &[u8]) {
        println!("client {} -> {} bytes", self.id, msg.len());
    }

    fn disconnect(self) -> Client<Disconnected> {
        Client { id: self.id, _state: PhantomData }
    }
}

fn main() {
    let c = Client::<Disconnected>::new(1);

    // c.send(b"hi"); // Compile error: only Connected clients can send.
    let c = c.connect();
    c.send(b"hi");
    let _c = c.disconnect();
}
```

### Inline Directives

Strategic use of `#[inline]` directives guides the optimizer.

### Standards

* **SHOULD** apply `#[inline]` to small functions (1-10 lines) in hot paths, especially across crate boundaries
* **SHOULD** use `#[inline(always)]` only for proven bottlenecks with profiling evidence
* **SHOULD** mark error paths with `#[cold]` and `#[inline(never)]` to optimize branch prediction
* **AVOID** `#[inline]` without profiling; let the compiler decide in most cases

```rust
#[inline]
fn small_hot_function(x: i32) -> i32 {
    x * 2 + 1  // Small, frequently called, benefits from inlining
}

#[cold]
#[inline(never)]
fn handle_error() -> Error {
    Error::Critical  // Cold path, keep code size down
}

// Across crate boundaries, inline helps
#[inline]
pub fn public_hot_path(x: u64) -> u64 {
    x.wrapping_mul(42)  // Simple operation, inline across crate
}
```

---

## 15. Benchmarking & Measurement

### Understanding Performance Validation

**"It feels faster" is not data.** Proper benchmarking with statistical analysis prevents regression and validates optimization claims. Different tools serve different purposes: Criterion for statistical rigor, flamegraphs for hotspot identification, dhat for allocation profiling, and iai-callgrind for deterministic CI benchmarks.

### Standards

* **MUST** add at least one `criterion` benchmark when changing hot code paths
* **MUST** report **median** and **p95** latency deltas in PR descriptions
* **SHOULD** report allocation counts using `dhat` or similar heap profilers
* **SHOULD** use `cargo flamegraph` or `perf` to identify true bottlenecks before optimizing
* **SHOULD** use `iai-callgrind` for deterministic, CI-stable instruction counts
* **SHOULD** verify optimizations with `cargo asm` to inspect generated assembly
* **SHOULD** add allocation regression tests for critical paths
* **AVOID** optimizing without profiling data
* **AVOID** micro-optimizations that don't show measurable improvement

### Pattern: Criterion Benchmark

```rust
// benches/my_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n-1) + fibonacci(n-2),
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
    
    // Throughput measurement
    let data = vec![1u8; 1024];
    c.bench_function("process 1KB", |b| {
        b.iter(|| process(black_box(&data)))
    });
    
    // Parameterized benchmarks
    let mut group = c.benchmark_group("sizes");
    for size in [100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &s| {
            b.iter(|| process_n(black_box(s)))
        });
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
```

### Pattern: Deterministic CI Benchmarks with iai-callgrind

```rust
// benches/iai_benchmark.rs
use iai_callgrind::{library_benchmark, library_benchmark_group, main};

#[library_benchmark]
fn bench_fibonacci() -> u64 {
    fibonacci(20)
}

#[library_benchmark]
fn bench_process() {
    let data = vec![1u8; 1024];
    process(&data)
}

library_benchmark_group!(
    name = my_group;
    benchmarks = bench_fibonacci, bench_process
);

main!(library_benchmark_groups = my_group);
```

```bash
# iai-callgrind provides deterministic instruction counts (no timing variance)
# Perfect for CI regression detection
cargo bench --bench iai_benchmark

# Output shows:
# - Instructions executed
# - L1/L2 cache hits/misses  
# - Branch predictions
# - Estimated CPU cycles
```

### Local Experiment Methodology

1. **Select** one endpoint/function with clear inputs
2. **Baseline**: Add a criterion benchmark and record current performance
3. **Apply**: Make one focused change (borrow, iterator chain, typed error, etc.)
4. **Measure**: Record median, p95, and allocations (before/after)
5. **Verify**: Check that the improvement is statistically significant
6. **Keep**: Leave the benchmark in-tree to prevent regressions

### Profiling Tools Reference

```bash
# Run benchmarks
cargo bench

# CPU hotspot profiling with flamegraph
cargo install flamegraph
cargo flamegraph --bin myapp -- args

# Heap allocation profiling with dhat
cargo build --profile profiling
valgrind --tool=dhat ./target/profiling/myapp

# Check assembly output
cargo install cargo-asm
cargo asm mylib::hot_function

# Deterministic instruction-level benchmarks
cargo install iai-callgrind
cargo bench --bench iai_benchmark

# Cache and branch prediction analysis
valgrind --tool=cachegrind ./target/release/myapp
```

### Allocation Regression Tests

For critical paths, consider tracking allocation counts in CI:

```rust
#[cfg(test)]
mod allocation_tests {
    use dhat::{Dhat, DhatAlloc};
    
    #[global_allocator]
    static ALLOCATOR: DhatAlloc = DhatAlloc;
    
    #[test]
    fn test_no_allocations_in_hot_path() {
        let _dhat = Dhat::start_heap_profiling();
        
        // Code that should not allocate
        for i in 0..1000 {
            process_item(i);
        }
        
        let stats = dhat::HeapStats::get();
        assert_eq!(stats.total_blocks, 0, "Hot path should not allocate");
    }
}
```

---

## 16. Defining "Hot Paths"

Apply rigorous performance standards to a code path when **any** of the following is true:

### Execution Frequency
* Executed per request/message/event in a service
* Executed per item in a large batch (>1000 items)
* Called in a tight loop

### Resource Impact
* Dominates CPU time in flamegraphs (>5% of total CPU)
* Drives allocator traffic or peak memory usage
* Contributes to p95/p99 tail latency

### Security/Reliability
* Processes untrusted data (parsing, hashing, validation)
* Required for system availability or correctness

### Examples of Hot Paths
- Request handler bodies in HTTP services
- Message processing in queue consumers
- Per-record transformations in batch jobs
- Serialization/deserialization of large payloads
- Database query execution and result mapping
- Inner loops in data processing pipelines

### Examples of Cold Paths
- Application startup and initialization
- Configuration file loading
- Infrequent admin operations
- Error handling code (though errors should still be typed)
- One-time setup or migration code

When in doubt, **profile first**. If a path appears in the top 10% of CPU time or allocation count, treat it as hot.

---

## 17. Tooling & Lints

### Standards

* **MUST** enable Clippy in CI with performance-focused lints
* **MUST** run `rustfmt` with project default configuration
* **MUST NOT** blanket-suppress unused `#[must_use]` warnings; handle `Result`/`Option` or discard explicitly (`let _ = ...;`)
* **SHOULD** use `cargo-udeps` to prune unused dependencies
* **SHOULD** use `cargo-deny` for license and vulnerability checks
* **SHOULD** use `cargo-nextest` for faster test execution
* **AVOID** per-file formatting overrides

### Recommended Clippy Configuration

```toml
# clippy.toml
warn = [
  "clippy::perf",
  "clippy::nursery",
  "clippy::needless_collect",
  "clippy::map_flatten",
  "clippy::or_fun_call",
  "clippy::inefficient_to_string",
  "clippy::unnecessary_wraps",
  "clippy::useless_conversion",
]
allow = [
  "clippy::module_name_repetitions",
]
```

### CI Pipeline Example

```yaml
# .github/workflows/ci.yml
- name: Clippy
  run: cargo clippy --all-targets --all-features -- -D warnings

- name: Format check
  run: cargo fmt -- --check

- name: Tests
  run: cargo nextest run

- name: Benchmarks (on main)
  if: github.ref == 'refs/heads/main'
  run: cargo bench --no-fail-fast
```

---

## 18. Code Review Checklist

Apply this checklist to every PR touching performance-sensitive code:

* [ ] **Clones**: Any `.clone()` inside loops or task spawns? Can it borrow or share `Arc`?
* [ ] **Collections**: Any `collect()` immediately followed by `map`/`filter`/`sum`? Chain lazily.
* [ ] **Capacity**: Are large `Vec`s/`String`s/`HashMap`s pre-sized with `with_capacity`?
* [ ] **Strings**: Are function parameters `&str` instead of `String`? Any `format!()` in hot paths?
* [ ] **Errors**: Are failures typed with `thiserror`? Using `?` and small enums?
* [ ] **Discarded must_use values**: Any intentionally ignored `Result`/`Option` is explicit (`let _ = ...;` / `drop(...)`) and justified (best-effort)?
* [ ] **Logs**: Are logs structured with fields via `tracing`? No hot-path string formatting?
* [ ] **Hashing**: Using appropriate hasher? Default for untrusted input, FxHash/AHash for internal hot paths with documented rationale?
* [ ] **I/O**: Is file/network I/O buffered? Large serialization streaming to writer instead of building intermediate String?
* [ ] **Concurrency**: Is shared data immutable or guarded appropriately? Unnecessary locks? Could we use scoped threads or channels? Bounded concurrency for async?
* [ ] **Dispatch**: Are hot paths using static dispatch (generics/`impl Trait`) instead of `dyn Trait`?
* [ ] **Async**: Are concurrent operations processed concurrently? Is CPU work in `spawn_blocking`?
* [ ] **Build**: Production profile configured with LTO and optimized codegen units?
* [ ] **Bench**: Does the PR include before/after numbers (median/p95, allocs) for hot path changes?

---

## 19. Conclusion: Measurement Drives Optimization

These patterns represent battle-tested best practices from the Rust Performance Book, Jon Gjengset's work, and community benchmarks. **The unifying principle: Rust's zero-cost abstractions are real** — iterators, generics, and Result types compile to optimal machine code when used correctly.

### The Core Performance Levers

1. **Borrow by default**, clone intentionally — the 40,000x difference
2. **Pre-allocate collections** with known sizes — eliminate 2-3x reallocation overhead
3. **Chain iterators** lazily — zero intermediate allocations
4. **Choose collections** matching access patterns — O(1) vs O(n) matters
5. **Select hashers** appropriately — 5-10x speedup with FxHash for trusted keys
6. **Buffer I/O** operations — reduce syscalls by 100-1000x
7. **Static dispatch** in hot paths — zero overhead vs 3-5ns per call
8. **Structured logging** — queryable fields without allocation
9. **Scoped concurrency** — eliminate Arc overhead when possible
10. **Optimize build config** — LTO and PGO for production binaries
11. **Measure everything** — profile before optimizing, benchmark to validate

### The Development Cycle

1. **Write correct code first** — get the logic right
2. **Identify hot paths** — use the criteria in Section 16
3. **Profile to find bottlenecks** — don't guess (flamegraphs, dhat)
4. **Apply patterns strategically** — focus on measured hot paths
5. **Benchmark to validate** — measure the improvement (Criterion, iai-callgrind)
6. **Keep benchmarks in-tree** — prevent regressions with CI

### Build Performance Culture

* **PR discipline**: Require benchmarks for hot path changes
* **Profiling first**: "It feels faster" is not data
* **Measure twice, optimize once**: Validate assumptions with tools
* **Document tradeoffs**: When using FxHash or other unsafe-for-untrusted-input optimizations, explain why
* **Track regressions**: Use iai-callgrind or allocation tests in CI

What distinguishes high-performance Rust from merely adequate code isn't exotic unsafe blocks or inline assembly, but systematic application of these patterns. The compiler's optimizer is sophisticated — trust but verify through profiling and benchmarks. When you do optimize, these patterns transform Rust from a safe systems language into one that rivals C++ performance while maintaining memory safety guarantees.

**Start with one pattern per PR, measure the impact, and build performance culture through discipline.**