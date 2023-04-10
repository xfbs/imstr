use crate::data::Data;
use crate::string::{CharIndices, Chars, ImString};
use core::ops::{Range, RangeFrom, RangeFull, RangeTo};
use core::str::FromStr;
use nom::{
    error::{ErrorKind, ParseError},
    AsBytes, Compare, CompareResult, Err, IResult, InputIter, InputLength, InputTake,
    InputTakeAtPosition, Needed, Offset, ParseTo, Slice,
};

/// Test that the specified function behaves the same regardless of whether the type is `&str` or
/// `ImString`.
#[cfg(test)]
macro_rules! test_equivalence {
    ($input:expr, |$name:ident: $type:path $(, $extra:path)*| $body:tt) => {{
        fn test<'a>($name: impl $type $(+ $extra)* + PartialEq<&'a str> + std::fmt::Debug) {
            $body
        }

        let input = $input;

        println!("Testing {input:?} for &str");
        test(input);

        println!("Testing {input:?} for ImString<Arc<String>>");
        test(ImString::<std::sync::Arc<String>>::from(input));

        println!("Testing {input:?} for ImString<Rc<String>>");
        test(ImString::<std::rc::Rc<String>>::from(input));

        println!("Testing {input:?} for ImString<Box<String>>");
        test(ImString::<std::boxed::Box<String>>::from(input));
    }};
}

impl<S: Data<String>> Slice<Range<usize>> for ImString<S> {
    fn slice(&self, range: Range<usize>) -> Self {
        self.slice(range)
    }
}

#[test]
fn test_slice_range() {
    test_equivalence!("this is some string", |string: Slice<Range<usize>>| {
        assert_eq!(string.slice(0..0), "");
        assert_eq!(string.slice(0..4), "this");
        assert_eq!(string.slice(5..7), "is");
        assert_eq!(string.slice(8..12), "some");
        assert_eq!(string.slice(13..19), "string");
    });
}

impl<S: Data<String>> Slice<RangeFrom<usize>> for ImString<S> {
    fn slice(&self, range: RangeFrom<usize>) -> Self {
        self.slice(range)
    }
}

#[test]
fn test_slice_range_from() {
    test_equivalence!("this is some string", |string: Slice<RangeFrom<usize>>| {
        assert_eq!(string.slice(0..), "this is some string");
        assert_eq!(string.slice(8..), "some string");
        assert_eq!(string.slice(13..), "string");
        assert_eq!(string.slice(19..), "");
    });
}

impl<S: Data<String>> Slice<RangeTo<usize>> for ImString<S> {
    fn slice(&self, range: RangeTo<usize>) -> Self {
        self.slice(range)
    }
}

#[test]
fn test_slice_range_to() {
    test_equivalence!("this is some string", |string: Slice<RangeTo<usize>>| {
        assert_eq!(string.slice(..0), "");
        assert_eq!(string.slice(..4), "this");
        assert_eq!(string.slice(..7), "this is");
        assert_eq!(string.slice(..12), "this is some");
    });
}

impl<S: Data<String>> Slice<RangeFull> for ImString<S> {
    fn slice(&self, range: RangeFull) -> Self {
        self.slice(range)
    }
}

#[test]
fn test_slice_range_full() {
    test_equivalence!("this is some string", |string: Slice<RangeFull>| {
        assert_eq!(string.slice(..), "this is some string");
    });

    test_equivalence!("", |string: Slice<RangeFull>| {
        assert_eq!(string.slice(..), "");
    });

    test_equivalence!("string", |string: Slice<RangeFull>| {
        assert_eq!(string.slice(..), "string");
    });
}

impl<S: Data<String>> InputTake for ImString<S> {
    fn take(&self, count: usize) -> Self {
        self.slice(..count)
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        (self.slice(count..), self.slice(..count))
    }
}

#[test]
fn test_input_take() {
    test_equivalence!("this is some string", |string: InputTake| {
        assert_eq!(string.take(0), "");
        assert_eq!(string.take(4), "this");
        assert_eq!(string.take(19), "this is some string");

        assert_eq!(string.take_split(0).1, "");
        assert_eq!(string.take_split(0).0, "this is some string");

        assert_eq!(string.take_split(4).1, "this");
        assert_eq!(string.take_split(4).0, " is some string");

        assert_eq!(string.take_split(7).1, "this is");
        assert_eq!(string.take_split(7).0, " some string");

        assert_eq!(string.take_split(12).1, "this is some");
        assert_eq!(string.take_split(12).0, " string");

        assert_eq!(string.take_split(19).1, "this is some string");
        assert_eq!(string.take_split(19).0, "");
    });
}

impl<S: Data<String>> InputLength for ImString<S> {
    fn input_len(&self) -> usize {
        self.len()
    }
}

