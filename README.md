# Bluepaper

Bluepaper is a small web app that exports Dropbox Paper documents to valid and clean LaTeX code.

- **Try it out here:** [https://robamler.github.io/bluepaper/](https://robamler.github.io/bluepaper/)

Bluepaper uses the excellent [pulldown-cmark](https://crates.io/crates/pulldown-cmark) Rust crate and adds some Dropbox-specific preprocessing and a generator for clean LaTeX code.
It will soon also include a packager that will deal with included images.

The most convenient way to use Bluepaper is via a client side [web app](https://robamler.github.io/bluepaper/) that runs the Bluepaper back end as a WebAssembly (WASM) module.
There is also a command line interface app in the directory `cli` but it currently requires building from source.


## Motivaton

I'm writing this tool mostly for my personal use, but it may be useful to others too.
I often want to quickly draft out some ideas for a scientific project.
For quickly drafted documents with lots of equations, Dropbox Paper seems to be the only usable solution to me due to its inline LaTeX support.
Starting a full LaTeX document just to quickly write up some notes has too much overhead, and all other typesetting or note taking apps that I know of have either no or too limited support for mathematical equations.

If an idea pans out then Dropbox Paper becomes too limiting at some point, and I need a tool to quickly export my notes to a LaTeX document.
[Bluepaper](https://robamler.github.io/bluepaper/) does just that.


## Priorities

Bluepaper's priorities are, in this order:

1. **Ease of use:**
  comes with a client side [web-app](https://robamler.github.io/bluepaper/) that requires no installation, and that optionally talks directly to Dropbox via its REST API.
2. **Hassle free:**
  generates LaTeX code that should always compile on the first try without requiring any fixups.
  If Bluepaper generates LaTeX code that does not compile with `pdflatex` then that's a bug.
  Please [report it](https://github.com/robamler/bluepaper/issues/new).
3. **Clean LaTeX code:**
  generates nicely formatted LaTeX code with a minimal preamble, even if this means that the resulting PDF document may not look very pretty.
  Most users will want to apply their own style sheet anyway, so Bluepaper favors clean and minimalistic LaTeX code over pretty layout.


## State

A "minimal viable product" of the [web app](https://robamler.github.io/bluepaper/) front end is ready.
It can generate LaTeX code from Dropbox Paper documents, either by opening an exported Markdown file or—if the user grants access permissions—by talking directly to Dropbox via a REST API.
It does not yet handle images in the document correctly, but this will come soon.

There's also a draft command line interface application in the directory `cli` that technically works but it's a bit rough around the edges.

### Todo

- [x] Add wasm frontend with basic Markdown --> LaTeX support.
- [x] Use Dropbox REST API in wasm frontend (as an optional feature).
- [ ] Download images included in documents, and include them in the generated LaTeX (convert to a supported format if necessary).


## Legal

- The source code of Bluepaper is released under the [MIT License](LICENSE).
- Dropbox and Dropbox Paper are trademarks or registered trademarks of Dropbox, Inc.
  I'm not affiliated with Dropbox.
