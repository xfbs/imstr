use imstr::ImString;
use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take_while},
    character::complete::{alphanumeric1 as alphanumeric, char, one_of},
    combinator::{cut, map, opt, value},
    error::{context, convert_error, ContextError, ErrorKind, ParseError, VerboseError},
    multi::separated_list0,
    number::complete::double,
    sequence::{delimited, preceded, separated_pair, terminated},
    Err, IResult,
};
use std::collections::HashMap;
use std::str;

#[derive(Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Str(ImString),
    Boolean(bool),
    Num(f64),
    Array(Vec<JsonValue>),
    Object(HashMap<ImString, JsonValue>),
}

fn sp<E: ParseError<ImString>>(i: ImString) -> IResult<ImString, ImString, E> {
    let chars = " \t\r\n";
    take_while(move |c| chars.contains(c))(i)
}

#[test]
fn test_sp() {
    assert_eq!(sp::<()>(ImString::from("")).unwrap().0, "");
    assert_eq!(sp::<()>(ImString::from("")).unwrap().1, "");

    assert_eq!(sp::<()>(ImString::from("abc")).unwrap().0, "abc");
    assert_eq!(sp::<()>(ImString::from("abc")).unwrap().1, "");

    assert_eq!(sp::<()>(ImString::from(" \r\nabc")).unwrap().0, "abc");
    assert_eq!(sp::<()>(ImString::from(" \r\nabc")).unwrap().1, " \r\n");
}

fn string_inner<E: ParseError<ImString>>(i: ImString) -> IResult<ImString, ImString, E> {
    escaped(alphanumeric, '\\', one_of("\"\\\\nrtfb"))(i)
}

#[test]
fn test_string_inner() {
    assert_eq!(string_inner::<()>(ImString::from("")).unwrap().0, "");
    assert_eq!(string_inner::<()>(ImString::from("")).unwrap().1, "");
    assert_eq!(string_inner::<()>(ImString::from("string")).unwrap().0, "");
    assert_eq!(
        string_inner::<()>(ImString::from("string")).unwrap().1,
        "string"
    );
    assert_eq!(
        string_inner::<()>(ImString::from("new\\nline\""))
            .unwrap()
            .0,
        "\""
    );
    assert_eq!(
        string_inner::<()>(ImString::from("new\\nline\""))
            .unwrap()
            .1,
        "new\\nline"
    );
    assert_eq!(
        string_inner::<()>(ImString::from("string\"end")).unwrap().0,
        "\"end"
    );
    assert_eq!(
        string_inner::<()>(ImString::from("string\"end")).unwrap().1,
        "string"
    );
}

fn boolean<E: ParseError<ImString>>(input: ImString) -> IResult<ImString, bool, E> {
    let parse_true = value(true, tag("true"));
    let parse_false = value(false, tag("false"));
    alt((parse_true, parse_false))(input)
}

#[test]
fn test_boolean() {
    assert_eq!(boolean::<()>(ImString::from("true")).unwrap().0, "");
    assert_eq!(boolean::<()>(ImString::from("true")).unwrap().1, true);
    assert_eq!(boolean::<()>(ImString::from("false")).unwrap().0, "");
    assert_eq!(boolean::<()>(ImString::from("false")).unwrap().1, false);
    assert!(boolean::<()>(ImString::from("xyz")).is_err());
}

fn null<E: ParseError<ImString>>(input: ImString) -> IResult<ImString, (), E> {
    value((), tag("null"))(input)
}

#[test]
fn test_null() {
    assert_eq!(null::<()>(ImString::from("null")).unwrap().0, "");
    assert_eq!(null::<()>(ImString::from("null")).unwrap().1, ());
    assert_eq!(null::<()>(ImString::from("null,")).unwrap().0, ",");
    assert_eq!(null::<()>(ImString::from("null,")).unwrap().1, ());
    assert!(null::<()>(ImString::from("xyz")).is_err());
}

fn string<E: ParseError<ImString> + ContextError<ImString>>(
    i: ImString,
) -> IResult<ImString, ImString, E> {
    context(
        "string",
        preceded(char('\"'), cut(terminated(string_inner, char('\"')))),
    )(i)
}

//#[test]
fn test_string() {
    assert_eq!(
        string::<(ImString, ErrorKind)>(ImString::from("\"json string\", "))
            .unwrap()
            .0,
        ""
    );
}

fn array<E: ParseError<ImString> + ContextError<ImString>>(
    i: ImString,
) -> IResult<ImString, Vec<JsonValue>, E> {
    context(
        "array",
        preceded(
            char('['),
            cut(terminated(
                separated_list0(preceded(sp, char(',')), json_value),
                preceded(sp, char(']')),
            )),
        ),
    )(i)
}

fn key_value<E: ParseError<ImString> + ContextError<ImString>>(
    i: ImString,
) -> IResult<ImString, (ImString, JsonValue), E> {
    separated_pair(
        preceded(sp, string),
        cut(preceded(sp, char(':'))),
        json_value,
    )(i)
}

fn hash<E: ParseError<ImString> + ContextError<ImString>>(
    i: ImString,
) -> IResult<ImString, HashMap<ImString, JsonValue>, E> {
    context(
        "map",
        preceded(
            char('{'),
            cut(terminated(
                map(
                    separated_list0(preceded(sp, char(',')), key_value),
                    |tuple_vec| tuple_vec.into_iter().collect(),
                ),
                preceded(sp, char('}')),
            )),
        ),
    )(i)
}

fn json_value<E: ParseError<ImString> + ContextError<ImString>>(
    i: ImString,
) -> IResult<ImString, JsonValue, E> {
    preceded(
        sp,
        alt((
            map(hash, JsonValue::Object),
            map(array, JsonValue::Array),
            map(string, |s| JsonValue::Str(s)),
            map(double, JsonValue::Num),
            map(boolean, JsonValue::Boolean),
            map(null, |_| JsonValue::Null),
        )),
    )(i)
}

fn root<E: ParseError<ImString> + ContextError<ImString>>(
    i: ImString,
) -> IResult<ImString, JsonValue, E> {
    delimited(
        sp,
        alt((
            map(hash, JsonValue::Object),
            map(array, JsonValue::Array),
            map(null, |_| JsonValue::Null),
        )),
        opt(sp),
    )(i)
}

fn main() {
    let mut input = std::io::stdin();
    let mut data = Vec::new();
    std::io::copy(&mut input, &mut data).unwrap();
    let string = ImString::from_utf8_lossy(&data);
    match root::<VerboseError<ImString>>(string.clone()) {
        Ok(result) => println!("{result:?}"),
        Err(Err::Error(error) | Err::Failure(error)) => {
            println!("{}", convert_error(string, error))
        }
        Err(other) => println!("{other}"),
    }
}
