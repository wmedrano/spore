/// Describes the location of a substring within a string.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Span {
    /// The start of the substring.
    pub start: u32,
    /// The end of the substring.
    pub end: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct SpanWithSource<T> {
    /// The location within [Self::src] to refer to.
    pub span: Span,
    /// The entire source string.
    pub src: T,
}

impl Span {
    /// Create a new span.
    pub fn new(start: u32, end: u32) -> Span {
        Span { start, end }
    }

    /// Link the underlying span with its source.
    pub fn with_src<T>(self, src: T) -> SpanWithSource<T> {
        SpanWithSource { span: self, src }
    }

    /// Expand the current span to `end`. If `end` is less than the current end, then `self` is
    /// returned
    pub fn extend_end(self, end: u32) -> Span {
        Span {
            start: self.start,
            end: self.end.max(end),
        }
    }

    /// Get the next window. The next window is defined as starting at the end of `self` with length
    /// `end`.
    pub fn next_window(self, len: u32) -> Span {
        Span {
            start: self.end,
            end: self.end + len,
        }
    }

    /// Returns a span that overlaps with both `self` and `other` or `None` if there is no overlap.
    pub fn overlap(self, other: Span) -> Option<Span> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        (start <= end).then_some(Span { start, end })
    }
}

impl<'a> SpanWithSource<&'a str> {
    /// Get the `str` for the string pointed to by [Self::span] within [Self::src].
    pub fn as_str(&self) -> &'a str {
        &self.src[self.span.start as usize..self.span.end as usize]
    }
}

impl<T: AsRef<str>> SpanWithSource<T> {
    pub fn contextual_formatter(&self) -> impl '_ + std::fmt::Display {
        SpanWithSourceContextualFormatter(self)
    }
}

impl<T> std::fmt::Display for SpanWithSource<T>
where
    T: AsRef<str>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let src = self.src.as_ref();
        let s = &src[self.span.start as usize..self.span.end as usize];
        write!(f, "{s}")
    }
}

struct SpanWithSourceContextualFormatter<'a, T>(&'a SpanWithSource<T>);

impl<'a, T> std::fmt::Display for SpanWithSourceContextualFormatter<'a, T>
where
    T: AsRef<str>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let span = self.0.span;
        let src = self.0.src.as_ref();

        let mut current_span = Span::new(0, 0);
        writeln!(f, "Source:")?;
        for (idx, line) in src.split('\n').enumerate() {
            current_span = current_span.next_window(1 + line.len() as u32);
            if current_span.overlap(span).is_some() {
                let line_number = idx + 1;
                let line_src = current_span.with_src(src);
                writeln!(f, "{line_number:3}: {line_src}")?;
            }
        }
        Ok(())
    }
}
