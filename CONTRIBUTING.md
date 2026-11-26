# Contributing

Read [docs/DEVELOP.md](DEVELOP.md) for Windows and WSL build prerequisites, and [SECURITY.md](SECURITY.md) before changing launch, configuration, icon, or process handling. Only build code you trust: Cargo dependencies can execute build scripts.

Use a short-lived branch from `main`, such as `fix/config-validation` or
`feature/new-setting`, and open a pull request. Never send secrets, personal
configuration, certificates, or generated binaries. Keep `Cargo.lock` committed.
Contributions to this project are under GPL-3.0-only; do not add third-party code
without compatible licensing and attribution.

Before opening a pull request on Windows:

```powershell
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
```

Use `cargo fmt --all` to fix formatting. The repository pins Rust in
`rust-toolchain.toml`; rustup installs it when first needed. WSL can run tests and
check the Windows GNU target as described in the README, but it does not replace
Windows UI, UAC, startup, or installer verification.

Dependency changes must include the manifest and lockfile, advisory review, and
review of license changes. Action updates must keep full commit-SHA pins.
Run `cargo audit` after installing the version used in CI; review warnings even
when the command succeeds. Do not silence new advisories to get a green build.

See [docs/RELEASING.md](docs/RELEASING.md) for branch protections, versioning, and releases.
