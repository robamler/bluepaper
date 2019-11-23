//! A library to convert "Dropbox flavored" markdown to clean LaTeX code.
//!
//! This library is the common backend of the `bluepaper_cli` and the
//! `bluepaper_wasm` creates, with which it shares a cargo workspace. The three
//! crates are separated because cargo makes it a bit difficult to build both a wasm
//! and a non-wasm binary from a single crate.
//!
//! The main struct in this library is
//! [`MarkdownToLatex`](struct.MarkdownToLatex.html).

pub mod format;
pub mod latex_escape;
mod preprocess;

use format::WhitespaceFormatter;
use latex_escape::escape_str;
use preprocess::Replacer;

use log::{info, warn};
use std::io::prelude::*;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag};

const HEADINGS: [(&[u8], u32); 4] = [
    (br"\section{", 2),
    (br"\subsection{", 2),
    (br"\subsubsection{", 2),
    (br"\paragraph{", 1),
];

const MAX_ENUMERATE_NESTING: u32 = 4;
const LOWER_ROMAN: [&str; MAX_ENUMERATE_NESTING as usize] = ["i", "ii", "iii", "iv"];

/// A converter from "Dropbox flavoured" markdown to clean LaTeX code.
///
/// # Examples
///
/// Converting a `String` of markdown into a `String` of LaTeX:
/// ```
/// let markdown = "# Title\n\nText with *emphasis* and $$m_a^{th}$$.".to_string();
/// let latex = bluepaper_core::MarkdownToLatex::from_string(markdown).into_string();
///
/// assert_eq!(&latex[..24], "\\documentclass{article}\n");
/// assert!(latex
///     .find("\\section{Title}\n\nText with \\emph{emphasis} and $m_a^{th}$.")
///     .is_some());
/// assert_eq!(&latex[latex.len() - 15..], "\\end{document}\n");
/// ```
///
/// Writing LaTeX to an arbitrary writer (i.e., anything that implements
/// `std::io::Write`):
/// ```
/// let markdown = "# Title\n\nText with a `code span`.".to_string();
/// let mut converter = bluepaper_core::MarkdownToLatex::from_string(markdown);
///
/// // `MarkdownToLatex` can write to any writer. In a real application, `writer`
/// // could, e.g., be a `std::io::BufWriter<std::fs::File>`. We use a `Vec<u8>`
/// // here just for demonstration. Don't write to a `Vec<u8>` in a real
/// // application, call `converter.into_string()` instead.
/// let writer = Vec::<u8>::new();
///
/// // `MarkdownToLatex::write_to` takes a writer, writes LaTeX to it,
/// // and returns the writer back. It consumes the converter.
/// let writer = converter.write_to(writer).unwrap();
///
/// let latex = String::from_utf8(writer).unwrap();
/// assert_eq!(&latex[..24], "\\documentclass{article}\n");
/// assert!(latex
///     .find("\\section{Title}\n\nText with a \\texttt{code span}.")
///     .is_some());
/// assert_eq!(&latex[latex.len() - 15..], "\\end{document}\n");
/// ```
pub struct MarkdownToLatex {
    preprocessed: String,
    replacer: Replacer,
}

impl MarkdownToLatex {
    /// Creates a new converter from a `String` of markdown.
    ///
    /// Takes ownership of `markdown` because it has to do some in-place
    /// preprocessing to handle LaTeX equations.
    pub fn from_string(mut markdown: String) -> Self {
        let replacer = Replacer::replace(&mut markdown);
        Self {
            replacer,
            preprocessed: markdown,
        }
    }

    /// Consumes the converter and returns a `String` of LaTeX code without images.
    ///
    /// Comments out any generated `\includegraphics`. If you would like to generate
    /// uncommented `\includegraphics`, use
    /// [`into_string_with_image_callback`](#method.into_string_with_image_callback).
    pub fn into_string(self) -> String {
        self.into_string_with_image_callback(&mut |_| None)
    }

    /// Consumes the converter and returns a `String` of LaTeX code with images.
    ///
    /// Calls the callback `f` for each encountered image. The argument of `f` is the
    /// URL of the image and `f` must return the path to an image file and a bool. If
    /// the bool returned by `f` is `true` then an uncommented `\includegraphics` will
    /// be generated. If it is `false`, then the `\includegraphics` will be generated
    /// but commented out.
    pub fn into_string_with_image_callback(
        self,
        f: &mut dyn FnMut(&str) -> Option<String>,
    ) -> String {
        unsafe {
            let mut latex = Vec::new();
            self.write_to_with_image_callback(&mut latex, f).unwrap();
            String::from_utf8_unchecked(latex)
        }
    }

