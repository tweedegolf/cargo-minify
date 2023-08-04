# cargo-minify

A tool to minimize bindgen-generated Rust files (but that may also work on more general Rust programs)

## Limitations

* Public functions and types in libraries (which makes sense) but also in examples are not considered unused
* `cargo check` (which `cargo minify` uses in the background) occasionally ignores unused definitions for some reason

## License

Licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as
defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
