#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Style {
    Comment = 0,
    String = 1,
    Keyword = 2,
    Type = 3,
    Command = 4,
    Attribute = 5,
    Variable = 6,
    Value = 7,
    Number = 8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LexState {
    pub triple_quotes: usize,
    pub comment_depth: usize,
    pub macro_close: Option<char>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub style: Style,
}

const WORD_KEYWORDS: &[&str] = &[
    "in", "and", "fun", "match", "typecase", "function", "with", "without", "as", "when", "inl", "forall", "let",
    "inm", "inb", "rec", "if", "then", "elif", "else", "join", "join_backend", "type", "nominal", "real", "union",
    "open", "_", "prototype", "instance", "exists",
];

const SYMBOL_KEYWORDS: &[&str] = &[
    ":=", ":~", "^", ":&", ";;", "\\", "\\|", "\\*", "\\?", "?", "?>", "|?", "|:", "?=", "?:", "?|", "-|", "&=",
    "?&", ":==", ":!", ":|", ":?", ":+", ":<", "~>", "~~", "~~>",
];

const SYMBOL_BOOLEANS: &[&str] = &["!+", "!-"];

const NUMBER_SUFFIXES: &[&str] = &["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64"];

fn is_var_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

fn is_var_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '\''
}

fn is_open_bracket(ch: char) -> bool {
    matches!(ch, '(' | '[' | '{')
}

fn is_close_bracket(ch: char) -> bool {
    matches!(ch, ')' | ']' | '}')
}

fn is_operator_char(ch: char) -> bool {
    ('!'..='~').contains(&ch) && !is_var_char(ch) && ch != '"' && !is_open_bracket(ch) && !is_close_bracket(ch)
}

fn is_prefix_separator(ch: Option<char>) -> bool {
    ch.is_none_or(|ch| ch == ' ' || ch == '\t' || is_open_bracket(ch))
}

fn is_postfix_separator(ch: Option<char>) -> bool {
    ch.is_none_or(|ch| ch == ' ' || ch == '\t' || is_close_bracket(ch))
}

fn symbol_style(name: &str) -> Option<Style> {
    if SYMBOL_KEYWORDS.contains(&name) {
        Some(Style::Keyword)
    } else if SYMBOL_BOOLEANS.contains(&name) {
        Some(Style::Number)
    } else {
        None
    }
}

fn comment_scan(chars: &[char], from: usize, depth: usize) -> (usize, usize) {
    let mut at = from;
    let mut depth = depth;
    while at < chars.len() && depth > 0 {
        let next = chars.get(at + 1).copied();
        if chars[at] == '(' && next == Some('*') {
            if chars.get(at + 2) == Some(&')') {
                at += 3;
            } else {
                depth += 1;
                at += 2;
            }
        } else if chars[at] == '*' && next == Some(')') {
            depth -= 1;
            at += 2;
        } else {
            at += 1;
        }
    }
    (at, depth)
}

struct LineLexer<'a> {
    chars: &'a [char],
    at: usize,
    spans: Vec<Span>,
    state: LexState,
}

pub fn tokenize_line(state: LexState, chars: &[char]) -> (Vec<Span>, LexState) {
    let mut lexer = LineLexer { chars, at: 0, spans: Vec::new(), state };
    lexer.resume();
    while lexer.at < chars.len() && lexer.state.comment_depth == 0 && lexer.state.triple_quotes == 0 {
        lexer.token();
    }
    (lexer.spans, lexer.state)
}

