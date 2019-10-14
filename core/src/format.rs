//! Utility for writing cleanly formatted text files.

use std::cmp::{max, min};
use std::io::prelude::*;

const MAX_NEWLINES: u32 = 4;
const MAX_INDENT: u32 = 32;
const NEWLINE_AND_INDENT: &[u8; (MAX_NEWLINES + MAX_INDENT) as usize] =
    b"\n\n\n\n                                ";

/// A wrapper around a writer that performs indentation and fuses newlines.
///
/// Wraps a struct that implements `std::io::Write`. Implements `Write` itself and
/// adds methods to increase and decrease the indentation level, and to lazily
/// inject newlines.
///
/// The lazy injection of newlines is useful because it allows to fuse newlines from
/// consecutive blocks of text. For example, consider writing a paragraph of text
/// followed by a section header. We may want to have at least one blank line (i.e.,
/// two newlines) after a paragraph of text, and we may want to always have two
/// blank lines (i.e., three newlines) above every section header. Inserting these
/// newlines eagerly would result in an excessive 2 + 3 = 5 newlines (i.e., four blank
/// lines) above the section header. Instead, a `WhitespaceFormatter` fuses the
/// consecutive space to just two blank lines:
/// ```
/// use bluepaper_core::format::WhitespaceFormatter;
/// use std::io::prelude::*;
///
/// let buf = Vec::<u8>::new();
/// let mut formatter = WhitespaceFormatter::new(buf);
///
/// // Write the paragraph of text without any newlines.
/// formatter.write_all(b"Some paragraph of text.").unwrap();
/// // Ensure that at least two newlines must follow a paragraph.
/// formatter.add_newlines(2);
///
/// // ... some more code that may or may not write more paragraphs ...
///
/// // Ensure that at least three newlines must precede a header.
/// formatter.add_newlines(3);
/// // Write the section header without any newlines.
/// formatter.write_all(b"## Section Header");
///
/// // Verify that exactly three newlines were written:
/// let s = std::str::from_utf8(formatter.get_ref()).unwrap();
/// assert_eq!(s, "Some paragraph of text.\n\n\n## Section Header");
/// ```
///
/// See [`limit_newlines`](#method.limit_newlines) for a more advanced example.
pub struct WhitespaceFormatter<W: Write> {
    inner: W,
    indent_width: u32,
    indent_level: u32,
    current_newlines: u32,
    current_newline_limit: u32,
}

impl<W: Write> WhitespaceFormatter<W> {
    /// Creates a new `WhitespaceFormatter`.
    ///
    /// Returns a formatter that writes to `inner` with a default indentation width of `2`.
    pub fn new(inner: W) -> Self {
        Self::with_indent_width(2, inner)
    }

    /// Creates a new `WhitespaceFormatter` with a custom indentation width.
    ///
    /// The `indentation_width` controls how many spaces of indentation are added per
    /// call to [`increase_indent`](#method.increase_indent).
    pub fn with_indent_width(indent_width: u32, inner: W) -> Self {
        Self {
            inner,
            indent_width,
            indent_level: 0,
            current_newlines: 0,
            current_newline_limit: 0,
        }
    }

    /// Consumes the formatter and returns the wrapped writer.
    ///
    /// Inserts any pending newlines and indentations before returning the the wrapped
    /// writer. If that's not what you want, call
    /// [`limit_newlines(0)`](#method.limit_newlines) first.  Returns `Err(..)` if
    /// inserting the pending newlines resulted in an error.
    pub fn into_inner(mut self) -> std::io::Result<W> {
        self.prepare_writing()?;
        Ok(self.inner)
    }

    /// Returns a mutable reference to the wrapped writer.
    ///
    /// Inserts any pending newlines and indentations first. This is so that the
    /// returned mutable reference can immediately be used for writing. Returns
    /// `Err(..)` if inserting the pending newlines resulted in an error.
    ///
    /// This method is useful if you want to perform a lot of small writes without any
    /// newlines. This would be wasteful to do on the `WhitespaceFormatter`, which has
    /// to check on each write operation if any pending newlines have to be written
    /// first. Writing directly to the wrapped inner writer elides these checks.
    pub fn get_mut(&mut self) -> std::io::Result<&mut W> {
        self.prepare_writing()?;
        Ok(&mut self.inner)
    }

