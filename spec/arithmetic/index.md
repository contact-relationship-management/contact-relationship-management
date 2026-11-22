# Arithmetic

Math must not panic. Integer arithmetic that can overflow, underflow or divide by zero uses Rust's `checked_*`
functions, and when the operation cannot be done it returns `None`. It does not panic and it does not wrap.

Example, an `Option<T>`:

```rust
assert_eq!(255_u8.checked_add(1), None); // 255 + 1 does not fit in a u8: None, not 0 and not a panic
assert_eq!(250_u8.checked_add(5), Some(255));
assert_eq!(7_u32.checked_div(0), None); // division by zero: None, not a panic
```

## Rules

1. **Use `checked_*`.** `checked_add`, `checked_sub`, `checked_mul`, `checked_div`, `checked_rem`, `checked_neg`,
   `checked_abs`, `checked_pow` and `checked_shl` for integers. A bare `+`, `-`, `*`, `/` or `%` on integers that come from
   a request, a database row, a setting or a clock is not allowed in production code.
2. **`None` is the answer.** A function whose arithmetic can fail returns `Option<T>`, or a `Result` made from the
   `Option` (`ok_or(...)`) when the caller needs a reason. The caller decides what `None` means: a refusal (`422`), a
   refused start, or a documented default. It is never a wrapped or clamped number that looks like a real result.
3. **Never unwrap a checked result.** The `Option` that `checked_*` returns is handled, not forced: no `.unwrap()`,
   `.expect(...)`, `unwrap_unchecked()`, and no `.unwrap_or(...)`, `.unwrap_or_default()` or `.unwrap_or_else(...)` that
   turns `None` into a number (`i64::MAX`, `0`), because that is the clamp or the wrap by another name. Handle `None`
   with `?`, `ok_or(...)?`, `let ... else`, `match`, or `is_some_and`, and let the caller decide.
4. **No wrapping.** `wrapping_*` and `overflowing_*` are not used for a value that is stored, shown or compared.
   Release builds wrap on overflow unless told otherwise; the checked function makes the failure visible in every build.
5. **Indexing and slicing are arithmetic too.** `slice[i]` and `&text[a..b]` panic past the end; use `get`, `first_chunk`,
   `split_at_checked` or `strip_prefix`.
6. **Tests may panic.** Test code, examples and benchmarks may use bare operators and `unwrap` to read a figure, and the
   lints are off there; a test that is about overflow asserts `None`.

## How it is enforced

- `clippy::arithmetic_side_effects`, `clippy::indexing_slicing`, `clippy::panic`, `clippy::unreachable`,
  `clippy::unwrap_used` and `clippy::expect_used` warn outside `#[cfg(test)]`, and CI runs clippy with `-D warnings`
  (`scripts/ci-check.sh clippy`).
- A finding is fixed, not allowed. An `#[allow(...)]` needs a `reason` that shows the failure is impossible (a constant
  divisor that is not zero, for instance), and is reviewed with the change.

## Where it stands (2026-11-22)

- `unwrap`, `expect`, `panic!`, `unreachable!` and indexing were removed from production code (CRM-T300).
- Integer arithmetic is being moved to `checked_*` (CRM-T301). Not yet at the rule: `Timestamp` and `Date` arithmetic in
  `time.rs` **saturates** at the ends of the range rather than returning `None`, and the first pass over event and row
  counters used `saturating_add`. Both clamp, which rule 2 does not allow; they are to become `checked_*` with the
  caller handling `None`. Until then they are recorded as exceptions in `tasks.md`, not as the standard.
