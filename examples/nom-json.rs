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
    Str(String),
    Boolean(bool),
    Num(f64),
    Array(Vec<JsonValue>),
    Object(HashMap<ImString, JsonValue>),
}

fn sp<E: ParseError<ImString>>(i: ImString) -> IResult<ImString, ImString, E> {
    let chars = " \t\r\n";
    take_while(move |c| chars.contains(c))(i)
}

fn parse_str<E: ParseError<ImString>>(i: ImString) -> IResult<ImString, ImString, E> {
    escaped(alphanumeric, '\\', one_of("\"n\\"))(i)
}

fn boolean<E: ParseError<ImString>>(input: ImString) -> IResult<ImString, bool, E> {
    let parse_true = value(true, tag("true"));
    let parse_false = value(false, tag("false"));
    alt((parse_true, parse_false))(input)
}

fn null<E: ParseError<ImString>>(input: ImString) -> IResult<ImString, (), E> {
    value((), tag("null"))(input)
}

fn string<E: ParseError<ImString> + ContextError<ImString>>(
    i: ImString,
) -> IResult<ImString, ImString, E> {
    context(
        "string",
        preceded(char('\"'), cut(terminated(parse_str, char('\"')))),
    )(i)
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
            map(string, |s| JsonValue::Str(String::from(s))),
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
    let result = root::<(ImString, ErrorKind)>(string);
    println!("{result:?}");
}
