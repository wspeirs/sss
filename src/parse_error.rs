use super::Rule;
use pest::iterators::Pair;

use std::fmt;

#[derive(Debug, Clone)]
pub struct ParseError {
    source: String,
    line: usize,
    reason: String
}

impl ParseError {
    pub fn new(rule: Pair<Rule>, reason: String) -> ParseError {
        let source = String::from(rule.as_str());
        let line = rule.as_span().start_pos().line_col().0;

        ParseError { source, line, reason}
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}: {}", self.line, self.source)?;
        writeln!(f, "{}", self.reason)
    }
}

