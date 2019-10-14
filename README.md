# Bluepaper

Bluepaper is a small and not quite functional tool to export Dropbox Paper documents to valid and clean LaTeX code.

Bluepaper uses the excellent [pulldown-cmark](https://crates.io/crates/pulldown-cmark) Rust crate and adds some Dropbox-specific preprocessing and a generator for clean LaTeX code.
It will soon also include a packager that will deal with included images, and it will provide this functionality via both a command line interface and a client side web app (via WebAssembly).


## Motivaton

I'm writing this tool mostly for my personal use, but it may be useful to others too.
I often want to quickly draft out some ideas for a scientific project.
For quickly drafted documents with lots of equations, Dropbox Paper seems to be the only usable solution to me due to its inline LaTeX support.
Starting a full LaTeX document just to quickly write up some notes has too much overhead, and the equation editors in all other typesetting applications that I know are broken.
However, if an idea pans out, Dropbox Paper becomes too limiting at some point, and I need a tool to quickly convert my notes to LaTeX.


## Priorities

Bluepaper's priorities are, in this order:

1. **Ease of use:**
  comes (soon) with a client side web-app (using WASM) that requires no installation, and that optionally talks directly to Dropbox via its REST API.
2. **Hassle free:**
  generates LaTeX code that should always compile on the first try without requiring any fixups.
3. **Clean LaTeX code:**
  generates nicely formatted LaTeX code with a minimal preamble, even if this means that the resulting PDF document may not look very pretty.
  Most users will want to apply their own styles anyway, so Bluepaper prioritizes clean and minimalistic LaTeX code that should be compatible with most style sheets over pretty layout.


## State

The backend in directory `core` is fairly stable.
There's a draft command line interface application in the directory `cli` that technically works but it's a bit rough around the edges, and it currently lacks support for included images.
An installation free "wasm" based web frontend will come soon.


## Todo

- [ ] Add wasm frontend with basic Markdown --> LaTeX support.
- [ ] Use Dropbox REST API in wasm frontend (as an optional feature).
- [ ] Download images included in documents, and include them in the generated LaTeX (convert to a supported format if necessary).


## License

[MIT License](LICENSE)
