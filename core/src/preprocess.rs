use log::warn;
use pulldown_cmark::CowStr;

use std::ops::Range;

pub struct Replacer {
    positions: Vec<usize>,
    index: usize,
}

/// A Preprocessor for "Dropbox flavoured" markdown to turn it into CommonMark.
impl Replacer {
    /// Replace LaTeX deliminators "$$" with "``" and remember replacement points.
    ///
    /// Mutates the input string in place. These replacements ensure that that
    /// pulldown-cmark treats the content as code and does not misinterpret, e.g.,
    /// underscores "_" or stars "*" in LaTeX code for emphasis markers. This
    /// approach to treating math blocks is not perfect and can be tricked, but
    /// the possible ambiguities are unlikely to appear unintentionally.
    ///
    /// Returns a struct that internally contains a list of the replacement points.
    /// This serves two purposes:
    /// - It allows to check if a code block emitted by the parser is a regular
    ///   code block or a math span turned into a code block during preprocessing
    ///   (using `check_if_replacement_point`).
    /// - It allows to revert preprocessing on substrings in case any unpaired "$$"
    ///   existed.
    ///
    /// See unit tests for more.
    pub fn replace(input: &mut String) -> Self {
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

            // Push end sentinel so that we don't have to check for out-of-bounds.
            replacer.positions.push(input.len());
            replacer
        }
    }

    /// Check if, at position `pos` and `pos + 1`, a "$$" was replaced by "``".
    /// This consumes all memorized replacement points before `pos` so that
    /// it runs in O(n) instead of O(n^2) time. Thus, always call with with
    /// increasing `pos` values.
    pub fn check_if_replacement_point(&mut self, pos: usize) -> bool {
        self.skip_smaller_than(pos) == pos
    }

    /// Revert any possible replacements performed within a given substring `s`
    /// of the preprocessed markdown text. `range` must be the correct range,
    /// i.e., if the `Replacer` was created from an input string `input`, then
    /// `s` must be the preprocessed portion of `input[range]`. Consumes all
    /// memorized replacement points up to the end of the range, consecutive
    /// calls must not jump back. Allocates only
    pub fn un_replace<'a>(&mut self, s: CowStr<'a>, range: Range<usize>) -> CowStr<'a> {
        if s.len() != range.len() {
            warn!(
                "Ambiguous use of \"$$\"$. Some generated text may contain spurious \"`\" \
                 characters instead of \"$\"."
            );
            return s;
        }

        let mut pos = self.skip_smaller_than(range.start);
        if s.len() >= 2 && pos < range.end - 1 {
            unsafe {
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
                    pos = self.positions[self.index]; // Cannot fail because of end sentinel.
                    if pos >= range.end - 1 {
                        return s.into();
                    }
                }
            }
        } else {
            // Nothing to replace.
            s
        }
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn replacer() {
        let mut markdown = "Text $$math$$ text ``code in double carets`` text.\n\
                            A costs $1 and B costs $3 and ``it's all $$ expensive''."
            .to_string();
        let mut replacer = Replacer::replace(&mut markdown);

        // `Replacer::replace` replaces all occurrences of $$, even ones that don't pair up.
        assert_eq!(
            markdown,
            "Text ``math`` text ``code in double carets`` text.\n\
             A costs $1 and B costs $3 and ``it's all `` expensive''."
        );

        // Use `check_if_replacement_point` to figure out if a code block is genuine
        // or a result of preprocessing (i.e., a math span).
        assert!(replacer.check_if_replacement_point(5));
        assert!(replacer.check_if_replacement_point(11));

        // `check_if_replacement_point` consumes all replacement points up to the
        // point in question so that it's runtime is O(n) and not O(n^2).
        assert!(!replacer.check_if_replacement_point(5));

        // Not all "``" result from preprocessing, some are genuine.
        assert!(!replacer.check_if_replacement_point(19));

        // `un_replace` recovers the original text of substrings provided that
        // `check_if_replacement_point` has never been called beyond the starting
        // point of the substring.
        assert_eq!(
            replacer.un_replace(CowStr::Borrowed(&markdown[51..]), 51..markdown.len()),
            CowStr::Borrowed("A costs $1 and B costs $3 and ``it's all $$ expensive''.")
        );
    }
}
