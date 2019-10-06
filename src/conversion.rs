use std::io::prelude::*;
use log::{debug, warn};
use latex_escape::LatexEscaper;
use pulldown_cmark::{CowStr, Event, LinkType, Options, Parser, Tag};

mod latex_escape;

const HEADINGS: [(&[u8], u32); 4] = [
    (br"\section{", 2),
    (br"\subsection{", 2),
    (br"\subsubsection{", 2),
    (br"\paragraph{", 1),
];

const MAX_ENUMERATE_NESTING: u32 = 4;
const LOWER_ROMAN: [&str; MAX_ENUMERATE_NESTING as usize] = ["i", "ii", "iii", "iv"];


pub fn markdown_to_latex(input: &str, output: &mut impl Write) -> Result<(), std::io::Error> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    let mut parser = Parser::new_ext(input, options);

    let mut output = LatexEscaper::new(output);

    let mut enumerate_nesting = 0;

    while let Some(event) = parser.next() {
        debug!("event: {:?}", &event);

        match event {
            Event::Start(Tag::Heading(level)) => {
                let h = HEADINGS[(std::cmp::min(level as usize, HEADINGS.len()) - 1)];
                output.add_newlines(h.1 + 1);
                output.write_all(h.0)?;
            }
            Event::End(Tag::Heading(level)) => {
                output.write_all(b"}")?;
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
                output.on_single_line(br"\begin{quote}")?;
            }
            Event::End(Tag::BlockQuote) => {
                output.on_single_line(br"\end{quote}")?;
            }

            Event::Start(Tag::CodeBlock(_language)) => {
                // TODO: Use specified language for syntax highlighting.
                // TODO: Use the verbatim package to define a new verbatim environment if the text
                // of the code block contains the string "\end{verbatim}".
                output.on_single_line(br"\begin{verbatim}")?;
                output.limit_newlines(1);
                // TODO: set some variable that we're in a code block so that text blocks will be
                // escaped differently.
            }
            Event::End(Tag::CodeBlock(_language)) => {
                // No need to write a newline as the contents of code blocks already ends with one.
                output.write_all(br"\end{verbatim}")?;
                output.add_newlines(1);
            }

            Event::Start(Tag::List(None)) => {
                output.on_single_line(br"\begin{itemize}")?;
                output.increase_indent();
                output.increase_indent();
            }
            Event::End(Tag::List(None)) => {
                output.decrease_indent();
                output.decrease_indent();
                output.on_single_line(br"\end{itemize}")?;
            }

            Event::Start(Tag::List(Some(first_number))) => {
                output.on_single_line(br"\begin{enumerate}")?;
                output.increase_indent();

                if first_number != 1 && enumerate_nesting < MAX_ENUMERATE_NESTING {
                    output.on_single_line(
                        format!(
                            r"\setcounter{{enum{}}}{{{}}}",
                            LOWER_ROMAN[enumerate_nesting as usize],
                            first_number as i64 - 1
                        )
                        .as_bytes(),
                    )?;
                }
                output.increase_indent();
                enumerate_nesting += 1
            }
            Event::End(Tag::List(Some(_first_number))) => {
                output.decrease_indent();
                output.decrease_indent();
                output.on_single_line(br"\end{enumerate}")?;
                enumerate_nesting -= 1
            }

            Event::Start(Tag::Item) => {
                output.add_newlines(1);
                output.decrease_indent();
                output.write_all(br"\item ")?;
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
                output.write_all(code)?;
                output.limit_newlines(0);
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
                output.write_all(br"\emph{")?;
            }
            Event::End(Tag::Emphasis) => {
                output.write_all(br"}")?;
            }

            Event::Start(Tag::Strong) => {
                output.write_all(br"\textbf{")?;
            }
            Event::End(Tag::Strong) => {
                output.write_all(br"}")?;
            }

            Event::Start(Tag::Strikethrough) => {
                // TODO: requires package `ulem`
                output.write_all(br"\sout{")?;
            }
            Event::End(Tag::Strikethrough) => {
                output.write_all(br"}")?;
            }

            Event::Start(Tag::Link(LinkType::Inline, url, CowStr::Borrowed(""))) => {
                output.write_all(br"\href{")?;
                output.write_all(url.as_bytes())?; // TODO: escape
            }
            Event::End(Tag::Link(LinkType::Inline, _url, CowStr::Borrowed(""))) => {
                output.write_all(br"}")?;
            }
            Event::Start(Tag::Link(_, _, _)) => {
                warn!("Ignoring non-inline link (not yet implemented).")
            }
            Event::End(Tag::Link(_, _, _)) => panic!(),

            Event::Start(Tag::Image(LinkType::Inline, url, CowStr::Borrowed(""))) => {
                // TODO: download and name image
                output.add_newlines(1);
                output.write_all(br"\includegraphics[width=\textwidth]{")?;
                output.write_all(&*url.as_bytes())?;
                output.write_all(b"}")?;
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
            Event::Start(Tag::Image(_, _, _)) => {
                warn!("Ignoring non-inline image (not yet implemented).")
            }

            Event::Text(text) => {
                output.write_all_escaped(&text)?;
            }

            Event::Code(text) => {
                output.write_all(br"\texttt{")?;
                output.write_all_escaped(&text)?;
                output.write_all(br"}")?;
            }

            Event::Html(html) => {
                warn!("Found an HTML tag. Including it verbatim in LaTeX output.");
                output.write_all(br"\texttt{")?;
                output.write_all_escaped(&html)?;
                output.write_all(br"}")?;
            }

            Event::SoftBreak => {
                output.add_newlines(1);
            }

            Event::HardBreak => {
                output.write_all(b" \\")?;
                output.add_newlines(1);
            }

            Event::Rule => {
                output.write_all(br"\par\noindent\hrulefill\par")?;
            }
        }
    }

    output.get_mut().write_all(b"\n")?;

    Ok(())
}
