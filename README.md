# Bluepaper

Bluepaper is a small web app that exports Dropbox Paper documents to valid and clean LaTeX code.
It also downloads included images, and packages them together with the LaTeX code in a `.zip` file.

- **Try it out here:** [Bluepaper Web App](https://robamler.github.io/bluepaper/)

Bluepaper runs entirely on the client side using JavaScript WebAssembly (WASM).
There is also a proof-of-concept command line interface app in the directory `cli` but it requires building from source and it is not actively maintained (volunteers welcome).


## Motivaton

I wrote this tool mostly for my personal use, but it may be useful to others too.
I often want to quickly draft out some ideas for a scientific project.
For quickly drafted documents with lots of equations, Dropbox Paper seems to be the only usable solution to me due to its inline LaTeX support.
Starting a full LaTeX document just to quickly write up some notes has too much overhead, and all other typesetting or note taking apps that I know of have either no or too limited support for mathematical equations.

If an idea pans out then Dropbox Paper becomes too limiting at some point, and I need a tool to quickly export my notes to a LaTeX document.
[Bluepaper](https://robamler.github.io/bluepaper/) does just that.


## Legal

- The source code of Bluepaper is released under the [MIT License](LICENSE).
- Dropbox and Dropbox Paper are trademarks or registered trademarks of Dropbox, Inc.
  I'm not affiliated with Dropbox.
