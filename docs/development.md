# Development

```sh
cargo test      # engine, app and screen tests
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

`packaging/` has the Scoop manifest and winget manifests, `bucket/` has the
Scoop bucket, and `Formula/` has the Homebrew formula. After each release,
the release workflow writes the new version and SHA256 hashes into the
formula and the Scoop manifests (`.github/scripts/packaging.sh`) and commits
them to `master`, and opens the winget pull request.

CI runs fmt, clippy and the tests on Windows, Linux (x86-64 and ARM64) and
macOS, and builds the Intel macOS binary. To publish a release, push a tag such
as `v0.4.0`: the release workflow builds the six binaries (Windows x64 and
ARM64, macOS Intel and Apple Silicon, Linux x86-64 and ARM64) and adds them
to the GitHub Release.

The code has three parts:

- `src/engine/`: typing rules, stats and word lists. No terminal code.
- `src/app.rs`: screens and key handling.
- `src/ui/`: drawing with [ratatui](https://ratatui.rs).