    /// Returns a shared reference to the wrapped writer.
    ///
    /// In contrast to [`into_inner`](#method.into_inner) and
    /// [`get_mut`](#method.get_mut), this method does not insert any pending
    /// newlines. This is because the receiver of the shared reference cannot write to
    /// the wrapped writer anyway, so it's better to hold off with writing newlines in
    /// case they have to be be fused with future calls to
    /// [`add_newlines`](#method.add_newlines).
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Lazily add the requested number of newlines to the output stream.
    ///
    /// Records an intend to write at least `num` newlines at the current position, but
    /// does not actually write them yet. This is so that blank lines from consecutive
    /// blocks of text can be fused, see [struct level
    /// documentation](struct.WhitespaceFormatter.html). The newlines are written as
    /// soon as the user writes any actual text via [`write`](#method.write),
    /// [`write_all`](#method.write_all), or [`get_mut`](#method.get_mut).
    ///
    /// More precisely, calling `add_newlines(num)` ensures that *at least* `num`
    /// newlines will eventually be inserted at the current position. If `add_newlines`
    /// is called several times with arguments `num1, num2, ...` without writing any
    /// text in-between these calls, then the number of newlines that will be written at
    /// the current position is the max of `num1, num2, ...`, as long as they don't
    /// exceed the limit set by [`limit_newlines`], or it's default value of 4.
    ///
    /// `limit_newlines`: #method.limit_newlines
    pub fn add_newlines(&mut self, num: u32) {
        self.current_newlines = max(self.current_newlines, num);
    }

    /// Limit the number of newlines inserted at the current position.
    ///
    /// Ensures that no more than `num` newlines are inserted at the current position,
    /// even if there's a call to [`add_newlines`](#method.add_newlines) with an
    /// argument larger than `num`. A global limit of at most four consecutive newlines
    /// is always enforced implicitly.
    ///
    /// This method is useful, e.g., for consecutive headers. You may want to insert two
    /// blank blank lines between a paragraph and the header of the next section.
    /// However, you may want only a single blank line between a section header and an
    /// immediately following subsection header (see result of the example below). This
    /// can be achieved as follows:
    ///
    /// ```
    /// use bluepaper_core::format::WhitespaceFormatter;
    /// use std::io::prelude::*;
    ///
    /// fn write_paragraph(text: &str, formatter: &mut WhitespaceFormatter<impl Write>) {
    ///     formatter.write_all(text.as_bytes()).unwrap();
    ///     formatter.add_newlines(2); // One blank line (two newlines) below paragraphs.
    /// }
    ///
    /// /// Write a section header, separated with
    /// fn write_header(text: &str, formatter: &mut WhitespaceFormatter<impl Write>) {
    ///     formatter.add_newlines(3); // Two blank lines (three newlines) above headers ...
    ///     formatter.write_all(text.as_bytes()).unwrap();
    ///     formatter.add_newlines(2);
    ///     formatter.limit_newlines(2); // ... but no more than one blank line below.
    /// }
    ///
    /// let buf = Vec::<u8>::new();
    /// let mut formatter = WhitespaceFormatter::new(buf);
    ///
    /// write_paragraph("Some paragraph of text.", &mut formatter);
    /// write_paragraph("Another paragraph of text.", &mut formatter);
    /// write_header("# Section Header", &mut formatter);
    /// write_header("# Subsection Header", &mut formatter);
    ///
    /// let s = std::str::from_utf8(formatter.get_ref()).unwrap();
    /// assert_eq!(s, concat!(
    ///     "Some paragraph of text.\n",
    ///     "\n",
    ///     "Another paragraph of text.\n",
    ///     "\n",                                // <-- Two blank lines between a
    ///     "\n",                                // <-- paragraph and a section header.
    ///     "# Section Header\n",
    ///     "\n",                                // <-- But only a single blank
    ///     "# Subsection Header"                //     line between headers.
    /// ));
    /// ```
    pub fn limit_newlines(&mut self, num: u32) {
        self.current_newline_limit = min(self.current_newline_limit, num);
    }

