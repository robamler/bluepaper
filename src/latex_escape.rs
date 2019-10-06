use std::cmp::{max, min};
use std::io::prelude::*;

const MAX_NEWLINES: u32 = 4;
const MAX_INDENT: u32 = 32;
const NEWLINE_AND_INDENT: &[u8; (MAX_NEWLINES + MAX_INDENT) as usize] =
    b"\n\n\n\n                                ";

pub const UTF8_NB_SPACE_1: u8 = 0xC2;
pub const UTF8_NB_SPACE_2: u8 = 0xA0;
pub const UTF8_PUNCTUATION_1: u8 = 0xE2; // first byte of "en dash", "em dash", and "ellipsis"
pub const UTF8_PUNCTUATION_2: u8 = 0x80; // second byte of "en dash", "em dash", and "ellipsis"
pub const UTF8_EN_DASH_3: u8 = 0x93;
pub const UTF8_EM_DASH_3: u8 = 0x94;
pub const UTF8_ELLIPSIS_3: u8 = 0xA6;

pub struct LatexEscaper<W: Write> {
    inner: W,
    indent_width: u32,
    indent_level: u32,
    current_newlines: u32,
    current_newline_limit: u32,
    math_mode: bool,
}

impl<W: Write> LatexEscaper<W> {
    pub fn new(inner: W) -> Self {
        Self::with_indent_width(2, inner)
    }

    pub fn with_indent_width(indent_width: u32, inner: W) -> Self {
        Self {
            inner,
            indent_width,
            indent_level: 0,
            current_newlines: 0,
            current_newline_limit: 0,
            math_mode: false,
        }
    }

    #[allow(dead_code)]
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    #[allow(dead_code)]
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    #[allow(dead_code)]
    pub fn into_inner(self) -> W {
        self.inner
    }

    pub fn add_newlines(&mut self, num: u32) {
        self.current_newlines = max(self.current_newlines, num);
    }

    pub fn limit_newlines(&mut self, num: u32) {
        self.current_newline_limit = min(self.current_newline_limit, num);
    }

    #[inline(always)]
    fn prepare_writing(&mut self) -> Result<(), std::io::Error> {
        let newlines = min(self.current_newlines, self.current_newline_limit);
        if newlines != 0 {
            if self.math_mode {
                // Invalid document. Try to recover gracefully by leaving math mode.
                self.inner.write_all(b"$")?;
                self.math_mode = false;
            }

            let real_indent = min(MAX_INDENT, self.indent_width * self.indent_level);
            let start = (MAX_NEWLINES - newlines) as usize;
            let end = (MAX_NEWLINES + real_indent) as usize;
            self.inner.write_all(&NEWLINE_AND_INDENT[start..end])?;
        }

        self.current_newlines = 0;
        self.current_newline_limit = MAX_NEWLINES;
        Ok(())
    }

    pub fn write_all(&mut self, buf: &[u8]) -> Result<(), std::io::Error> {
        self.prepare_writing()?;
        self.inner.write_all(buf)?;
        Ok(())
    }

