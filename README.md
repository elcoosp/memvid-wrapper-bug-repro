# memvid-wrapper-bug-repro

Minimal reproduction of a lexical indexing bug in `memvid-core`.

## Problem

When using a simple wrapper that stores a `Memvid` instance inside a `tokio::sync::Mutex` and enables the lexical index **while holding the lock** (exactly as done in a passing raw test), subsequent searches (both plain‑text and tag‑based) fail with:

- Zero hits for plain‑text search, or
- `Internal("Search failed: Lexical index is not enabled")` for tag search.

The raw test, which performs the same steps in the same order, **passes** successfully. This suggests a subtle difference between the two approaches, possibly related to how the `Memvid` instance is initialised or how the index is persisted across commits.

## Tests

- `test_raw_with_mutex_passes` – uses a raw `Memvid` wrapped in a `Mutex`, manually enables the index, inserts a frame, and searches. **This test passes**.
- `test_wrapper_fails` – uses a `MemvidStore` wrapper that mimics your application’s `MemvidStore`. It enables the index while holding the lock, inserts a frame, and searches. **This test fails** with the errors described above.

## How to run

```bash
cargo test -- --nocapture
```

Expected output:

- `test_raw_with_mutex_passes ... ok`
- `test_wrapper_fails ... FAILED`

## Dependencies

- `memvid_core` with the `lex` feature enabled
- `tokio`, `tempfile`, `serde_json`, `chrono`

## Suspected cause

The index configuration may not be properly persisted when the `Memvid` is first created and enabled, even though a commit follows immediately. The raw test recreates the `Mutex` each time, while the wrapper reuses the same instance – but that alone should not cause failure. Possibly the index is not created until the first actual document is added, and the wrapper’s early commit (without any data) does not materialise it, whereas the raw test’s commit after enabling (still without data) somehow does. Alternatively, the library may require that the index be enabled **before** the first `put` **without** an intervening empty commit.

## Workaround

None yet. The issue is being investigated.
