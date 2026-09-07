# Run all unit, integration, binary, and documentation tests.
[group('test')]
test args="":
  cargo test --workspace --all-targets {{ args }}
  cargo test --workspace --doc --all-features

# Generate an LCOV coverage report.
[group('test')]
coverage output="lcov.info" args="":
  cargo llvm-cov --workspace --all-targets --lcov --output-path {{ output }} {{ args }}

# Start the WebDAV container the `containers` tests connect to.
[group('test')]
containers_up:
  docker compose -f tests/docker-compose.yml up -d

# Stop and remove the WebDAV test container.
[group('test')]
containers_down:
  docker compose -f tests/docker-compose.yml down

# Run the container-backed integration tests for every backend.
[group('test')]
test_containers args="":
  cargo test -p dav-xml-client --all-features --test containers {{ args }}

# Run the test suite for every backend feature combination.
[group('test')]
test_features args="":
  cargo test -p dav-xml-client --no-default-features {{ args }}
  cargo test -p dav-xml-client --no-default-features --features ureq {{ args }}
  cargo test -p dav-xml-client --no-default-features --features reqwest {{ args }}
  cargo test -p dav-xml-client --no-default-features --features isahc {{ args }}
  cargo test -p dav-xml-client --all-features {{ args }}
