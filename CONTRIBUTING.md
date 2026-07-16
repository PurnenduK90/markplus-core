# Contributing to MarkPlus Core

Thank you for contributing to `markplus-core`! To ensure a smooth review process and maintain code quality, please follow these guidelines before submitting a pull request.

## Developer Checklist

Before committing your changes, please run the following commands to ensure everything is formatted, linted, tested, and compiles correctly.

### 1. Format Code
Ensure your code complies with the standard Rust formatting style.
```bash
cargo fmt
```

### 2. Linting (Clippy)
Run Clippy to catch common mistakes and improve your Rust code.
```bash
cargo clippy -- -D warnings
```

### 3. Run Tests
Run the standard test suite to ensure no regressions were introduced.
```bash
cargo test
```

### 4. Code Coverage (Optional but Recommended)
If you have `cargo-llvm-cov` installed, you can check your test coverage.
```bash
cargo llvm-cov
```

### 5. Build WebAssembly (Wasm)
`markplus-core` compiles to WebAssembly. You must verify that your changes do not break the Wasm build!
```bash
wasm-pack build --target web --release
```

## Submitting a Pull Request
1. Ensure all the steps above complete without errors.
2. If you added new features, please add corresponding tests.
3. Update `CHANGELOG.md` if your change is notable.
4. Create a descriptive Pull Request.
