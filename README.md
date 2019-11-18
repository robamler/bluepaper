# Bluepaper

Bluepaper is a small web app that exports Dropbox Paper documents to valid and clean LaTeX code.

- **Try it out here:** [https://robamler.github.io/bluepaper/](https://robamler.github.io/bluepaper/)

Bluepaper uses the excellent [pulldown-cmark](https://crates.io/crates/pulldown-cmark) Rust crate with some minor tweaks to parse some Dropbox-specific formatting like LaTeX equations.
Bluepaper also automatically downloads included images and packages them in a `.zip` file together with the generated LaTeX code.

The [Bluepaper web app](https://robamler.github.io/bluepaper/) runs entirely client side using WebAssembly.
There is also a proof-of-concept command line interface app in the directory `cli` but it requires building from source and it is not actively maintained (volunteers welcome).


## Status

The [Bluepaper web app](https://robamler.github.io/bluepaper/) should work on Firefox and Chrome for most Dropbox Paper documents but it isn't tested extensively yet.
If you find a bug, please [report it](https://github.com/robamler/bluepaper/issues/new).
Bluepaper may not work on Safari yet, but this should be easy to fix.


## Motivaton

I wrote this tool mostly for my personal use, but it may be useful to others too.
I often want to quickly draft out some ideas for a scientific project.
For quickly drafted documents with lots of equations, Dropbox Paper seems to be the only usable solution to me due to its inline LaTeX support.
Starting a full LaTeX document just to quickly write up some notes has too much overhead, and all other typesetting or note taking apps that I know of have either no or too limited support for mathematical equations.

If an idea pans out then Dropbox Paper becomes too limiting at some point, and I need a tool to quickly export my notes to a LaTeX document.
[Bluepaper](https://robamler.github.io/bluepaper/) does just that.


## Todo

- [x] Add wasm frontend with basic Markdown --> LaTeX support.
- [x] Use Dropbox REST API in wasm frontend (as an optional feature).
- [ ] Support included images:
  - [x] Download `.png` and `.jp(e)g` images and package them in a `.zip` file together with the generated LaTeX.
  - [ ] Download `.svg` images and convert them to `.png`.
- [ ] Polyfill JavaScript features that aren't supported by Safari (mostly around fetch API).


## Legal

- The source code of Bluepaper is released under the [MIT License](LICENSE).
- Dropbox and Dropbox Paper are trademarks or registered trademarks of Dropbox, Inc.
  I'm not affiliated with Dropbox.
