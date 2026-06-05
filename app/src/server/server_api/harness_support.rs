// We don't directly run agent harnesses on WASM, so this code is unused.
#![cfg_attr(target_family = "wasm", expect(dead_code))]

#[cfg(test)]
#[path = "harness_support_tests.rs"]
mod tests;
