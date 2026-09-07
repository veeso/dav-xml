# Run all unit, integration, binary, and documentation tests.
[group('test')]
test args="":
  cargo test --all-targets {{ args }}
  cargo test --doc {{ args }}

# Generate an LCOV coverage report.
[group('test')]
coverage output="lcov.info":
  cargo llvm-cov --all-targets --lcov --output-path {{ output }}
