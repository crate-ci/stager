# stager

> **Stage files** - Generic tool to lay out files for bundling into platform-specific installers.

[![Travis Status](https://travis-ci.org/crate-ci/stager.svg?branch=master)](https://travis-ci.org/crate-ci/stager)
[![Appveyor Status](https://ci.appveyor.com/api/projects/status/mj0bbemw47jyfwta/branch/master?svg=true)](https://ci.appveyor.com/project/epage/stager/branch/master)
[![Documentation](https://img.shields.io/badge/docs-master-blue.svg)][Documentation]
![License](https://img.shields.io/crates/l/stager.svg)
[![Crates Status](https://img.shields.io/crates/v/stager.svg)](https://crates.io/crates/stager)

## Install

Add to your `Cargo.toml`:

```toml
[dependencies]
stager = "0.3"
```

## Example

[staging][staging] will
- Read a stage configuration (using `staging::de`) and variables to be substitued using [liquid][liquid].
- Transform the configuration and variables into the stager API (`staging::builder`).
- Transform the builders into distinct actions to be performed on the file system (`staging::action`).
- Apply these actions to the target directory.

[staging]: https://github.com/crate-ci/stager/blob/master/src/bin/staging/main.rs
[liquid]: https://shopify.github.io/liquid/

### Packaging Systems

- [`cargo-tarball`][tarball]: Tarball a Rust projct for github releases.

[tarball]: https://github.com/crate-ci/cargo-tarball

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

[Crates.io]: https://crates.io/crates/stager
[Documentation]: https://docs.rs/stager
