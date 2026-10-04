# Third-party licenses

Skill Atlas bundles open-source Rust and JavaScript dependencies. Their license texts and attribution remain available from the respective package sources and package metadata.

Major runtime components include:

- Tauri and Tauri plugins: Apache-2.0 OR MIT
- React: MIT
- Phosphor Icons: MIT
- SQLite and rusqlite: Public Domain / MIT
- Tokio, Reqwest, Serde, Walkdir, Notify, SHA-2 and related Rust crates: MIT OR Apache-2.0 unless their package metadata states otherwise

Run `cargo metadata --manifest-path apps/desktop/src-tauri/Cargo.toml` and `pnpm licenses list --prod` against the released source tag for the exact dependency graph.