    /// Consumes the converter and writes LaTeX code without images to `writer`.
    ///
    /// Comments out any generated `\includegraphics`. If you would like to generate
    /// uncommented `\includegraphics`, use
    /// [`write_to_with_image_callback`](#method.write_to_with_image_callback).
    ///
    /// Hands back ownership of the writer when it's done. The written output is
    /// guaranteed to be valid UTF-8.
    pub fn write_to<W: Write>(self, writer: W) -> std::io::Result<W> {
        self.write_to_with_image_callback(writer, &mut |_| None)
    }

    /// Consumes the converter and writes LaTeX with images code to `writer`.
    ///
    /// Calls the `image_callback` for each encountered image. The argument of
    /// `image_callback` is the URL of the image and `image_callback` must return the
    /// path to an image file and a bool. If the bool returned by `image_callback` is
    /// `true` then an uncommented `\includegraphics` will be generated. If it is
    /// `false`, then the `\includegraphics` will be generated but commented out.
    ///
    /// Hands back ownership of the writer when it's done. The written output is
    /// guaranteed to be valid UTF-8.
    pub fn write_to_with_image_callback<W: Write>(
        mut self,
        mut writer: W,
        image_callback: &mut dyn FnMut(&str) -> Option<String>,
    ) -> std::io::Result<W> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);
        let mut parser = Parser::new_ext(&self.preprocessed, options).into_offset_iter();

        writer.write_all(include_bytes!("preamble.tex"))?;
        let mut writer = WhitespaceFormatter::new(writer);
        writer.limit_newlines(2);
        writer.add_newlines(2);

        let mut enumerate_nesting = 0;

        while let Some((event, range)) = parser.next() {
            match event {
                Event::Start(Tag::Heading(level)) => {
                    let h = HEADINGS[(std::cmp::min(level as usize, HEADINGS.len()) - 1)];
                    writer.add_newlines(h.1 + 1);
                    writer.write_all(h.0)?;
                }
                Event::End(Tag::Heading(level)) => {
                    writer.write_all(b"}")?;
                    let h = HEADINGS[(std::cmp::min(level as usize, HEADINGS.len()) - 1)];
                    writer.add_newlines(h.1);
                    writer.limit_newlines(2); // Handles case of multiple consecutive headers
                }

                Event::Start(Tag::Paragraph) => {
                    writer.add_newlines(2);
                }
                Event::End(Tag::Paragraph) => {
                    writer.add_newlines(2);
                }

                Event::Start(Tag::BlockQuote) => {
                    writer.write_on_single_line(r"\begin{quote}")?;
                }
                Event::End(Tag::BlockQuote) => {
                    writer.write_on_single_line(r"\end{quote}")?;
                }

                Event::Start(Tag::CodeBlock(_language)) => {
                    // TODO: Use the verbatim package to define a new verbatim environment if the
                    //       text of the code block contains the string "\end{verbatim}".
                    writer.write_on_single_line(r"\begin{verbatim}")?;
                    writer.limit_newlines(1);
                    // TODO: Check if there's always exactly one text block inside a code block.
                    //       If that's the case, process it here and properly escape it.
                }
                Event::End(Tag::CodeBlock(_language)) => {
                    // No newline before `\end{verbatim}` because the contents of the code block
                    // already ended with one.
                    writer.write_all(br"\end{verbatim}")?;
                    writer.add_newlines(1);
                }

                Event::Start(Tag::List(None)) => {
                    writer.write_on_single_line(r"\begin{itemize}")?;
                    writer.increase_indent();
                    writer.increase_indent();
                }
                Event::End(Tag::List(None)) => {
                    writer.decrease_indent();
                    writer.decrease_indent();
                    writer.write_on_single_line(r"\end{itemize}")?;
                }

                Event::Start(Tag::List(Some(first_number))) => {
                    writer.write_on_single_line(r"\begin{enumerate}")?;
                    writer.increase_indent();

                    if first_number != 1 && enumerate_nesting < MAX_ENUMERATE_NESTING {
                        writer.write_on_single_line(&format!(
                            r"\setcounter{{enum{}}}{{{}}}",
                            LOWER_ROMAN[enumerate_nesting as usize],
                            first_number as i64 - 1
                        ))?;
                    }
                    writer.increase_indent();
                    enumerate_nesting += 1
                }
                Event::End(Tag::List(Some(_first_number))) => {
                    writer.decrease_indent();
                    writer.decrease_indent();
                    writer.write_on_single_line(r"\end{enumerate}")?;
                    enumerate_nesting -= 1
                }

                Event::Start(Tag::Item) => {
                    writer.add_newlines(1);
                    writer.decrease_indent();
                    writer.write_all(br"\item ")?;
                    writer.increase_indent();
                    writer.limit_newlines(0);
                }
                Event::End(Tag::Item) => {}

                Event::TaskListMarker(checked) => {
                    let checkbox = if checked {
                        &br"[\checkedbox] "[..]
                    } else {
                        &br"[\uncheckedbox] "[..]
                    };
                    writer.write_all(checkbox)?;
                    writer.limit_newlines(0);
                }

                Event::Start(Tag::FootnoteDefinition(_)) => {
                    warn!("Ignoring footnote definition (not yet implemented).") // TODO
                }
                Event::End(Tag::FootnoteDefinition(_)) => {}

                Event::FootnoteReference(_) => {
                    warn!("Ignoring footnote reference (not yet implemented).") // TODO
                }

                Event::Start(Tag::Table(..)) => {
                    warn!("Ignoring table (not yet implemented)."); // TODO
                }
                Event::End(Tag::Table(..)) => {}

                Event::Start(Tag::TableHead) => {}
                Event::End(Tag::TableHead) => {}

                Event::Start(Tag::TableRow) => {}
                Event::End(Tag::TableRow) => {}

                Event::Start(Tag::TableCell) => {}
                Event::End(Tag::TableCell) => {}

                Event::Start(Tag::Emphasis) => {
                    writer.write_all(br"\emph{")?;
                }
                Event::End(Tag::Emphasis) => {
                    writer.write_all(br"}")?;
                }

                Event::Start(Tag::Strong) => {
                    writer.write_all(br"\textbf{")?;
                }
                Event::End(Tag::Strong) => {
                    writer.write_all(br"}")?;
                }

                Event::Start(Tag::Strikethrough) => {
                    writer.write_all(br"\sout{")?;
                }
                Event::End(Tag::Strikethrough) => {
                    writer.write_all(br"}")?;
                }

                Event::Start(Tag::Link(LinkType::Inline, url, _title)) => {
                    writer.write_all(br"\href{")?;
                    // TODO: escape url (Note: unfortunately, we cannot un_replace here).
                    writer.write_all(url.as_bytes())?;
                    writer.write_all(br"}{")?;
                }
                Event::End(Tag::Link(LinkType::Inline, _url, _title)) => {
                    writer.write_all(br"}")?;
                }
                Event::Start(Tag::Link(_, _, _)) => {
                    warn!("Ignoring non-inline link (not yet implemented).")
                }
                Event::End(Tag::Link(_, _, _)) => panic!(),

                Event::Start(Tag::Image(LinkType::Inline, url, _title)) => {
                    writer.add_newlines(1);
                    let filename = image_callback(url.as_ref());
                    let filename = if let Some(ref filename) = filename {
                        filename.as_str()
                    } else {
                        writer.write_all(b"%")?;
                        url.as_ref()
                    };
                    writer.write_all(br"\includegraphics[width=\textwidth]{")?;
                    // TODO: escape url  (Note: unfortunately, we cannot un_replace here).
                    writer.write_all(filename.as_bytes())?;
                    writer.write_all(b"}")?;
                    writer.add_newlines(1);
                    if let Some((Event::End(Tag::Image(LinkType::Inline, ..)), _)) = parser.next() {
                        // OK.
                    } else {
                        panic!("Unclosed inline link.")
                    }
                }
                Event::End(Tag::Image(_, _, _)) => panic!(),
                Event::Start(Tag::Image(_, _, _)) => {
                    warn!("Ignoring non-inline image (not yet implemented).")
                }

                Event::Text(text) => {
                    let text = self.replacer.un_replace(text, range.start..range.end);
                    let inner_writer = writer.get_mut()?;
                    escape_str(&text, inner_writer)?;
                }

                Event::Code(text) => {
                    if self.replacer.check_if_replacement_point(range.start)
                        && range.end > 2
                        && self.replacer.check_if_replacement_point(range.end - 2)
                    {
                        let math = self
                            .replacer
                            .un_replace(text, range.start + 2..range.end - 2);
                        writer.write_all(br"$")?;
                        writer.write_all(math.as_bytes())?;
                        writer.write_all(br"$")?;
                    } else {
                        let inner_writer = writer.get_mut()?;
                        inner_writer.write_all(br"\texttt{")?;
                        // TODO: un_replace
                        escape_str(&text, inner_writer)?;
                        inner_writer.write_all(br"}")?;
                    }
                }

                Event::Html(html) => {
                    info!("Found an HTML tag. Including it verbatim in LaTeX writer.");
                    let html = self.replacer.un_replace(html, range.start..range.end);
                    let inner_writer = writer.get_mut()?;
                    inner_writer.write_all(br"\texttt{")?;
                    escape_str(&html, inner_writer)?;
                    inner_writer.write_all(br"}")?;
                }

                Event::SoftBreak => {
                    // We should not have to print " \\" here but Dropbox seems to abuse
                    // soft line breaks for hard line breaks.
                    writer.write_all(br" \\")?;
                    writer.add_newlines(1);
                }

                Event::HardBreak => {
                    writer.write_all(br" \\")?;
                    writer.add_newlines(1);
                }

                Event::Rule => {
                    writer.write_all(br"\par\noindent\hrulefill\par")?;
                }
            }
        }

        writer.limit_newlines(3);
        writer.add_newlines(3);
        writer.write_all(b"\\end{document}\n")?;

        writer.into_inner()
    }
}
