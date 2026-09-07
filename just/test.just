# Run all unit, integration, binary, and documentation tests.
[group('test')]
test args="":
  cargo test --workspace --all-targets {{ args }}
  cargo test --workspace --doc {{ args }}

# Generate an LCOV coverage report.
[group('test')]
coverage output="lcov.info" args="":
  cargo llvm-cov --workspace --all-targets --lcov --output-path {{ output }} {{ args }}
