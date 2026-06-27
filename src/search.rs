//! Search Matcher — pure substring search over a slice of lines.
//!
//! Implements smartcase: a query that contains no uppercase letters is matched
//! case-insensitively (ASCII case folding); a query containing any uppercase letter
//! is matched case-sensitively. Regex metacharacters are not interpreted — the query
//! is always treated as a literal substring.
//!
//! # Offset semantics
//! `Match::start` and `Match::end` are **byte offsets** into the original (un-folded) line
//! string such that `&lines[m.line][m.start..m.end]` is a valid UTF-8 slice equal to the
//! matched text. ASCII case folding is used for the case-insensitive path so byte offsets
//! stay aligned with the original line; non-ASCII letters compare case-sensitively, which is
//! acceptable for a code/diff viewer whose content is overwhelmingly ASCII.

/// A single non-overlapping substring match.
///
/// `line` is the 0-based index into the `lines` slice passed to [`find_matches`].
/// `start` and `end` are byte offsets (half-open `[start, end)`) into that line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

/// Find all non-overlapping occurrences of `query` within `lines`.
///
/// Returns matches in document order (line index ascending, then column ascending).
/// An empty query or a query with no occurrences returns an empty `Vec`.
///
/// Smartcase rule: if `query` contains no uppercase ASCII letters the search is
/// case-insensitive (ASCII fold); otherwise it is case-sensitive.
pub fn find_matches(query: &str, lines: &[String]) -> Vec<Match> {
    if query.is_empty() {
        return Vec::new();
    }

    // Determine case-sensitivity once for the whole call.
    let case_sensitive = query.chars().any(|c| c.is_ascii_uppercase());

    if case_sensitive {
        find_matches_case_sensitive(query, lines)
    } else {
        find_matches_case_insensitive(query, lines)
    }
}

fn find_matches_case_sensitive(query: &str, lines: &[String]) -> Vec<Match> {
    let mut matches = Vec::new();
    for (line_idx, line) in lines.iter().enumerate() {
        // `str::match_indices` yields byte offsets directly.
        let mut search_start = 0usize;
        while search_start <= line.len().saturating_sub(query.len()) {
            match line[search_start..].find(query) {
                Some(offset) => {
                    let start = search_start + offset;
                    let end = start + query.len();
                    matches.push(Match {
                        line: line_idx,
                        start,
                        end,
                    });
                    search_start = end; // non-overlapping: resume after this match
                }
                None => break,
            }
        }
    }
    matches
}

fn find_matches_case_insensitive(query: &str, lines: &[String]) -> Vec<Match> {
    // Fold the query once.
    let query_folded = query.to_ascii_lowercase();
    let mut matches = Vec::new();

    for (line_idx, line) in lines.iter().enumerate() {
        // Fold the line for matching; offsets align because ASCII fold is byte-length-preserving.
        let line_folded = line.to_ascii_lowercase();
        let mut search_start = 0usize;
        while search_start <= line_folded.len().saturating_sub(query_folded.len()) {
            match line_folded[search_start..].find(query_folded.as_str()) {
                Some(offset) => {
                    let start = search_start + offset;
                    let end = start + query_folded.len();
                    matches.push(Match {
                        line: line_idx,
                        start,
                        end,
                    });
                    search_start = end;
                }
                None => break,
            }
        }
    }
    matches
}
