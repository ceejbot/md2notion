# md2notion

Convert markdown to a vector of Notion blocks for the [notion-client](https://lib.rs/crates/notion-client) Rust library.

The crate exposes exactly one function: `convert`. This takes a `&str` and returns a vector of blocks suitable for passing as page children. See the example [make-page.rs](./examples/make-page.rs) for typical usage.

The complicated Markdown test file is from [this repo](https://github.com/mxstbr/markdown-test-file). Other test files are from GitHub docs from my own projects or were written for this project.

## Development

If you have the `just` command runner installed, run `just setup`. If you prefer not to use just, the only thing that matters is having a nightly installed so you can run `cargo +nightly fmt` to make imports formatted the fussy way I like.

Some of the converted Markdown markups might be incorrect or not quite right for your use case. I haven't looked at every single one in Notion yet. The unit tests are not yet comprehensive.

## License

This code is licensed via [the Parity Public License.](https://paritylicense.com) This license requires people who build on top of this source code to share their work with the community, too. See the license text for details.
