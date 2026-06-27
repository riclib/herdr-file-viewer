//! Match Highlighter — overlay highlight styling onto content-pane spans (T-7 / AC-9, AC-11).
//!
//! `apply` re-segments each `Line`'s spans at match boundaries and patches highlight
//! styles onto the sub-spans that fall within a match's `[start, end)` byte range.
//! The current match (`matches[current]`) gets a visually distinct style.
//!
//! Invariants upheld:
//! - **Pure & read-only**: allocates a new `Vec<Line>`; never mutates the input.
//! - **Zero new Cargo deps**: ratatui + std only.
//! - **Skip, never panic**: stale or out-of-range matches are dropped silently.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::search::Match;

// ── public style constants ────────────────────────────────────────────────────

/// Style applied to every non-current highlighted match (black text on yellow).
pub const HIGHLIGHT: Style = Style::new().fg(Color::Black).bg(Color::Yellow);

/// Style applied to the **current** match — visually distinct from `HIGHLIGHT`
/// (black text on cyan).
pub const CURRENT_HIGHLIGHT: Style = Style::new().fg(Color::Black).bg(Color::Cyan);

// ── public API ────────────────────────────────────────────────────────────────

/// Re-segment `lines` at match boundaries and overlay highlight styles.
///
/// For each line in `lines`:
/// - If no match targets that line, the line is cloned unchanged.
/// - Otherwise the line's spans are split at every match `[start, end)` byte
///   boundary and the resulting sub-spans inside a match get the highlight style
///   patched onto the original span style. Sub-spans inside `matches[current]`
///   get `CURRENT_HIGHLIGHT`; all other matched sub-spans get `HIGHLIGHT`.
///
/// Out-of-range matches (line index beyond `lines.len()`, byte range past the
/// line's plain-text length, or a range not on a UTF-8 char boundary) are
/// silently skipped.
pub fn apply(lines: &[Line<'static>], matches: &[Match], current: usize) -> Vec<Line<'static>> {
    lines
        .iter()
        .enumerate()
        .map(|(line_idx, line)| {
            // Collect only the matches that target this line.
            let line_matches: Vec<&Match> = matches.iter().filter(|m| m.line == line_idx).collect();

            if line_matches.is_empty() {
                return line.clone();
            }

            // Compute the total plain-text byte length for this line once so we
            // can validate match bounds cheaply.
            let line_text_len: usize = line.spans.iter().map(|s| s.content.len()).sum();

            // Validate each match; drop silently if it is out of range or not on
            // a char boundary (we reconstruct the plain text on demand below).
            let validated: Vec<(usize, usize, bool)> = line_matches
                .iter()
                .filter_map(|m| {
                    let match_idx = matches.iter().position(|x| std::ptr::eq(x, *m))?;
                    validate_match(line, line_text_len, m).map(|(s, e)| {
                        let is_current = match_idx == current;
                        (s, e, is_current)
                    })
                })
                .collect();

            if validated.is_empty() {
                return line.clone();
            }

            // Re-segment the spans.
            let new_spans = resegment(&line.spans, &validated);
            Line {
                spans: new_spans,
                style: line.style,
                alignment: line.alignment,
            }
        })
        .collect()
}

// ── internals ─────────────────────────────────────────────────────────────────

/// Validate a match against the line's plain-text length and char boundaries.
/// Returns `Some((start, end))` if the match is usable, `None` to skip.
fn validate_match(line: &Line<'static>, line_text_len: usize, m: &Match) -> Option<(usize, usize)> {
    if m.start > m.end || m.end > line_text_len {
        return None;
    }
    if m.start == m.end {
        // Zero-length match — nothing to highlight, skip.
        return None;
    }
    // Verify that start and end are on UTF-8 char boundaries by reconstructing
    // the relevant portion of the plain text and checking boundary validity.
    // We walk the spans to collect just enough text to verify.
    let mut plain = String::with_capacity(line_text_len);
    for span in &line.spans {
        plain.push_str(&span.content);
    }
    if !plain.is_char_boundary(m.start) || !plain.is_char_boundary(m.end) {
        return None;
    }
    Some((m.start, m.end))
}

/// Split `spans` at the boundaries implied by `intervals` (sorted `(start, end, is_current)`)
/// and return the resulting sub-spans with highlight styles patched in.
///
/// `intervals` must all be valid (validated by `validate_match`).
fn resegment(spans: &[Span<'static>], intervals: &[(usize, usize, bool)]) -> Vec<Span<'static>> {
    // Collect all boundary points from the intervals and sort them.
    let mut boundaries: Vec<usize> = Vec::with_capacity(intervals.len() * 2);
    for (s, e, _) in intervals {
        boundaries.push(*s);
        boundaries.push(*e);
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    // Walk spans, splitting at each boundary.
    let mut result: Vec<Span<'static>> = Vec::new();
    let mut byte_cursor: usize = 0; // global byte offset in the plain text

    for span in spans {
        let span_text = span.content.as_ref();
        let span_len = span_text.len();
        let span_lo = byte_cursor;
        let span_hi = byte_cursor + span_len;

        // Find boundaries that fall strictly inside this span.
        let cuts: Vec<usize> = boundaries
            .iter()
            .copied()
            .filter(|&b| b > span_lo && b < span_hi)
            .collect();

        if cuts.is_empty() {
            // No split needed — emit the whole span with the appropriate style.
            let style = style_for_offset(span_lo, span_hi, span.style, intervals);
            result.push(Span {
                content: span.content.clone(),
                style,
            });
        } else {
            // Split the span at each cut point.
            let mut pos = span_lo;
            for &cut in &cuts {
                if cut > pos {
                    let sub = &span_text[(pos - span_lo)..(cut - span_lo)];
                    let style = style_for_offset(pos, cut, span.style, intervals);
                    result.push(Span {
                        content: std::borrow::Cow::Owned(sub.to_owned()),
                        style,
                    });
                }
                pos = cut;
            }
            // Remaining part after last cut.
            if pos < span_hi {
                let sub = &span_text[(pos - span_lo)..(span_hi - span_lo)];
                let style = style_for_offset(pos, span_hi, span.style, intervals);
                result.push(Span {
                    content: std::borrow::Cow::Owned(sub.to_owned()),
                    style,
                });
            }
        }

        byte_cursor = span_hi;
    }

    result
}

/// Determine the effective style for the byte range `[lo, hi)` of plain text.
///
/// If the range is fully covered by an interval, patch the highlight onto the
/// original span style. Otherwise keep the original style.
///
/// The range `[lo, hi)` is always a sub-segment of a single original span, so
/// it cannot partially overlap an interval — it is either fully inside or fully
/// outside every interval (boundaries were inserted at interval edges).
fn style_for_offset(
    lo: usize,
    hi: usize,
    original_style: Style,
    intervals: &[(usize, usize, bool)],
) -> Style {
    for &(start, end, is_current) in intervals {
        if lo >= start && hi <= end {
            let highlight = if is_current {
                CURRENT_HIGHLIGHT
            } else {
                HIGHLIGHT
            };
            return original_style.patch(highlight);
        }
    }
    original_style
}
