use crate::data::Data;
use crate::string::{CharIndices, Chars, ImString};
use nom::{
    error::{ErrorKind, ParseError},
    AsBytes, Compare, CompareResult, Err, IResult, InputIter, InputLength, InputTake,
    InputTakeAtPosition, Needed, Offset, ParseTo, Slice,
};
use std::ops::{Range, RangeFrom, RangeFull, RangeTo};
use std::str::FromStr;

impl<S: Data<String>> Slice<Range<usize>> for ImString<S> {
    fn slice(&self, range: Range<usize>) -> Self {
        self.slice(range)
    }
}

impl<S: Data<String>> Slice<RangeFrom<usize>> for ImString<S> {
    fn slice(&self, range: RangeFrom<usize>) -> Self {
        self.slice(range)
    }
}

impl<S: Data<String>> Slice<RangeTo<usize>> for ImString<S> {
    fn slice(&self, range: RangeTo<usize>) -> Self {
        self.slice(range)
    }
}

impl<S: Data<String>> Slice<RangeFull> for ImString<S> {
    fn slice(&self, range: RangeFull) -> Self {
        self.slice(range)
    }
}

impl<S: Data<String>> InputTake for ImString<S> {
    fn take(&self, count: usize) -> Self {
        self.slice(..count)
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        (self.slice(..count), self.slice(count..))
    }
}

impl<S: Data<String>> InputLength for ImString<S> {
    fn input_len(&self) -> usize {
        self.len()
    }
}

impl<S: Data<String>> InputIter for ImString<S> {
    type Item = char;
    type Iter = CharIndices<S>;
    type IterElem = Chars<S>;

    fn iter_indices(&self) -> Self::Iter {
        self.char_indices()
    }

    fn iter_elements(&self) -> Self::IterElem {
        self.chars()
    }

    fn position<P: Fn(Self::Item) -> bool>(&self, predicate: P) -> Option<usize> {
        self.as_str().find(predicate)
    }

    fn slice_index(&self, count: usize) -> Result<usize, Needed> {
        match self.iter_indices().skip(count).next() {
            Some((index, _)) => Ok(index),
            None => Err(Needed::Unknown),
        }
    }
}

impl<S: Data<String>> InputTakeAtPosition for ImString<S> {
    type Item = char;

    fn split_at_position<P, E: ParseError<Self>>(&self, predicate: P) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.as_str().find(predicate) {
            Some(i) => Ok((self.slice(i..), self.slice(..i))),
            None => Err(Err::Incomplete(Needed::new(1))),
        }
    }

    fn split_at_position1<P, E: ParseError<Self>>(
        &self,
        predicate: P,
        e: ErrorKind,
    ) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.as_str().find(predicate) {
            Some(0) => Err(Err::Error(E::from_error_kind(self.clone(), e))),
            Some(i) => Ok((self.slice(i..), self.slice(..i))),
            None => Err(Err::Incomplete(Needed::new(1))),
        }
    }

    fn split_at_position_complete<P, E: ParseError<Self>>(
        &self,
        predicate: P,
    ) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.as_str().find(predicate) {
            Some(i) => Ok((self.slice(i..), self.slice(..i))),
            None => Ok((self.slice(self.len()..), self.clone())),
        }
    }

    fn split_at_position1_complete<P, E: ParseError<Self>>(
        &self,
        predicate: P,
        e: ErrorKind,
    ) -> IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.as_str().find(predicate) {
            Some(0) => Err(Err::Error(E::from_error_kind(self.clone(), e))),
            Some(i) => Ok((self.slice(i..), self.slice(..i))),
            None if self.is_empty() => Err(Err::Error(E::from_error_kind(self.clone(), e))),
            None => Ok((self.slice(self.len()..), self.clone())),
        }
    }
}

impl<S: Data<String>> Offset for ImString<S> {
    fn offset(&self, second: &Self) -> usize {
        second.raw_offset().start - self.raw_offset().start
    }
}

impl<'a, S: Data<String>> Compare<&'a str> for ImString<S> {
    fn compare(&self, t: &'a str) -> CompareResult {
        self.as_str().compare(t)
    }

    fn compare_no_case(&self, t: &'a str) -> CompareResult {
        self.as_str().compare_no_case(t)
    }
}

impl<'a, S: Data<String>> Compare<&'a [u8]> for ImString<S> {
    fn compare(&self, t: &'a [u8]) -> CompareResult {
        self.as_bytes().compare(t)
    }

    fn compare_no_case(&self, t: &'a [u8]) -> CompareResult {
        self.as_bytes().compare_no_case(t)
    }
}

impl<S: Data<String>> AsBytes for ImString<S> {
    fn as_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<S: Data<String>, R: FromStr> ParseTo<R> for ImString<S> {
    fn parse_to(&self) -> Option<R> {
        self.parse().ok()
    }
}
