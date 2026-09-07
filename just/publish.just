# Publish every workspace crate to crates.io in dependency order.
[group('publish')]
publish args="":
  cargo publish --locked --workspace {{ args }}

# Publish a single workspace crate to crates.io.
[group('publish')]
publish_crate name args="":
  cargo publish --locked -p {{ name }} {{ args }}
