//! Prompt-Input primitive — a single-line query buffer for keyboard-driven text entry.
//!
//! [`PromptInput`] holds the current query string and exposes four operations:
//! [`push`](PromptInput::push), [`backspace`](PromptInput::backspace),
//! [`clear`](PromptInput::clear), and [`query`](PromptInput::query).
//! No cursor-in-the-middle editing, no history, no persistence — YAGNI.

/// A single-line, append/backspace text buffer used for query entry.
///
/// This is the canonical home for query-edit state shared across features
/// (go-to-file, in-file navigation, …). Construct with [`PromptInput::new`]
/// or [`Default::default`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PromptInput {
    query: String,
}

impl PromptInput {
    /// Returns an empty `PromptInput`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends `c` to the query buffer.
    pub fn push(&mut self, c: char) {
        self.query.push(c);
    }

    /// Removes the last character from the query buffer.
    ///
    /// No-op (no panic) when the buffer is already empty.
    pub fn backspace(&mut self) {
        self.query.pop();
    }

    /// Empties the query buffer.
    pub fn clear(&mut self) {
        self.query.clear();
    }

    /// Returns a shared reference to the current query string.
    pub fn query(&self) -> &str {
        &self.query
    }
}

#[cfg(test)]
mod tests {
    use super::PromptInput;

    #[test]
    fn new_is_empty() {
        let p = PromptInput::new();
        assert_eq!(p.query(), "");
    }

    #[test]
    fn default_is_empty() {
        let p = PromptInput::default();
        assert_eq!(p.query(), "");
    }

    #[test]
    fn push_builds_query_in_order() {
        let mut p = PromptInput::new();
        p.push('a');
        p.push('b');
        assert_eq!(p.query(), "ab");
    }

    #[test]
    fn push_multiple_chars() {
        let mut p = PromptInput::new();
        for c in "hello".chars() {
            p.push(c);
        }
        assert_eq!(p.query(), "hello");
    }

    #[test]
    fn backspace_removes_last_char() {
        let mut p = PromptInput::new();
        p.push('a');
        p.push('b');
        p.backspace();
        assert_eq!(p.query(), "a");
    }

    #[test]
    fn backspace_on_empty_is_noop() {
        let mut p = PromptInput::new();
        p.backspace(); // must not panic
        assert_eq!(p.query(), "");
    }

    #[test]
    fn backspace_empties_single_char_buffer() {
        let mut p = PromptInput::new();
        p.push('x');
        p.backspace();
        assert_eq!(p.query(), "");
    }

    #[test]
    fn clear_empties_buffer() {
        let mut p = PromptInput::new();
        p.push('a');
        p.push('b');
        p.push('c');
        p.clear();
        assert_eq!(p.query(), "");
    }

    #[test]
    fn clear_on_empty_is_noop() {
        let mut p = PromptInput::new();
        p.clear();
        assert_eq!(p.query(), "");
    }

    #[test]
    fn query_reflects_current_buffer_after_operations() {
        let mut p = PromptInput::new();
        p.push('f');
        p.push('o');
        p.push('o');
        assert_eq!(p.query(), "foo");
        p.backspace();
        assert_eq!(p.query(), "fo");
        p.clear();
        assert_eq!(p.query(), "");
        p.push('z');
        assert_eq!(p.query(), "z");
    }

    #[test]
    fn backspace_is_char_safe_with_multibyte() {
        // String::pop removes the last Unicode scalar, not a byte — verify correctness.
        let mut p = PromptInput::new();
        p.push('é'); // U+00E9, 2 UTF-8 bytes
        p.push('a');
        p.backspace();
        assert_eq!(p.query(), "é");
        p.backspace();
        assert_eq!(p.query(), "");
    }
}
