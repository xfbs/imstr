use crate::data::Data;
use crate::string::ImString;
use peg_runtime::str::LineCol;
use peg_runtime::Parse;
use peg_runtime::ParseElem;
use peg_runtime::ParseLiteral;
use peg_runtime::ParseSlice;
use peg_runtime::RuleResult;

impl<T: Data<String>> Parse for ImString<T> {
    type PositionRepr = LineCol;
    fn start(&self) -> usize {
        0
    }

    fn is_eof(&self, pos: usize) -> bool {
        pos >= self.len()
    }

    fn position_repr(&self, pos: usize) -> LineCol {
        self.as_str().position_repr(pos)
    }
}

impl<'input, T: Data<String>> ParseElem<'input> for ImString<T> {
    type Element = char;

    fn parse_elem(&'input self, pos: usize) -> RuleResult<char> {
        self.as_str().parse_elem(pos)
    }
}

impl<T: Data<String>> ParseLiteral for ImString<T> {
    fn parse_string_literal(&self, pos: usize, literal: &str) -> RuleResult<()> {
        self.as_str().parse_string_literal(pos, literal)
    }
}

impl<'input, S: Data<String>> ParseSlice<'input> for ImString<S> {
    type Slice = ImString<S>;
    fn parse_slice(&'input self, p1: usize, p2: usize) -> ImString<S> {
        self.slice(p1..p2)
    }
}