    /// Convenience method for writing a line of text.
    ///
    /// Inserts at least one newline above and below the string `s`.
    pub fn write_on_single_line(&mut self, s: &str) -> std::io::Result<()> {
        self.add_newlines(1);
        self.write_all(s.as_bytes())?;
        self.add_newlines(1);
        Ok(())
    }

    /// Increase the indentation level by one.
    ///
    /// Every text that is preceded by newlines is indented by a certain number of
    /// spaces. This method increases the indentation level. The width per indentation
    /// level can be specified with [`with_indent_width`](#method.with_indent_width).
    ///
    /// Indentation is only inserted after lazily inserted newlines (see
    /// [`insert_newlines`](#method.insert_newlines)). Explicit newline characters
    /// ("\n") in the argument to [`write`](#method.write) or
    /// [`write_all`](#method.write_all) do not lead to indentation. Also, blank lines
    /// are not indented.
    pub fn increase_indent(&mut self) {
        self.indent_level += 1;
    }

    /// Decrease the indentation level by one if it is not zero.
    ///
    /// See [`increase_indent`](#method.increase_indent).
    pub fn decrease_indent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    fn prepare_writing(&mut self) -> std::io::Result<()> {
        let newlines = min(self.current_newlines, self.current_newline_limit);

        if newlines != 0 {
            let indentation = min(MAX_INDENT, self.indent_width * self.indent_level);
            let start = (MAX_NEWLINES - newlines) as usize;
            let end = (MAX_NEWLINES + indentation) as usize;
            self.inner.write_all(&NEWLINE_AND_INDENT[start..end])?;
        }

        self.current_newlines = 0;
        self.current_newline_limit = MAX_NEWLINES;
        Ok(())
    }
}

impl<W: Write> Write for WhitespaceFormatter<W> {
    /// Writes any pending newlines and indentation, followed by the beginning of `buf`.
    ///
    /// This is most likely *not* the method you want to call in an application. Most of
    /// the time, you'll want to call [`write_all`](#method.write_all).
    ///
    /// Returns the number of bytes written from `buf`, not including any lazily written
    /// newlines or indentation. Resets the number of pending newlines to zero, and the
    /// limit on newlines (see [`limit_newlines`](#method.limit_newlines)) to the global
    /// limit.
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.prepare_writing()?;
        Ok(self.inner.write(buf)?)
    }

    /// Writes the full contents of `buf`, possibly preceded by newlines and
    /// indentation. Then resets the number of pending newlines to zero, and the limit
    /// on newlines (see [`limit_newlines`](#method.limit_newlines)) to the global
    /// limit.
    ///
    /// Avoid this method if you want to perform a lot of small writes without any calls
    /// to [`add_newlines`] in between (such as, when escaping a string of text
    /// character by character). This method has to check on every call whether
    /// [`add newlines`] has been called since the last write, which incurs some
    /// overhead. If you make a lot of small writes without calling [`add_newlines`] in
    /// between, consider calling [`get_mut`] instead and writing directly to the
    /// wrapped writer. Calling [`get_mut`] also inserts any pending newlines and
    /// indentation, but it has to be called only once at the beginning of the sequence
    /// of writes.
    ///
    /// [`add_newlines`]: #method.add_newlines
    /// [`get_mut`]: #method.get_mut
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.prepare_writing()?;
        self.inner.write_all(buf)?;
        Ok(())
    }

    /// Writes a formatted string, possibly preceded by newlines and indentation.
    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        self.prepare_writing()?;
        self.inner.write_fmt(fmt)
    }

    /// Flushes the wrapped writer. Does not write any pending newlines or indentation.
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
