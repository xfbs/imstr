use crate::data::Data;
use crate::string::ImString;
use peg_runtime::str::LineCol;
use peg_runtime::Parse;
use peg_runtime::ParseElem;
use peg_runtime::ParseLiteral;
use peg_runtime::ParseSlice;
use peg_runtime::RuleResult;

#[cfg(feature = "std")]
use std::string::String;

#[cfg(not(feature = "std"))]
use alloc::string::String;

impl<T: Data<String>> Parse for ImString<T> {
    type PositionRepr = LineCol;
    fn start(&self) -> usize {
        0
    }

    fn is_eof(&self, pos: usize) -> bool {
        pos >= self.len()
    }

    fn position_repr(&self, pos: usize) -> LineCol {
        let before = &self[..pos];
        let line = before.as_bytes().iter().filter(|&&c| c == b'\n').count() + 1;
        let column = before.chars().rev().take_while(|&c| c != '\n').count() + 1;
        LineCol {
            line,
            column,
            offset: pos,
        }
    }
}

impl<'input, T: Data<String>> ParseElem<'input> for ImString<T> {
    type Element = char;

    fn parse_elem(&'input self, pos: usize) -> RuleResult<char> {
        match self[pos..].chars().next() {
            Some(c) => RuleResult::Matched(pos + c.len_utf8(), c),
            None => RuleResult::Failed,
        }
    }
}

impl<T: Data<String>> ParseLiteral for ImString<T> {
    fn parse_string_literal(&self, pos: usize, literal: &str) -> RuleResult<()> {
        let l = literal.len();
        if self.len() >= pos + l && &self.as_bytes()[pos..pos + l] == literal.as_bytes() {
            RuleResult::Matched(pos + l, ())
        } else {
            RuleResult::Failed
        }
    }
}

impl<'input, T: Data<String>> ParseSlice<'input> for ImString<T> {
    type Slice = &'input str;
    fn parse_slice(&'input self, p1: usize, p2: usize) -> &'input str {
        &self[p1..p2]
    }
}
