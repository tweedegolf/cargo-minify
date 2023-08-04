# cargo-minify

A tool to minimize bindgen-generated Rust files (but that may also work on more general Rust programs)

## Limitations

* Public functions and types in libraries (which makes sense) but also in examples are not considered unused
* `cargo check` (which `cargo minify` uses in the background) occasionally ignores unused definitions for some reason