#[test]
fn test_input_length() {
    test_equivalence!("this is some string", |string: InputLength| {
        assert_eq!(string.input_len(), 19);
    });

    test_equivalence!("", |string: InputLength| {
        assert_eq!(string.input_len(), 0);
    });

    test_equivalence!("string", |string: InputLength| {
        assert_eq!(string.input_len(), 6);
    });
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
        let mut cnt = 0;
        for (index, _) in self.char_indices() {
            if cnt == count {
                return Ok(index);
            }
            cnt += 1;
        }
        if cnt == count {
            return Ok(self.len());
        }
        Err(Needed::Unknown)
    }
}

#[test]
fn test_input_iter() {
    test_equivalence!("", |string: InputIter<Item = char>| {
        assert_eq!(string.iter_indices().next(), None);
        assert_eq!(string.iter_elements().next(), None);
        assert_eq!(string.position(|_| true), None);
        assert_eq!(string.slice_index(0), Ok(0));
        assert_eq!(string.slice_index(1), Err(Needed::Unknown));
    });

    test_equivalence!("über", |string: InputIter<Item = char>| {
        let indices: Vec<_> = string.iter_indices().collect();
        assert_eq!(indices, &[(0, 'ü'), (2, 'b'), (3, 'e'), (4, 'r')]);
        let chars: Vec<_> = string.iter_elements().collect();
        assert_eq!(chars, &['ü', 'b', 'e', 'r']);

        assert_eq!(string.position(|_| true), Some(0));
        assert_eq!(string.position(|c| c == 'ü'), Some(0));
        assert_eq!(string.position(|c| c == 'b'), Some(2));
        assert_eq!(string.position(|c| c == 'e'), Some(3));
        assert_eq!(string.position(|c| c == 'r'), Some(4));

        assert_eq!(string.slice_index(0), Ok(0));
        assert_eq!(string.slice_index(1), Ok(2));
        assert_eq!(string.slice_index(2), Ok(3));
        assert_eq!(string.slice_index(3), Ok(4));
        assert_eq!(string.slice_index(4), Ok(5));
        assert_eq!(string.slice_index(5), Err(Needed::Unknown));
    });
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

#[test]
fn test_input_take_at_position() {
    test_equivalence!("", |string: InputTakeAtPosition<Item = char>| {
        assert_eq!(
            string.split_at_position::<_, ()>(|_| true).err().unwrap(),
            Err::Incomplete(Needed::new(1))
        );

        assert_eq!(
            string
                .split_at_position1::<_, ()>(|_| true, ErrorKind::Fail)
                .err()
                .unwrap(),
            Err::Incomplete(Needed::new(1))
        );

        let result = string
            .split_at_position_complete::<_, ()>(|_| true)
            .unwrap();
        assert_eq!(result.0, "");
        assert_eq!(result.1, "");

        let result = string
            .split_at_position1_complete::<_, ()>(|_| true, ErrorKind::Fail)
            .err()
            .unwrap();
        assert_eq!(result, Err::Error(()));
    });

    test_equivalence!("some input", |string: InputTakeAtPosition<Item = char>| {
        assert_eq!(
            string
                .split_at_position::<_, ()>(|c| c == 'x')
                .err()
                .unwrap(),
            Err::Incomplete(Needed::new(1))
        );

        let result = string.split_at_position::<_, ()>(|c| c == ' ').unwrap();
        assert_eq!(result.0, " input");
        assert_eq!(result.1, "some");

        assert_eq!(
            string
                .split_at_position1::<_, ()>(|c| c == 'x', ErrorKind::Fail)
                .err()
                .unwrap(),
            Err::Incomplete(Needed::new(1))
        );
        let result = string
            .split_at_position1::<_, ()>(|c| c == ' ', ErrorKind::Fail)
            .unwrap();
        assert_eq!(result.0, " input");
        assert_eq!(result.1, "some");
        assert_eq!(
            string
                .split_at_position1::<_, ()>(|c| c == 's', ErrorKind::Fail)
                .err()
                .unwrap(),
            Err::Error(())
        );

        let result = string
            .split_at_position_complete::<_, ()>(|_| true)
            .unwrap();
        assert_eq!(result.0, "some input");
        assert_eq!(result.1, "");

        let result = string
            .split_at_position_complete::<_, ()>(|c| c == ' ')
            .unwrap();
        assert_eq!(result.0, " input");
        assert_eq!(result.1, "some");

        let result = string
            .split_at_position_complete::<_, ()>(|_| false)
            .unwrap();
        assert_eq!(result.0, "");
        assert_eq!(result.1, "some input");

        let result = string
            .split_at_position1_complete::<_, ()>(|_| true, ErrorKind::Fail)
            .err()
            .unwrap();
        assert_eq!(result, Err::Error(()));

        let result = string
            .split_at_position1_complete::<_, ()>(|c| c == ' ', ErrorKind::Fail)
            .unwrap();
        assert_eq!(result.0, " input");
        assert_eq!(result.1, "some");

        let result = string
            .split_at_position1_complete::<_, ()>(|_| false, ErrorKind::Fail)
            .unwrap();
        assert_eq!(result.0, "");
        assert_eq!(result.1, "some input");
    });
}

impl<S: Data<String>> Offset for ImString<S> {
    fn offset(&self, second: &Self) -> usize {
        second.raw_offset().start - self.raw_offset().start
    }
}

#[test]
fn test_offset() {
    test_equivalence!("", |string: Offset| {
        assert_eq!(string.offset(&string), 0);
    });

    test_equivalence!("hello", |string: Offset, Slice<Range<usize>>| {
        assert_eq!(string.offset(&string), 0);
        assert_eq!(string.offset(&string.slice(1..5)), 1);
        assert_eq!(string.offset(&string.slice(2..5)), 2);
        assert_eq!(string.offset(&string.slice(3..5)), 3);
        assert_eq!(string.offset(&string.slice(4..5)), 4);
        assert_eq!(string.offset(&string.slice(5..5)), 5);
    });
}

impl<'a, S: Data<String>> Compare<&'a str> for ImString<S> {
    fn compare(&self, t: &'a str) -> CompareResult {
        self.as_str().compare(t)
    }

    fn compare_no_case(&self, t: &'a str) -> CompareResult {
        self.as_str().compare_no_case(t)
    }
}

#[test]
fn test_compare_str() {
    test_equivalence!("", |string: Compare<&'a str>| {
        assert_eq!(string.compare(""), CompareResult::Ok);
        assert_eq!(string.compare("err"), CompareResult::Incomplete);

        assert_eq!(string.compare_no_case(""), CompareResult::Ok);
        assert_eq!(string.compare_no_case("err"), CompareResult::Incomplete);
    });

    test_equivalence!("string", |string: Compare<&'a str>| {
        assert_eq!(string.compare("string"), CompareResult::Ok);
        assert_eq!(string.compare("str"), CompareResult::Ok);
        assert_eq!(string.compare("string0"), CompareResult::Incomplete);
        assert_eq!(string.compare("var"), CompareResult::Error);

        assert_eq!(string.compare_no_case("STRING"), CompareResult::Ok);
        assert_eq!(string.compare_no_case("STR"), CompareResult::Ok);
        assert_eq!(string.compare_no_case("STRING0"), CompareResult::Incomplete);
        assert_eq!(string.compare_no_case("VAR"), CompareResult::Error);
    });
}

