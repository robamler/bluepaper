use latex_escape::LatexEscaper;
use pulldown_cmark::{CowStr, Event, LinkType, Options, Parser, Tag};
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter};

mod latex_escape;

const HEADINGS: [(&[u8], u32); 4] = [
    (br"\section{", 2),
    (br"\subsection{", 2),
    (br"\subsubsection{", 2),
    (br"\paragraph{", 1),
];

const MAX_ENUMERATE_NESTING: u32 = 4;
const LOWER_ROMAN: [&str; MAX_ENUMERATE_NESTING as usize] = ["i", "ii", "iii", "iv"];

fn main() {
    let file = File::open("sample.md").unwrap();
    let mut file = BufReader::new(file);
    let mut input = String::new();
    file.read_to_string(&mut input).unwrap();
    drop(file);

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    let mut parser = Parser::new_ext(&input, options);

    let output = std::io::stdout();
    let output = BufWriter::new(output);
    let mut output = LatexEscaper::new(output);

    let mut enumerate_nesting = 0;

    while let Some(event) = parser.next() {
        {
            #![cfg(debug_assertions)]
            dbg!(&event);
        }

        match event {
            Event::Start(Tag::Heading(level)) => {
                let h = HEADINGS[(std::cmp::min(level as usize, HEADINGS.len()) - 1)];
                output.add_newlines(h.1 + 1);
                output.write_all(h.0).unwrap();
            }
            Event::End(Tag::Heading(level)) => {
                output.write_all(b"}").unwrap();
                let h = HEADINGS[(std::cmp::min(level as usize, HEADINGS.len()) - 1)];
                output.add_newlines(h.1);
                output.limit_newlines(2); // Handles case of multiple consecutive headers
            }

            Event::Start(Tag::Paragraph) => {
                output.add_newlines(2);
            }
            Event::End(Tag::Paragraph) => {
                output.add_newlines(2);
            }

            Event::Start(Tag::BlockQuote) => {
                output.on_single_line(br"\begin{quote}").unwrap();
            }
            Event::End(Tag::BlockQuote) => {
                output.on_single_line(br"\end{quote}").unwrap();
            }

            Event::Start(Tag::CodeBlock(_language)) => {
                // TODO: Use specified language for syntax highlighting.
                // TODO: Use the verbatim package to define a new verbatim environment if the text
                // of the code block contains the string "\end{verbatim}".
                output.on_single_line(br"\begin{verbatim}").unwrap();
                output.limit_newlines(1);
                // TODO: set some variable that we're in a code block so that text blocks will be
                // escaped differently.
            }
            Event::End(Tag::CodeBlock(_language)) => {
                // No need to write a newline as the contents of code blocks already ends with one.
                output.write_all(br"\end{verbatim}").unwrap();
                output.add_newlines(1);
            }

            Event::Start(Tag::List(None)) => {
                output.on_single_line(br"\begin{itemize}").unwrap();
                output.increase_indent();
                output.increase_indent();
            }
            Event::End(Tag::List(None)) => {
                output.decrease_indent();
                output.decrease_indent();
                output.on_single_line(br"\end{itemize}").unwrap();
            }

            Event::Start(Tag::List(Some(first_number))) => {
                output.on_single_line(br"\begin{enumerate}").unwrap();
                output.increase_indent();

                if first_number != 1 && enumerate_nesting < MAX_ENUMERATE_NESTING {
                    output
                        .on_single_line(
                            format!(
                                r"\setcounter{{enum{}}}{{{}}}",
                                LOWER_ROMAN[enumerate_nesting as usize],
                                first_number as i64 - 1
                            )
                            .as_bytes(),
                        )
                        .unwrap();
                }
                output.increase_indent();
                enumerate_nesting += 1
            }
            Event::End(Tag::List(Some(_first_number))) => {
                output.decrease_indent();
                output.decrease_indent();
                output.on_single_line(br"\end{enumerate}").unwrap();
                enumerate_nesting -= 1
            }

            Event::Start(Tag::Item) => {
                output.add_newlines(1);
                output.decrease_indent();
                output.write_all(br"\item ").unwrap();
                output.increase_indent();
                output.limit_newlines(0);
            }
            Event::End(Tag::Item) => {}

            Event::TaskListMarker(checked) => {
                // TODO: in preamble, define `\checkedbox` as:
                // \mbox{\ooalign{$\square$\cr\hidewidth\raisebox{.45ex}{\hspace{0.2em}$\checkmark$}\hidewidth\cr}}
                // and `\uncheckedbox` as $\square$
                let code: &[u8] = if checked {
                    br"[\checkedbox] "
                } else {
                    br"[\uncheckedbox] "
                };
                output.write_all(code).unwrap();
                output.limit_newlines(0);
            }

            Event::Start(Tag::FootnoteDefinition(_)) => {
                unimplemented!("FootnoteDefinition") // TODO
            }
            Event::End(Tag::FootnoteDefinition(_)) => {
                unimplemented!("FootnoteDefinition") // TODO
            }

            Event::FootnoteReference(_) => {
                unimplemented!("FootnoteReference") // TODO
            }

            Event::Start(Tag::Table(..)) => {
                unimplemented!("Table"); // TODO
            }
            Event::End(Tag::Table(..)) => {
                unimplemented!("Table"); // TODO
            }

            Event::Start(Tag::TableHead) => {
                unimplemented!("TableHead"); // TODO
            }
            Event::End(Tag::TableHead) => {
                unimplemented!("TableHead"); // TODO
            }

            Event::Start(Tag::TableRow) => {
                unimplemented!("TableRow"); // TODO
            }
            Event::End(Tag::TableRow) => {
                unimplemented!("TableRow"); // TODO
            }

            Event::Start(Tag::TableCell) => {
                unimplemented!("TableCell"); // TODO
            }
            Event::End(Tag::TableCell) => {
                unimplemented!("TableCell"); // TODO
            }

            Event::Start(Tag::Emphasis) => {
                output.write_all(br"\emph{").unwrap();
            }
            Event::End(Tag::Emphasis) => {
                output.write_all(br"}").unwrap();
            }

            Event::Start(Tag::Strong) => {
                output.write_all(br"\textbf{").unwrap();
            }
            Event::End(Tag::Strong) => {
                output.write_all(br"}").unwrap();
            }

            Event::Start(Tag::Strikethrough) => {
                // TODO: requires package `ulem`
                output.write_all(br"\sout{").unwrap();
            }
            Event::End(Tag::Strikethrough) => {
                output.write_all(br"}").unwrap();
            }

            Event::Start(Tag::Link(LinkType::Inline, url, CowStr::Borrowed(""))) => {
                output.write_all(br"\href{").unwrap();
                output.write_all(url.as_bytes()).unwrap(); // TODO: escape
            }
            Event::End(Tag::Link(LinkType::Inline, _url, CowStr::Borrowed(""))) => {
                output.write_all(br"}").unwrap();
            }
            Event::Start(Tag::Link(_, _, _)) => unimplemented!("Non-inline Link"),
            Event::End(Tag::Link(_, _, _)) => panic!(),

            Event::Start(Tag::Image(LinkType::Inline, url, CowStr::Borrowed(""))) => {
                // TODO: download and name image
                output.add_newlines(1);
                output
                    .write_all(br"\includegraphics[width=\textwidth]{")
                    .unwrap();
                output.write_all(&*url.as_bytes()).unwrap();
                output.write_all(b"}").unwrap();
                output.add_newlines(1);
                assert_eq!(
                    parser.next(),
                    Some(Event::End(Tag::Image(
                        LinkType::Inline,
                        url,
                        CowStr::from("")
                    )))
                );
            }
            Event::End(Tag::Image(_, _, _)) => panic!(),
            Event::Start(Tag::Image(_, _, _)) => unimplemented!("Non-inline Image"),

            Event::Text(text) => {
                output.write_all_escaped(&text).unwrap();
            }

            Event::Code(text) => {
                output.write_all(br"\texttt{").unwrap();
                output.write_all_escaped(&text).unwrap();
                output.write_all(br"}").unwrap();
            }

            Event::Html(html) => {
                // TODO: output a warning that HTML conversion is not implemented.
                output.write_all(br"\texttt{").unwrap();
                output.write_all_escaped(&html).unwrap();
                output.write_all(br"}").unwrap();
            }

            Event::SoftBreak => {
                output.add_newlines(1);
            }

            Event::HardBreak => {
                output.write_all(b" \\").unwrap();
                output.add_newlines(1);
            }

            Event::Rule => {
                output.write_all(br"\par\noindent\hrulefill\par").unwrap();
            }
        }
    }

    output.get_mut().write_all(b"\n").unwrap();
}
