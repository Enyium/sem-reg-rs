[<img alt="crates.io" src="https://img.shields.io/crates/v/sem-reg.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/sem-reg)

A Rust library crate for Windows that abstracts binary registry values, so they can be handled semantically.

Currently, these registry values are handled:

- Those of the [Night Light](https://support.microsoft.com/windows/set-your-display-for-night-time-in-windows-18fe903a-e0a1-8326-4c68-fd23d7aaf136) feature. Includes a command line program (see below).

Since the knowledge about the undocumented registry values must be acquired through own investigation and not every unclarity can be resolved, this isn't an exact science. This implies that the parsing helpers, because of their potentially shape-shifting nature, aren't provided as their own crate, and handling of different registry values is done "in-house" in this repository for the time being. When you want to add your parsing code for other registry values, please approach me to include it in the crate (unless you solved all previously mentioned problems).

The API isn't stable. Future requirements of handling other registry values or more insights into the values' formats may necessitate changes.

# `night-light` Command Line Program

Allows you to adjust Night Light's active-state, color temperature, preview state and schedule. Corrects Windows bugs like warm color temperature being reset to cold after turning the screen back on. There are also a few extra commands, like for exporting the registry values.

Binaries are available on the [releases page](https://github.com/Enyium/sem-reg-rs/releases). Not every version may be provided there.

To install the newest version, first install [Rust](https://www.rust-lang.org/) to get `cargo`; then run:

```
cargo install --bin night-light sem-reg
```

This will also automatically make it available in the `PATH`.

## Similar Software

The [`nightlight`](https://crates.io/crates/nightlight) crate offers a library and command line program for macOS to control the screen color temperature.

For Linux, ways to control GNOME's Night Light or KDE's Night Color from the command line can easily be researched.

# License

Licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

# Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
