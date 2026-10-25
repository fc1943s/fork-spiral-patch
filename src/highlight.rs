use crate::lexer::{tokenize_line, LexState, Style};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Utf16Span {
    pub start: u32,
    pub end: u32,
    pub style: Style,
}

pub fn is_spiral_path(path: &str) -> bool {
    let extension = path.rsplit_once('.').map(|(_, extension)| extension.to_ascii_lowercase());
    matches!(extension.as_deref(), Some("spi" | "spir"))
}

pub fn highlight(text: &str, ranges_utf16: &[(u32, u32)]) -> Vec<Utf16Span> {
    let chars: Vec<char> = text.chars().collect();
    let mut utf16_at = Vec::with_capacity(chars.len() + 1);
    let mut offset = 0u32;
    for ch in &chars {
        utf16_at.push(offset);
        offset += ch.len_utf16() as u32;
    }
    utf16_at.push(offset);
    let char_index = |utf16: u32| utf16_at.partition_point(|&at| at < utf16).min(chars.len());
    let mut spans = Vec::new();
    for &(start, end) in ranges_utf16 {
        let (start, end) = (char_index(start), char_index(end));
        if start < end {
            highlight_range(&chars, start, end, &utf16_at, &mut spans);
        }
    }
    spans.sort_by_key(|span| span.start);
    spans.dedup_by(|next, previous| next.start < previous.end);
    spans
}

fn highlight_range(chars: &[char], start: usize, end: usize, utf16_at: &[u32], spans: &mut Vec<Utf16Span>) {
    let mut state = LexState::default();
    let mut line_start = start;
    while line_start < end {
        let line_end = chars[line_start..end].iter().position(|&ch| ch == '\n').map_or(end, |at| line_start + at);
        let content_end = if line_end > line_start && chars[line_end - 1] == '\r' { line_end - 1 } else { line_end };
        let (line_spans, next_state) = tokenize_line(state, &chars[line_start..content_end]);
        state = next_state;
        spans.extend(line_spans.into_iter().map(|span| Utf16Span {
            start: utf16_at[line_start + span.start],
            end: utf16_at[line_start + span.end],
            style: span.style,
        }));
        line_start = line_end + 1;
    }
}
