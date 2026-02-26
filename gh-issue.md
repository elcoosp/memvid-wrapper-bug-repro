## Bug Description

When using a wrapper that holds a `Memvid` inside a `tokio::sync::Mutex` and enables the lexical index while holding the lock, searches fail with zero hits or the error `"Lexical index is not enabled"`. A minimal raw test that performs the same steps passes.

## Steps to Reproduce

1. Clone the minimal repro repository:
   ```bash
   git clone https://github.com/elcoosp/memvid-wrapper-bug-repro
   cd memvid-wrapper-bug-repro
   ```

2. Run the tests:
   ```bash
   cargo test -- --nocapture
   ```

3. Observe that:
   - `test_raw_with_mutex_passes` passes
   - `test_wrapper_fails` fails with the described errors

## Expected Behavior

Both tests should pass – the wrapper should behave identically to the raw test.

## Actual Behavior

- `test_raw_with_mutex_passes` ✅ passes
- `test_wrapper_fails` ❌ fails with:
  - Plain‑text search returns zero hits, or
  - Tag search panics with `Internal("Search failed: Lexical index is not enabled")`

## Environment

- **OS**: macOS Tahoe 26.2
- **Rust Version**: v1.93.0-nightly
- **Memvid Version**: 2.0
- **Features Enabled**: lex

## Minimal Reproducible Example

[https://github.com/elcoosp/memvid-wrapper-bug-repro](https://github.com/elcoosp/memvid-wrapper-bug-repro)

## Error Output

```
thread 'test_wrapper_fails' panicked at src/main.rs:XXX:62:
called `Result::unwrap()` on an `Err` value: Internal("Search failed: Lexical index is not enabled")
```

Or for plain‑text search:
```
thread 'test_wrapper_fails' panicked at src/main.rs:YYY:9:
assertion `left == right` failed: Should find the frame with 'login' in the message
  left: 0
 right: 1
```

## Additional Context

- The wrapper enables the lexical index **while holding the lock** and commits immediately after (exactly like the raw test).
- The same `Memvid` instance is reused across operations within the wrapper test.
- The raw test recreates the `Mutex` each time, but that should not affect the outcome.
- The `lex` feature is enabled in `Cargo.toml` of the repro project.

### Suspected Cause

The index may not be created unless the first commit contains at least one document. In the raw test, the commit after enabling (without a document) might still create the index structures; in the wrapper, the same commit does not. Alternatively, the index configuration might be lost when the lock is released and reacquired, even though `enable_lex()` is called again before each operation.

This issue is blocking proper use of lexical search in an application that wraps `Memvid` for thread safety. Any guidance on how to correctly initialise the index would be appreciated.

## Checklist

- [x] I have searched existing issues for duplicates
- [x] I have tested with the latest version
- [x] I can reproduce this consistently
