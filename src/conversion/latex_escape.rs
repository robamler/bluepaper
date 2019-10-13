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
            let real_indent = min(MAX_INDENT, self.indent_width * self.indent_level);
            let start = (MAX_NEWLINES - newlines) as usize;
            let end = (MAX_NEWLINES + real_indent) as usize;
            self.inner.write_all(&NEWLINE_AND_INDENT[start..end])?;
        }

        self.current_newlines = 0;
        self.current_newline_limit = MAX_NEWLINES;
        Ok(())
    }

    pub fn write_str(&mut self, s: &str) -> Result<(), std::io::Error> {
        self.prepare_writing()?;
        self.inner.write_all(s.as_bytes())?;
        Ok(())
    }

    pub fn write_escaped(&mut self, s: &str) -> std::result::Result<(), std::io::Error> {
        self.prepare_writing()?;

        unsafe {
            let bytes = s.as_bytes();
            let mut i = 0usize;

            while i != bytes.len() {
                match *bytes.get_unchecked(i) {
                    b'&' => self.inner.write_all(br"\&")?,
                    b'%' => self.inner.write_all(br"\%")?,
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

        Ok(())
    }

    pub fn write_on_single_line(&mut self, s: &str) -> Result<(), std::io::Error> {
        self.add_newlines(1);
        self.write_str(s)?;
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

impl<W: Write> Drop for LatexEscaper<W> {
    fn drop(&mut self) {
        self.prepare_writing().unwrap();
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
        escaper.write_escaped(src).unwrap();

        assert_eq!(
            std::str::from_utf8(escaper.get_ref()).unwrap(),
            "test \\ldots{} ellipsis \\ldots{}, 2 dots .., \
             hyphenation\u{2027}point, ---em dash and--en dash"
        );
    }

    #[test]
    fn escape() {
        let dest = Vec::new();
        let mut escaper = LatexEscaper::new(dest);
        let src = r"   test   escaping $ and # and also & and % and_under{score}~or^caret \  ";
        escaper.write_escaped(src).unwrap();

        assert_eq!(
            std::str::from_utf8(escaper.get_ref()).unwrap(),
            " test escaping \\$ and \\# and also \\& and \\% \
             and\\_under\\{score\\}\\textasciitilde{}or\\textasciicircum{}caret \\textbackslash{} "
        );
    }
}