impl LineLexer<'_> {
    fn peek(&self, offset: usize) -> Option<char> {
        self.chars.get(self.at + offset).copied()
    }

    fn before(&self, index: usize) -> Option<char> {
        index.checked_sub(1).map(|previous| self.chars[previous])
    }

    fn emit(&mut self, start: usize, end: usize, style: Style) {
        if start >= end {
            return;
        }
        match self.spans.last_mut() {
            Some(last) if last.end == start && last.style == style => last.end = end,
            _ => self.spans.push(Span { start, end, style }),
        }
    }

    fn advance(&mut self, length: usize, style: Style) {
        self.emit(self.at, self.at + length, style);
        self.at += length;
    }

    fn take_while(&self, from: usize, predicate: impl Fn(char) -> bool) -> usize {
        let mut end = from;
        while end < self.chars.len() && predicate(self.chars[end]) {
            end += 1;
        }
        end
    }

    fn resume(&mut self) {
        if self.state.comment_depth > 0 {
            let (end, depth) = comment_scan(self.chars, 0, self.state.comment_depth);
            self.emit(0, end, Style::Comment);
            self.at = end;
            self.state.comment_depth = depth;
        } else if self.state.triple_quotes > 0 {
            self.triple_body(0);
        } else if let Some(close) = self.state.macro_close.take() {
            self.at = self.take_while(0, |ch| ch == ' ');
            self.macro_body(close);
        }
    }

    fn token(&mut self) {
        let ch = self.chars[self.at];
        let next = self.peek(1);
        match ch {
            ' ' | '\t' => self.at += 1,
            '$' if matches!(next, Some('"' | '\'')) => {
                self.advance(2, Style::String);
                self.macro_body(next.unwrap_or('"'));
            }
            '"' if next == Some('"') && self.peek(2) == Some('"') => self.triple_open(),
            '"' => self.quoted(),
            '0' if matches!(next, Some('x' | 'X')) && self.peek(2).is_some_and(|ch| ch.is_ascii_hexdigit()) => {
                let end = self.take_while(self.at + 2, |ch| ch.is_ascii_hexdigit());
                self.advance(end - self.at, Style::Number);
            }
            _ if ch.is_ascii_digit() => self.number(),
            '-' if next.is_some_and(|ch| ch.is_ascii_digit()) && is_prefix_separator(self.before(self.at)) => {
                self.number()
            }
            _ if is_var_start(ch) => self.word(),
            '.' if next == Some('(') && self.operator_symbol_end().is_some() => {
                let end = self.operator_symbol_end().unwrap_or(self.at + 1);
                self.advance(end - self.at, Style::Value);
            }
            '.' if next.is_some_and(is_var_start) => {
                let end = self.take_while(self.at + 1, is_var_char);
                self.advance(end - self.at, Style::Value);
            }
            '@' if next == Some('"') => self.raw(),
            '\'' if self.char_literal_length().is_some() => {
                let length = self.char_literal_length().unwrap_or(1);
                self.advance(length, Style::String);
            }
            '/' if next == Some('/') => self.line_comment(),
            '(' if next == Some('*') && matches!(self.peek(2), None | Some(' ')) => self.block_comment(),
            _ if is_open_bracket(ch) || is_close_bracket(ch) => self.at += 1,
            _ if is_operator_char(ch) => self.operator(),
            _ => self.at += 1,
        }
    }

    fn word(&mut self) {
        let start = self.at;
        let end = self.take_while(start, is_var_char);
        let word: String = self.chars[start..end].iter().collect();
        let style = if word == "true" || word == "false" {
            Some(Style::Number)
        } else if WORD_KEYWORDS.contains(&word.as_str()) {
            Some(Style::Keyword)
        } else if word == "test" && start == 0 && self.chars.get(end) == Some(&' ') {
            Some(Style::Keyword)
        } else if word.starts_with(char::is_uppercase) {
            Some(Style::Type)
        } else {
            None
        };
        match style {
            Some(style) => self.advance(end - start, style),
            None => self.at = end,
        }
    }

    fn number(&mut self) {
        let start = self.at;
        let digits = |ch: char| ch.is_ascii_digit() || ch == '_';
        let mut end = self.take_while(start + 1, digits);
        if self.chars.get(end) == Some(&'.') && self.chars.get(end + 1).is_some_and(|ch| ch.is_ascii_digit()) {
            end = self.take_while(end + 1, digits);
        }
        if matches!(self.chars.get(end), Some('e' | 'E')) {
            let exponent = if matches!(self.chars.get(end + 1), Some('+' | '-')) { end + 2 } else { end + 1 };
            if self.chars.get(exponent).is_some_and(|ch| ch.is_ascii_digit()) {
                end = self.take_while(exponent, digits);
            }
        }
        self.advance(end - start, Style::Number);
        let rest: String = self.chars[self.at..].iter().take(3).collect();
        let suffix = NUMBER_SUFFIXES
            .iter()
            .filter(|suffix| rest.starts_with(**suffix))
            .max_by_key(|suffix| suffix.len())
            .filter(|suffix| !self.peek(suffix.len()).is_some_and(is_var_char));
        if let Some(suffix) = suffix {
            self.advance(suffix.len(), Style::Keyword);
        }
    }

    fn operator_symbol_end(&self) -> Option<usize> {
        let body = self.at + 2;
        let end = self.take_while(body, is_operator_char);
        (end > body && self.chars.get(end) == Some(&')')).then_some(end + 1)
    }

    fn char_literal_length(&self) -> Option<usize> {
        let body = self.peek(1)?;
        let length = if body == '\\' {
            self.peek(2)?;
            4
        } else {
            3
        };
        (body != '\'' && self.peek(length - 1) == Some('\'')).then_some(length)
    }

    fn raw(&mut self) {
        let start = self.at;
        let close = self.take_while(start + 2, |ch| ch != '"');
        let end = (close + 1).min(self.chars.len());
        self.advance(end - start, Style::String);
    }

    fn quoted(&mut self) {
        self.advance(1, Style::String);
        while let Some(ch) = self.peek(0) {
            match ch {
                '"' => {
                    self.advance(1, Style::String);
                    return;
                }
                '\\' if self.peek(1).is_some() => self.advance(2, Style::Attribute),
                _ => self.advance(1, Style::String),
            }
        }
    }

    fn triple_open(&mut self) {
        let run = self.take_while(self.at, |ch| ch == '"') - self.at;
        let quotes = if run == 6 { 3 } else { run };
        self.advance(quotes, Style::String);
        self.state.triple_quotes = quotes;
        self.triple_body(self.at);
    }

    fn triple_body(&mut self, from: usize) {
        let quotes = self.state.triple_quotes;
        let mut cursor = from;
        while cursor < self.chars.len() {
            if self.chars[cursor] == '"' {
                let run_end = self.take_while(cursor, |ch| ch == '"');
                if run_end - cursor >= quotes {
                    self.emit(from, run_end, Style::String);
                    self.at = run_end;
                    self.state.triple_quotes = 0;
                    return;
                }
                cursor = run_end;
            } else {
                cursor += 1;
            }
        }
        self.emit(from, self.chars.len(), Style::String);
        self.at = self.chars.len();
    }

    fn line_comment(&mut self) {
        let start = self.at;
        let mut cursor = start + 2;
        loop {
            match self.chars.get(cursor) {
                Some('/') => cursor += 1,
                Some('!' | '-' | '>') if self.chars.get(cursor + 1) == Some(&' ') => cursor += 1,
                _ => break,
            }
        }
        if !matches!(self.chars.get(cursor), None | Some(' ')) {
            self.operator();
            return;
        }
        let marker: String = self.chars[start..cursor].iter().collect();
        let style = if start == 0 && matches!(marker.as_str(), "///>" | "///!" | "///-") {
            Style::Attribute
        } else {
            Style::Comment
        };
        self.advance(self.chars.len() - start, style);
    }

    fn block_comment(&mut self) {
        let start = self.at;
        let (end, depth) = comment_scan(self.chars, start + 2, 1);
        self.emit(start, end, Style::Comment);
        self.at = end;
        self.state.comment_depth = depth;
    }

    fn operator(&mut self) {
        let start = self.at;
        let end = self.take_while(start, is_operator_char);
        let name: String = self.chars[start..end].iter().collect();
        if let Some(style) = symbol_style(&name) {
            self.advance(end - start, style);
            return;
        }
        let trimmed = name.trim_end_matches([',', ';']);
        if !trimmed.is_empty() && trimmed.len() < name.len() {
            if let Some(style) = symbol_style(trimmed) {
                self.advance(trimmed.chars().count(), style);
                return;
            }
        }
        if is_prefix_separator(self.before(start)) && !is_postfix_separator(self.chars.get(end).copied()) {
            self.advance(end - start, Style::Command);
        } else {
            self.at = end;
        }
    }

    fn macro_body(&mut self, close: char) {
        while let Some(ch) = self.peek(0) {
            match ch {
                _ if ch == close => {
                    self.advance(1, Style::String);
                    return;
                }
                '\\' => match self.peek(1) {
                    None => {
                        self.advance(1, Style::String);
                        self.state.macro_close = Some(close);
                        return;
                    }
                    Some('v') => self.advance(2, Style::Keyword),
                    Some(_) => self.advance(2, Style::Attribute),
                },
                '`' | '!' | '@' | '#' => self.splice(ch, close),
                _ => {
                    let end = self.take_while(self.at, |ch| {
                        ch != close && ch != '\\' && !matches!(ch, '`' | '!' | '@' | '#')
                    });
                    self.advance(end - self.at, Style::String);
                }
            }
        }
    }

    fn splice(&mut self, sigil: char, close: char) {
        let next = self.peek(1);
        if next == Some('(') {
            match self.splice_expression_end() {
                Some(end) => {
                    let body_start = self.at + 2;
                    let (inner, _) = tokenize_line(LexState::default(), &self.chars[body_start..end]);
                    for span in inner {
                        self.emit(body_start + span.start, body_start + span.end, span.style);
                    }
                    self.at = end + 1;
                }
                None => self.advance(1, Style::Attribute),
            }
        } else if next == Some(sigil) {
            self.advance(2, Style::Attribute);
        } else if next.is_some_and(is_var_start) {
            let mut end = self.take_while(self.at + 1, is_var_char);
            if is_var_char(close) && end - self.at > 2 && self.chars[end - 1] == close {
                let rest: String = self.chars[end..].iter().collect();
                if !rest.contains(close) && !rest.trim_end().ends_with('\\') {
                    end -= 1;
                }
            }
            let style = if matches!(sigil, '`' | '@') { Style::Type } else { Style::Variable };
            self.advance(end - self.at, style);
        } else {
            self.advance(1, Style::Attribute);
        }
    }

    fn splice_expression_end(&self) -> Option<usize> {
        let mut depth = 1usize;
        let mut cursor = self.at + 2;
        while cursor < self.chars.len() {
            match self.chars[cursor] {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return (cursor > self.at + 2).then_some(cursor);
                    }
                }
                _ => {}
            }
            cursor += 1;
        }
        None
    }
}
