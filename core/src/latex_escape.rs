//! Utility for few-character level replacements in LaTeX output.

use std::io::Write;

const UTF8_NB_SPACE_1: u8 = 0xC2;
const UTF8_NB_SPACE_2: u8 = 0xA0;
const UTF8_PUNCTUATION_1: u8 = 0xE2; // first byte of "en dash", "em dash", and "ellipsis"
const UTF8_PUNCTUATION_2: u8 = 0x80; // second byte of "en dash", "em dash", and "ellipsis"
const UTF8_EN_DASH_3: u8 = 0x93;
const UTF8_EM_DASH_3: u8 = 0x94;
const UTF8_ELLIPSIS_3: u8 = 0xA6;

/// Writes a string of text so that it can be embedded in LaTeX code.
///
/// Writes `s` to `writer`, performing two kinds of replacements:
/// - Escapes common LaTeX control characters such as `\\`, `$`, `^`, `~`, ... so
///   that they appear as text and are not interpreted as LaTeX commands.
/// - Replaces a few common special characters or character sequences such as
///   en-dash (`–`), em-dash (`—`), and ellipsis (`…` and `...`) with their
///   corresponding LaTeX commands.
///
/// The replacement rules in neither group are exhaustive, so this method should not
/// be used to escape untrusted text in a safety critical environment. Suggestions
/// for additional replacement rules are welcome.
pub fn escape_str(s: &str, writer: &mut impl Write) -> std::io::Result<()> {
    unsafe {
        let bytes = s.as_bytes();
        let mut i = 0usize;

        while i != bytes.len() {
            match *bytes.get_unchecked(i) {
                b'&' => writer.write_all(br"\&")?,
                b'%' => writer.write_all(br"\%")?,
                b'$' => writer.write_all(br"\$")?,
                b'#' => writer.write_all(br"\#")?,
                b'_' => writer.write_all(br"\_")?,
                b'{' => writer.write_all(br"\{")?,
                b'}' => writer.write_all(br"\}")?,
                b'~' => writer.write_all(br"\textasciitilde{}")?,
                b'^' => writer.write_all(br"\textasciicircum{}")?,
                b'\\' => writer.write_all(br"\textbackslash{}")?,
                b'.' if i + 2 < bytes.len()
                    && *bytes.get_unchecked(i + 1) == b'.'
                    && *bytes.get_unchecked(i + 2) == b'.' =>
                {
                    writer.write_all(br"\ldots{}")?;
                    i += 2;
                }
                b' ' if i + 1 < bytes.len() && *bytes.get_unchecked(i + 1) == b' ' => {
                    // Ignore space if another space follows.
                }
                UTF8_NB_SPACE_1 if *bytes.get_unchecked(i + 1) == UTF8_NB_SPACE_2 => {
                    // Accessing the next byte is save because the input text is
                    // guaranteed to be valid UTF-8.
                    writer.write_all(br"~")?;
                    i += 1;
                }
                UTF8_PUNCTUATION_1 if *bytes.get_unchecked(i + 1) == UTF8_PUNCTUATION_2 => {
                    // Accessing the next two bytes is save because the input text is
                    // guaranteed to be valid UTF-8.
                    match *bytes.get_unchecked(i + 2) {
                        UTF8_EN_DASH_3 => writer.write_all(br"--")?,
                        UTF8_EM_DASH_3 => writer.write_all(br"---")?,
                        UTF8_ELLIPSIS_3 => writer.write_all(br"\ldots{}")?,
                        _ => writer.write_all(bytes.get_unchecked(i..(i + 3)))?,
                    }
                    i += 2;
                }
                b => writer.write_all(&[b])?,
            }
            i += 1;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn escape_to_string(src: &str) -> String {
        let mut result = Vec::new();
        escape_str(src, &mut result).unwrap();
        String::from_utf8(result).unwrap()
    }

    #[test]
    fn unicode() {
        let src = "test … ellipsis ..., 2 dots .., hyphenation‧point, —em dash and–en dash";

        assert_eq!(
            &escape_to_string(src),
            "test \\ldots{} ellipsis \\ldots{}, 2 dots .., \
             hyphenation\u{2027}point, ---em dash and--en dash"
        );
    }

    #[test]
    fn escape() {
        let src = r"   test   escaping $ and # and also & and % and_under{score}~or^caret \  ";

        assert_eq!(
            &escape_to_string(src),
            " test escaping \\$ and \\# and also \\& and \\% \
             and\\_under\\{score\\}\\textasciitilde{}or\\textasciicircum{}caret \\textbackslash{} "
        );
    }
}
