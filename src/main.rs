use pulldown_cmark::{CowStr, Event, LinkType, Options, Parser, Tag};
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter};

const HEADINGS: [(&[u8], bool); 4] = [
    (br"\section{", true),
    (br"\subsection{", true),
    (br"\subsubsection{", true),
    (br"\paragraph{", false),
];

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
    let mut output = BufWriter::new(output);

    let mut ident = 0u32;
    let mut newlines = 0u32;

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::Heading(level)) => {
                write_paragraph_break(&mut output, &mut newlines).unwrap();
                write_inline(
                    &mut output,
                    HEADINGS[(std::cmp::min(level as usize, HEADINGS.len()) - 1)].0,
                    &mut newlines,
                )
                .unwrap();
            }
            Event::End(Tag::Heading(level)) => {
                write_inline(&mut output, b"}", &mut newlines).unwrap();
                if HEADINGS[(std::cmp::min(level as usize, HEADINGS.len()) - 1)].1 {
                    write_paragraph_break(&mut output, &mut newlines).unwrap();
                } else {
                    write_soft_line_break(&mut output, &mut newlines).unwrap();
                }
            }

            Event::Start(Tag::Paragraph) => {}
            Event::End(Tag::Paragraph) => {}

            Event::Start(Tag::BlockQuote) => {
                write_on_single_line(&mut output, br"\begin{quote}", &mut newlines).unwrap();
            }
            Event::End(Tag::BlockQuote) => {
                write_on_single_line(&mut output, br"\end{quote}", &mut newlines).unwrap();
            }

            Event::Start(Tag::CodeBlock(_language)) => {
                // TODO: Use specified language for syntax highlighting.
                // TODO: Use the verbatim package to define a new verbatim environment if the text
                // of the code block contains the string "\end{verbatim}".
                write_on_single_line(&mut output, br"\begin{verbatim}", &mut newlines).unwrap();
                // TODO: set some variable that we're in a code block so that text blocks will be
                // escaped differently.
            }
            Event::End(Tag::CodeBlock(_language)) => {
                write_on_single_line(&mut output, br"\end{verbatim}", &mut newlines).unwrap();
            }

            Event::Start(Tag::List(None)) => {
                write_on_single_line(&mut output, br"\begin{itemize}", &mut newlines).unwrap();
            }
            Event::End(Tag::List(None)) => {
                write_on_single_line(&mut output, br"\end{itemize}", &mut newlines).unwrap();
            }

            Event::Start(Tag::List(Some(_first_number))) => {
                // TODO: use first_number
                write_on_single_line(&mut output, br"\begin{enumerate}", &mut newlines).unwrap();
            }
            Event::End(Tag::List(Some(_first_number))) => {
                write_on_single_line(&mut output, br"\end{enumerate}", &mut newlines).unwrap();
            }

            Event::Start(Tag::Item) => {
                write_on_single_line(&mut output, br"\item", &mut newlines).unwrap();
            }
            Event::End(Tag::Item) => {}

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
                write_inline(&mut output, br"\emph{", &mut newlines).unwrap();
            }
            Event::End(Tag::Emphasis) => {
                write_inline(&mut output, br"}", &mut newlines).unwrap();
            }

            Event::Start(Tag::Strong) => {
                write_inline(&mut output, br"\textbf{", &mut newlines).unwrap();
            }
            Event::End(Tag::Strong) => {
                write_inline(&mut output, br"}", &mut newlines).unwrap();
            }

            Event::Start(Tag::Strikethrough) => {
                // TODO: requires package `ulem`
                write_inline(&mut output, br"\sout{", &mut newlines).unwrap();
            }
            Event::End(Tag::Strikethrough) => {
                write_inline(&mut output, br"}", &mut newlines).unwrap();
            }

            Event::Start(Tag::Link(LinkType::Inline, url, CowStr::Borrowed(""))) => {
                write_inline(&mut output, br"\href{", &mut newlines).unwrap();
                write_inline(&mut output, url.as_bytes(), &mut newlines).unwrap(); // TODO: escape
            }
            Event::End(Tag::Link(LinkType::Inline, _url, CowStr::Borrowed(""))) => {
                write_inline(&mut output, br"}", &mut newlines).unwrap();
            }
            Event::Start(Tag::Link(_, _, _)) => unimplemented!("Non-inline Link"),
            Event::End(Tag::Link(_, _, _)) => panic!(),

            Event::Start(Tag::Image(LinkType::Inline, url, CowStr::Borrowed(""))) => {
                // TODO: download and name image
                write_soft_line_break(&mut output, &mut newlines).unwrap();
                write_inline(
                    &mut output,
                    br"\includegraphics[\textwidth]{",
                    &mut newlines,
                )
                .unwrap();
                write_inline(&mut output, &*url.as_bytes(), &mut newlines).unwrap();
                write_inline(&mut output, b"}", &mut newlines).unwrap();
                write_soft_line_break(&mut output, &mut newlines).unwrap();
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
                write_escaped_inline(&mut output, &text, &mut newlines).unwrap();
            }

            Event::Code(text) => {
                write_inline(&mut output, br"\texttt{", &mut newlines).unwrap();
                write_escaped_inline(&mut output, &text, &mut newlines).unwrap();
                write_inline(&mut output, br"}", &mut newlines).unwrap();
            }

            Event::Html(_) => {
                unimplemented!("Html") // TODO
            }

            Event::SoftBreak => {
                write_soft_line_break(&mut output, &mut newlines).unwrap();
            }

            Event::HardBreak => {
                write_inline(&mut output, b" \\", &mut newlines).unwrap();
                write_soft_line_break(&mut output, &mut newlines).unwrap();
            }

            Event::Rule => {
                write_on_single_line(&mut output, br"\par\noindent\hrulefill\par", &mut newlines)
                    .unwrap();
            }

            Event::TaskListMarker(_checked) => {
                unimplemented!("TaskListMarker") // TODO
            }
        }
    }
}

#[inline(always)]
fn write_escaped_inline(
    output: &mut impl Write,
    text: &str,
    existing_newlines: &mut u32,
) -> std::result::Result<(), std::io::Error> {
    // TODO: implement escaping
    output.write_all(text.as_bytes())?;
    *existing_newlines = 0;
    Ok(())
}

#[inline(always)]
fn write_soft_line_break(
    output: &mut impl Write,
    existing_newlines: &mut u32,
) -> Result<(), std::io::Error> {
    if *existing_newlines == 0 {
        output.write_all(b"\n")?;
        *existing_newlines = 1;
    }

    Ok(())
}

#[inline(always)]
fn write_paragraph_break(
    output: &mut impl Write,
    existing_newlines: &mut u32,
) -> Result<(), std::io::Error> {
    if *existing_newlines < 2 {
        output.write_all(&b"\n\n"[0..(2 - *existing_newlines) as usize])?;
        *existing_newlines = 2;
    }

    Ok(())
}

#[inline(always)]
fn write_inline(
    output: &mut impl Write,
    buf: &[u8],
    existing_newlines: &mut u32,
) -> Result<(), std::io::Error> {
    output.write_all(buf)?;
    *existing_newlines = 0;
    Ok(())
}

#[inline(always)]
fn write_on_single_line(
    output: &mut impl Write,
    buf: &[u8],
    existing_newlines: &mut u32,
) -> Result<(), std::io::Error> {
    if *existing_newlines == 0 {
        output.write_all(b"\n")?;
    }
    output.write_all(buf)?;
    output.write_all(b"\n")?;
    *existing_newlines = 1;
    Ok(())
}

#[cfg(test)]
mod test {}
