use latex_escape::LatexEscaper;
use log::warn;
use std::borrow::Cow;
use std::collections::VecDeque;
use std::io::prelude::*;
use std::ops::Range;

use pulldown_cmark::{CowStr, Event, LinkType, Options, Parser, Tag};

mod latex_escape;

const HEADINGS: [(&str, u32); 4] = [
    (r"\section{", 2),
    (r"\subsection{", 2),
    (r"\subsubsection{", 2),
    (r"\paragraph{", 1),
];

const MAX_ENUMERATE_NESTING: u32 = 4;
const LOWER_ROMAN: [&str; MAX_ENUMERATE_NESTING as usize] = ["i", "ii", "iii", "iv"];

pub fn markdown_to_latex(mut input: String, output: &mut impl Write) -> Result<(), std::io::Error> {
    let mut replacer = Replacer::replace(&mut input);

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    let mut parser = Parser::new_ext(&input, options).into_offset_iter();

    let mut output = LatexEscaper::new(output);
    output.write_str(include_str!("preamble.tex"))?;
    output.limit_newlines(2);
    output.add_newlines(2);

    let mut enumerate_nesting = 0;

    while let Some((event, range)) = parser.next() {
        match event {
            Event::Start(Tag::Heading(level)) => {
                let h = HEADINGS[(std::cmp::min(level as usize, HEADINGS.len()) - 1)];
                output.add_newlines(h.1 + 1);
                output.write_str(h.0)?;
            }
            Event::End(Tag::Heading(level)) => {
                output.write_str("}")?;
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
                output.write_on_single_line(r"\begin{quote}")?;
            }
            Event::End(Tag::BlockQuote) => {
                output.write_on_single_line(r"\end{quote}")?;
            }

            Event::Start(Tag::CodeBlock(_language)) => {
                // TODO: Use specified language for syntax highlighting.
                // TODO: Use the verbatim package to define a new verbatim environment if the text
                // of the code block contains the string "\end{verbatim}".
                output.write_on_single_line(r"\begin{verbatim}")?;
                output.limit_newlines(1);
                // TODO: set some variable that we're in a code block so that text blocks will be
                // escaped differently.
            }
            Event::End(Tag::CodeBlock(_language)) => {
                // No need to write a newline as the contents of code blocks already ends with one.
                output.write_str(r"\end{verbatim}")?;
                output.add_newlines(1);
            }

            Event::Start(Tag::List(None)) => {
                output.write_on_single_line(r"\begin{itemize}")?;
                output.increase_indent();
                output.increase_indent();
            }
            Event::End(Tag::List(None)) => {
                output.decrease_indent();
                output.decrease_indent();
                output.write_on_single_line(r"\end{itemize}")?;
            }

            Event::Start(Tag::List(Some(first_number))) => {
                output.write_on_single_line(r"\begin{enumerate}")?;
                output.increase_indent();

                if first_number != 1 && enumerate_nesting < MAX_ENUMERATE_NESTING {
                    output.write_on_single_line(&format!(
                        r"\setcounter{{enum{}}}{{{}}}",
                        LOWER_ROMAN[enumerate_nesting as usize],
                        first_number as i64 - 1
                    ))?;
                }
                output.increase_indent();
                enumerate_nesting += 1
            }
            Event::End(Tag::List(Some(_first_number))) => {
                output.decrease_indent();
                output.decrease_indent();
                output.write_on_single_line(r"\end{enumerate}")?;
                enumerate_nesting -= 1
            }

            Event::Start(Tag::Item) => {
                output.add_newlines(1);
                output.decrease_indent();
                output.write_str(r"\item ")?;
                output.increase_indent();
                output.limit_newlines(0);
            }
            Event::End(Tag::Item) => {}

            Event::TaskListMarker(checked) => {
                let checkbox = if checked {
                    r"[\checkedbox] "
                } else {
                    r"[\uncheckedbox] "
                };
                output.write_str(checkbox)?;
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
                output.write_str(r"\emph{")?;
            }
            Event::End(Tag::Emphasis) => {
                output.write_str(r"}")?;
            }

            Event::Start(Tag::Strong) => {
                output.write_str(r"\textbf{")?;
            }
            Event::End(Tag::Strong) => {
                output.write_str(r"}")?;
            }

            Event::Start(Tag::Strikethrough) => {
                output.write_str(r"\sout{")?;
            }
            Event::End(Tag::Strikethrough) => {
                output.write_str(r"}")?;
            }

            Event::Start(Tag::Link(LinkType::Inline, url, _title)) => {
                output.write_str(r"\href{")?;
                // TODO: escape [note: unfortunately, cannot un_replace here]
                output.write_str(&url)?;
                output.write_str(r"}{")?;
            }
            Event::End(Tag::Link(LinkType::Inline, _url, _title)) => {
                output.write_str(r"}")?;
            }
            Event::Start(Tag::Link(_, _, _)) => {
                warn!("Ignoring non-inline link (not yet implemented).")
            }
            Event::End(Tag::Link(_, _, _)) => panic!(),

            Event::Start(Tag::Image(LinkType::Inline, url, _title)) => {
                // TODO: download and name image
                output.add_newlines(1);
                output.write_str(r"\includegraphics[width=\textwidth]{")?;
                // TODO: escape [note: unfortunately, cannot un_replace here]
                output.write_str(&url)?;
                output.write_str("}")?;
                output.add_newlines(1);
                if let Some((Event::End(Tag::Image(LinkType::Inline, _, _)), _)) = parser.next() {
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
                output.write_escaped(&replacer.un_replace(text, range.start..range.end))?;
            }

            Event::Code(text) => {
                if replacer.check_if_replacement_point(range.start) {
                    output.write_str("$")?;
                    output.write_str(&replacer.un_replace(text, range.start + 2..range.end - 2))?;
                    output.write_str("$")?;
                } else {
                    output.write_str(r"\texttt{")?;
                    output.write_escaped(&text)?;
                    output.write_str(r"}")?;
                }
            }

            Event::Html(html) => {
                warn!("Found an HTML tag. Including it verbatim in LaTeX output.");
                output.write_str(r"\texttt{")?;
                // Unfortunately, we cannot un_replace here.
                output.write_escaped(&html)?;
                output.write_str(r"}")?;
            }

            Event::SoftBreak => {
                // We should not have to print " \\" here but Dropbox seems to abuse
                // soft line breaks for hard line breaks.
                // TODO: Make behavior dependent on whether or not the input comes from Dropbox.
                output.write_str(r" \\")?;
                output.add_newlines(1);
            }

            Event::HardBreak => {
                output.write_str(r" \\")?;
                output.add_newlines(1);
            }

            Event::Rule => {
                output.write_str(r"\par\noindent\hrulefill\par")?;
            }
        }
    }

    output.limit_newlines(3);
    output.add_newlines(3);
    output.write_on_single_line(r"\end{document}")?;

    Ok(())
}

pub struct Replacer {
    positions: Vec<usize>,
    index: usize,
}

impl Replacer {
    fn replace(input: &mut String) -> Self {
        let mut replacer = Replacer {
            positions: Vec::new(),
            index: 0,
        };

        unsafe {
            let bytes = input.as_bytes_mut();
            if bytes.len() >= 5 {
                // Need at least 5 characters to make a non-empty inline math span: "$$x$$"

                for i in 0..bytes.len() - 1 {
                    if *bytes.get_unchecked(i) == b'$'
                        && *bytes.get_unchecked(i + 1) == b'$'
                        && (i == 0 || *bytes.get_unchecked(i - 1) != b'`')
                        && (i == bytes.len() - 1 || *bytes.get_unchecked(i + 2) != b'`')
                    {
                        replacer.positions.push(i);
                        *bytes.get_unchecked_mut(i) = b'`';
                        *bytes.get_unchecked_mut(i + 1) = b'`';
                    }
                }
            }

            replacer.positions.push(input.len());
            replacer
        }
    }

    pub fn un_replace<'a>(&mut self, s: CowStr<'a>, range: Range<usize>) -> CowStr<'a> {
        unsafe {
            if s.len() == range.len() {
                let mut pos = self.skip_smaller_than(range.start);
                if s.len() >= 2 && pos < range.end - 1 {
                    let mut s = s.into_string();
                    let bytes = s.as_bytes_mut();
                    loop {
                        let b1 = bytes.get_unchecked_mut(pos - range.start);
                        assert_eq!(*b1, b'`');
                        *b1 = b'$';

                        let b2 = bytes.get_unchecked_mut(pos - range.start + 1);
                        assert_eq!(*b2, b'`');
                        *b2 = b'$';

                        self.index += 1;
                        pos = self.positions[self.index];
                        if pos >= range.end - 1 {
                            return s.into();
                        }
                    }
                } else {
                    // Nothing to replace.
                    return s;
                }
            }
        }

        warn!("Some output text may contain spurious '`' characters instead of '$'.");
        s
    }

    fn skip_smaller_than(&mut self, pos: usize) -> usize {
        unsafe {
            let remainder = self.positions.get_unchecked(self.index..);
            let (num_skip, p) = remainder
                .iter()
                .cloned()
                .enumerate()
                .find(|(_, x)| *x >= pos)
                .unwrap();
            self.index += num_skip;
            p
        }
    }

    fn check_if_replacement_point(&mut self, pos: usize) -> bool {
        self.skip_smaller_than(pos) == pos
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn range_mapping() {
        let parser = Parser::new("text `` math  `` text").into_offset_iter();
        for (event, range) in parser {
            dbg!(event);
            dbg!(range);
        }
    }
}
