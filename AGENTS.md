# AGENTS.md

Guidance for coding agents working in this repository.

## Project context

This repository is a Rust workspace containing the `dav-xml` XML library and
the `dav-xml-client` sync and async WebDAV client. `dav-xml` derives from the
`webdav-xml` crate by d-k-bo; migrated files retain their upstream SPDX
copyright header alongside the project copyright header.

The toolchain is pinned to Rust 1.98.0 with edition 2024. Keep
`rust-toolchain.toml` and `package.rust-version` in `Cargo.toml` synchronized.

## Working rules

- Read the relevant source, documentation, configuration, and tests before
  making changes.
- Keep changes scoped to the requested task and preserve unrelated user work.
- Use existing project patterns before adding new abstractions.
- Add or update focused tests before changing behavior.
- Update user documentation when commands, public APIs, configuration, or
  workflows change.
- Use `git`; never use `jj` or git worktrees.
- Never open a GitHub issue without prior user approval.
- Do not stage or commit local plans. Planning state belongs under the ignored
  `docs/superpowers/`, `.superpowers/`, or `.claude/plans/` directories.

## Source layout

- `crates/dav-xml/src/lib.rs` contains the public XML API and crate docs.
- `crates/dav-xml/src/element.rs`, `value.rs`, `read.rs`, and `write.rs`
  contain the generic XML representation and conversion machinery.
- `crates/dav-xml/src/elements.rs` and `elements/` contain RFC 4918 elements.
- `crates/dav-xml/src/properties.rs` and `properties/` contain RFC 4918
  properties.
- `crates/dav-xml-client/src/lib.rs` contains the public client API and crate
  docs (included from `crates/dav-xml-client/README.md`).
- `crates/dav-xml-client/src/client.rs` and `async_client.rs` hold
  `DavClient` and `AsyncDavClient`; both share the same method set over a
  `Transport`/`AsyncTransport` abstraction in `transport.rs`.
- Use `module_name.rs`; never introduce `mod.rs`.

## Rust conventions

- Follow `rustfmt.toml` and format Rust through `just fmt`.
- Clippy must pass with warnings denied.
- Public library modules and items require canonical rustdoc documentation.
- Avoid `unsafe` unless the task requires it and its safety invariants are
  documented and tested.
- Use named format placeholders instead of positional `{}` arguments.
- Prefer `#[expect]` with a reason over `#[allow]` for local lint overrides.
- Keep dependency entries and feature definitions alphabetically sorted.
- Use bare, minimal dependency versions in the workspace `Cargo.toml`.
- New Rust files must use the project SPDX header. Keep d-k-bo's SPDX header
  on migrated files.

## Command interface

Use [`just`](https://just.systems) recipes when one exists. Do not bypass a
recipe with an ad hoc `cargo` or tool command. If a recurring task has no
recipe, add a focused recipe before using it.

Run `just` to list every command. The primary recipes are:

```sh
just build
just release
just test
just coverage
just fmt
just fmt_check
just lint "-- -D warnings"
just doc
just deny
just scan_secrets
just check
just setup_githooks
```

`just check` is the required local quality gate. It runs formatting checks,
Clippy with warnings denied, rustdoc with warnings denied, cargo-deny, and the
workspace test suite.

When invoking compilation or test commands from the CLI, never request
parallelism greater than eight. This is an invocation constraint; do not encode
the local cap in tracked project files.

### `dav-xml-client` feature matrix

`dav-xml-client` gates each HTTP backend behind a Cargo feature so
downstream crates only build the dependencies they need:

| Feature      | Enables                                                                    | Default |
| ------------ | -------------------------------------------------------------------------- | ------- |
| `reqwest`    | `AsyncDavClient::reqwest`, an async backend for `tokio`.                   | yes     |
| `ureq`       | `DavClient::ureq`, a blocking backend with no async runtime.               | no      |
| `isahc`      | `DavClient::isahc` and `AsyncDavClient::isahc`, libcurl for sync or async. | no      |
| `native-tls` | Switches the `reqwest` and `ureq` backends to the platform TLS stack.      | no      |
| `rustls`     | No-op: `reqwest` and `ureq` already default to rustls (see below).         | no      |
| `mock`       | `transport::MockTransport`, for downstream tests.                          | no      |
| `containers` | Docker-backed integration tests in `tests/containers.rs`; pulls in `mock`. | no      |

`isahc` always builds with its own bundled `default-tls`, unaffected by
`native-tls` or `rustls`. For `reqwest` and `ureq`, rustls is already the
default (`reqwest`'s own `default-tls` feature resolves to rustls, and
`ureq`'s `TlsProvider` defaults to rustls too), so this crate's `rustls`
feature does nothing; enable `native-tls` to force the native TLS backend
for both instead. Enabling both `native-tls` and `rustls` together (as
`--all-features` does) resolves to `native-tls`.

Building `dav-xml-client` with the `isahc` feature (including any
`--all-features` build) compiles `curl-sys` from source and requires a C
toolchain (`cc`, `make`) on the machine. GitHub-hosted CI runners ship one by
default; install `build-essential` (Debian/Ubuntu), Xcode Command Line Tools
(macOS), or a Visual Studio Build Tools install (Windows) locally if the
build fails to find a C compiler.

Exercise the feature matrix and the container-backed integration tests with:

```sh
just test_features   # every backend feature combination, mocked transport
just containers_up   # start the WebDAV container used by containers tests
just test_containers # full verb sequence against the running container
just containers_down # stop and remove the container
```

## Required tools

The local recipes expect:

- Rust and rustup.
- just.
- dprint 0.56.1 and nightly rustfmt.
- cargo-deny.
- TruffleHog.
- git-cliff.
- cargo-llvm-cov for coverage.
- zizmor for workflow changes.
- shellcheck for shell hook changes.

If a required tool is unavailable, report it explicitly. Do not claim its check
passed or silently replace the repository command with a weaker check.

## Documentation

- Write task-oriented documentation with runnable examples.
- Use ATX headings and fenced code blocks with language identifiers.
- Keep Markdown lines readable, tables aligned, and files terminated by one
  newline.
- Run `fmt-md-tables -i <file>` after editing a Markdown file that contains a
  table.

## GitHub Actions

- Run `zizmor .github/workflows` after every workflow change until it exits
  successfully with no findings.
- Pin every external action to the full commit SHA of its latest stable release
  and record the exact matching tag in a trailing comment.
- Declare explicit least-privilege permissions.
- Set `persist-credentials: false` on checkout steps unless later authenticated
  Git operations are explicitly required.
- Never interpolate attacker-controlled GitHub expressions directly into a
  shell script. Pass values through `env` and quote the shell variable.

## Git and releases

- Use Conventional Commits with an imperative, lower-case description.
- Do not add agent attribution, session links, or agent `Co-Authored-By` lines.
- Inspect the diff before staging or committing changes.
- Generate release notes with `just changelog_preview <version>` and
  `just changelog <version>`.
- `just publish` publishes workspace crates in dependency order; use
  `just publish_crate <name>` for one crate.
- Verify packages locally with `just publish "--dry-run --allow-dirty"`.
- Live publication uses crates.io trusted publishing through
  `.github/workflows/publish.yml`.
