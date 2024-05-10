# mtc-token-healing

[![Crates.io](https://img.shields.io/crates/v/mtc-token-healing.svg)](https://crates.io/crates/mtc-token-healing)
[![Docs.rs](https://img.shields.io/docsrs/mtc-token-healing.svg)](https://docs.rs/mtc-token-healing)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-informational.svg)](#license)
[![Build status](https://github.com/ModelTC/mtc-token-healing/actions/workflows/ci.yml/badge.svg)](https://github.com/ModelTC/mtc-token-healing/actions)

Token healing implementation in Rust.

## Usage

See [`examples/rand-infer.rs`](examples/rand-infer.rs).

```sh
echo '"def helloworl"' | cargo run --example rand-infer
```

```sh
echo '"def hellowor<unk>l"' | cargo run --example rand-infer
```

## TODOs

- [x] Python bindings
- [ ] Python docs, examples and tests

## License

- &copy; 2024 Chielo Newctle \<[ChieloNewctle@gmail.com](mailto:ChieloNewctle@gmail.com)\>
- &copy; 2024 ModelTC Team

This project is licensed under either of

- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0) ([`LICENSE-APACHE`](LICENSE-APACHE))
- [MIT license](https://opensource.org/licenses/MIT) ([`LICENSE-MIT`](LICENSE-MIT))

at your option.

The [SPDX](https://spdx.dev) license identifier for this project is `MIT OR Apache-2.0`.