    pub fn write_all_escaped(&mut self, text: &str) -> std::result::Result<(), std::io::Error> {
        self.prepare_writing()?;

        unsafe {
            let bytes = text.as_bytes();
            let mut i = 0usize;

            if self.math_mode && !bytes.is_empty() {
                // Work around an incompatibility between Dropbox's LaTex extension to the
                // Markdown format and the pulldown-cmark crate: sometimes, backslashes inside
                // math mode cause the parser to break up a text element in two, and to omit
                // the backslash.
                self.inner.write_all(b"\\")?;
            }

            while i != bytes.len() {
                if self.math_mode {
                    if *bytes.get_unchecked(i) == b'$' {
                        // Edge case that can only happen in malformed documents. Try to recover
                        // gracefully. Write " $" with a space in front of the '$' so that the
                        // opening and closing '$' don't form an opening block equation.
                        self.inner.write_all(b" $")?;
                        i += 1;
                        if i + 1 != bytes.len() && *bytes.get_unchecked(i + 1) == b'$' {
                            i += 1;
                        }
                    }

                    while i != bytes.len() {
                        match *bytes.get_unchecked(i) {
                            b'\\' if i + 1 < bytes.len() => {
                                // Make sure we don't interpret the following byte as a
                                // control character.
                                self.inner.write_all(bytes.get_unchecked(i..(i + 2)))?;
                                i += 1;
                            }
                            b'$' => {
                                if i + 1 < bytes.len() && *bytes.get_unchecked(i + 1) == b'$' {
                                    i += 2;
                                    self.inner.write_all(b"$")?;
                                    self.math_mode = false;
                                    break;
                                } else {
                                    // Invalid document. Try to stay in math mode. We'll recover
                                    // at the next newline at the latest.
                                    self.inner.write_all(br"\$")?;
                                }
                            }
                            b'#' => self.inner.write_all(br"\#")?,
                            b'\n' => {
                                // Invalid document. Try to recover gracefully by leaving math mode.
                                i += 1;
                                self.inner.write_all(b"$")?;
                                self.add_newlines(1);
                                self.math_mode = false;
                                break;
                            }
                            b => self.inner.write_all(&[b])?,
                        }
                        i += 1;
                    }
                }

                while i != bytes.len() {
                    match *bytes.get_unchecked(i) {
                        b'&' => self.inner.write_all(br"\&")?,
                        b'%' => self.inner.write_all(br"\%")?,
                        b'$' if i + 1 < bytes.len() && *bytes.get_unchecked(i + 1) == b'$' => {
                            self.inner.write_all(b"$")?;
                            self.math_mode = true;
                            i += 2;
                            break;
                        }
                        b'$' => self.inner.write_all(br"\$")?,
                        b'#' => self.inner.write_all(br"\#")?,
                        b'_' => self.inner.write_all(br"\_")?,
                        b'{' => self.inner.write_all(br"\{")?,
                        b'}' => self.inner.write_all(br"\}")?,
                        b'~' => self.inner.write_all(br"\textasciitilde{}")?,
                        b'^' => self.inner.write_all(br"\textasciicircum{}")?,
                        b'\\' => self.inner.write_all(br"\textbackslash{}")?,
                        b'.' if i + 2 < bytes.len()
                            && *bytes.get_unchecked(i + 1) == b'.'
                            && *bytes.get_unchecked(i + 2) == b'.' =>
                        {
                            self.inner.write_all(br"\ldots{}")?;
                            i += 2;
                        }
                        b' ' if i + 1 < bytes.len() && *bytes.get_unchecked(i + 1) == b' ' => {
                            // Ignore space if another space follows.
                        }
                        UTF8_NB_SPACE_1 if *bytes.get_unchecked(i + 1) == UTF8_NB_SPACE_2 => {
                            // Accessing the next byte is save because the input text is
                            // guaranteed to be valid UTF-8.
                            self.inner.write_all(br"~")?;
                            i += 1;
                        }
                        UTF8_PUNCTUATION_1 if *bytes.get_unchecked(i + 1) == UTF8_PUNCTUATION_2 => {
                            // Accessing the next two bytes is save because the input text is
                            // guaranteed to be valid UTF-8.
                            match *bytes.get_unchecked(i + 2) {
                                UTF8_EN_DASH_3 => self.inner.write_all(br"--")?,
                                UTF8_EM_DASH_3 => self.inner.write_all(br"---")?,
                                UTF8_ELLIPSIS_3 => self.inner.write_all(br"\ldots{}")?,
                                _ => self.inner.write_all(bytes.get_unchecked(i..(i + 3)))?,
                            }
                            i += 2;
                        }
                        b => self.inner.write_all(&[b])?,
                    }
                    i += 1;
                }
            }
        }

        Ok(())
    }

    pub fn on_single_line(&mut self, buf: &[u8]) -> Result<(), std::io::Error> {
        self.add_newlines(1);
        self.write_all(buf)?;
        self.add_newlines(1);
        Ok(())
    }

    pub fn increase_indent(&mut self) {
        self.indent_level += 1;
    }

    pub fn decrease_indent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn unicode() {
        let dest = Vec::new();
        let mut escaper = LatexEscaper::new(dest);
        let src = "test … ellipsis ..., 2 dots .., hyphenation‧point, —em dash and–en dash";
        escaper.write_all_escaped(src).unwrap();

        assert_eq!(
            String::from_utf8(escaper.into_inner()).unwrap(),
            "test \\ldots{} ellipsis \\ldots{}, 2 dots .., \
             hyphenation\u{2027}point, ---em dash and--en dash"
        );
    }

    #[test]
    fn escape() {
        let dest = Vec::new();
        let mut escaper = LatexEscaper::new(dest);
        let src = r"   test   escaping $ and # and also & and % and_under{score}~or^caret \  ";
        escaper.write_all_escaped(src).unwrap();

        assert_eq!(
            String::from_utf8(escaper.into_inner()).unwrap(),
            " test escaping \\$ and \\# and also \\& and \\% \
             and\\_under\\{score\\}\\textasciitilde{}or\\textasciicircum{}caret \\textbackslash{} "
        );
    }

    #[test]
    fn math_mode() {
        let dest = Vec::new();
        let mut escaper = LatexEscaper::new(dest);
        assert!(!escaper.math_mode);

        let src1 = r"text $$math  $ and \$ & \{and\\ # 3_\frac{1}{2}^4$$ text  $$math_mode   ";
        let expect1 = r"text $math  \$ and \$ & \{and\\ \# 3_\frac{1}{2}^4$ text $math_mode   ";
        escaper.write_all_escaped(src1).unwrap();

        assert!(escaper.math_mode);
        assert_eq!(
            String::from_utf8(escaper.get_ref().to_vec()).unwrap(),
            expect1
        );

        let src2 = r"  still_math^mode$$ no $ mo^re $$\frac{ag^ain}{math}$$  done  ";
        let expect2 =
            r"\  still_math^mode$ no \$ mo\textasciicircum{}re $\frac{ag^ain}{math}$ done ";
        escaper.write_all_escaped(src2).unwrap();

        assert!(!escaper.math_mode);
        assert_eq!(
            String::from_utf8(escaper.into_inner()).unwrap(),
            format!("{}{}", expect1, expect2)
        );
    }
}