impl<'a, S: Data<String>> Compare<&'a [u8]> for ImString<S> {
    fn compare(&self, t: &'a [u8]) -> CompareResult {
        self.as_bytes().compare(t)
    }

    fn compare_no_case(&self, t: &'a [u8]) -> CompareResult {
        self.as_bytes().compare_no_case(t)
    }
}

#[test]
fn test_compare_bytes() {
    test_equivalence!("", |string: Compare<&'a [u8]>| {
        assert_eq!(string.compare(&[]), CompareResult::Ok);
        assert_eq!(string.compare(&[101, 108]), CompareResult::Incomplete);

        assert_eq!(string.compare_no_case(&[]), CompareResult::Ok);
        assert_eq!(
            string.compare_no_case(&[101, 108]),
            CompareResult::Incomplete
        );
    });

    test_equivalence!("string", |string: Compare<&'a [u8]>| {
        assert_eq!(
            string.compare(&[115, 116, 114, 105, 110, 103]),
            CompareResult::Ok
        );
        assert_eq!(string.compare(&[115, 116, 114]), CompareResult::Ok);
        assert_eq!(
            string.compare(&[115, 116, 114, 105, 110, 103, 100]),
            CompareResult::Incomplete
        );
        assert_eq!(string.compare(&[116, 116, 116]), CompareResult::Error);

        assert_eq!(
            string.compare_no_case(&[83, 84, 82, 73, 78, 71]),
            CompareResult::Ok
        );
        assert_eq!(string.compare_no_case(&[83, 84, 82]), CompareResult::Ok);
        assert_eq!(
            string.compare_no_case(&[83, 84, 82, 73, 78, 71, 100]),
            CompareResult::Incomplete
        );
        assert_eq!(string.compare_no_case(&[84, 84, 84]), CompareResult::Error);
    });
}

impl<S: Data<String>> AsBytes for ImString<S> {
    fn as_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

#[test]
fn test_as_bytes() {
    test_equivalence!("", |string: AsBytes| {
        assert_eq!(string.as_bytes(), &[]);
    });

    test_equivalence!("hello", |string: AsBytes| {
        assert_eq!(string.as_bytes(), &[104, 101, 108, 108, 111]);
    });

    test_equivalence!("über", |string: AsBytes| {
        assert_eq!(string.as_bytes(), &[195, 188, 98, 101, 114]);
    });
}

impl<S: Data<String>, R: FromStr> ParseTo<R> for ImString<S> {
    fn parse_to(&self) -> Option<R> {
        self.parse().ok()
    }
}

#[test]
fn test_parse_to() {
    test_equivalence!("", |string: ParseTo<i64>| {
        assert_eq!(string.parse_to(), None);
    });

    test_equivalence!("14", |string: ParseTo<i64>| {
        assert_eq!(string.parse_to(), Some(14));
    });

    test_equivalence!("-9", |string: ParseTo<i64>| {
        assert_eq!(string.parse_to(), Some(-9));
    });
}
