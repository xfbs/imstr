// Taken from https://github.com/rust-lang/rust/blob/master/library/alloc/tests/string.rs
use imstr::ImString;
use std::borrow::Cow;
use std::cell::Cell;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::ops::Bound;
use std::ops::Bound::*;
use std::ops::RangeBounds;
use std::panic;
use std::str;
use std::str::FromStr;

#[cfg(test)]
const EXAMPLE_STRINGS: &[&str] = &["", "text", "abcdef"];

#[test]
fn test_default() {
    let string = ImString::default();
    assert_eq!(string, "");
    assert_eq!(string.len(), 0);
}

#[test]
fn test_new() {
    let string = ImString::new();
    assert_eq!(string, "");
    assert_eq!(string.len(), 0);
}

#[test]
fn can_get_as_bytes() {
    for input in EXAMPLE_STRINGS.into_iter() {
        let string = ImString::from_std_string((*input).into());
        assert_eq!(string.as_bytes(), input.as_bytes());
    }
}

#[test]
fn can_deref() {
    for input in EXAMPLE_STRINGS.into_iter() {
        let string = ImString::from_std_string((*input).into());
        let string_slice: &str = &string;
        assert_eq!(&string_slice, input);
    }
}

#[test]
fn hash() {
    let mut hasher = DefaultHasher::new();
    let string = ImString::from("hello");
    string.hash(&mut hasher);
}

#[test]
fn test_from_utf8() {
    let xs = b"hello".to_vec();
    assert_eq!(ImString::from_utf8(xs).unwrap(), ImString::from("hello"));

    let xs = "ศไทย中华Việt Nam".as_bytes().to_vec();
    assert_eq!(
        ImString::from_utf8(xs).unwrap(),
        ImString::from("ศไทย中华Việt Nam")
    );

    let xs = b"hello\xFF".to_vec();
    let err = ImString::from_utf8(xs).unwrap_err();
    assert_eq!(err.as_bytes(), b"hello\xff");
    let err_clone = err.clone();
    assert_eq!(err, err_clone);
    assert_eq!(err.into_bytes(), b"hello\xff".to_vec());
    assert_eq!(err_clone.utf8_error().valid_up_to(), 5);
}

#[test]
fn test_push_str() {
    let mut s = ImString::new();
    s.push_str("");
    assert_eq!(&s[0..], "");
    s.push_str("abc");
    assert_eq!(&s[0..], "abc");
    s.push_str("ประเทศไทย中华Việt Nam");
    assert_eq!(&s[0..], "abcประเทศไทย中华Việt Nam");
}

#[test]
fn test_from_str() {
    let owned: Option<ImString> = "string".parse().ok();
    assert_eq!(owned.as_ref().map(|s| &**s), Some("string"));

    let input = "test";
    let string = ImString::from_str(input).unwrap();
    assert_eq!(&string, input);
}

#[test]
fn test_push() {
    let mut data = ImString::from("ประเทศไทย中");
    data.push('华');
    data.push('b'); // 1 byte
    data.push('¢'); // 2 byte
    data.push('€'); // 3 byte
    data.push('𤭢'); // 4 byte
    assert_eq!(&data, "ประเทศไทย中华b¢€𤭢");
}

#[test]
fn string_from_char_iter() {
    let chars = vec!['h', 'e', 'l', 'l', 'o'];
    let string: ImString = chars.into_iter().collect();
    assert_eq!(&string, "hello");
}

#[test]
fn test_add_assign() {
    let mut s = ImString::new();
    s += "";
    assert_eq!(s.as_str(), "");
    s += "abc";
    assert_eq!(s.as_str(), "abc");
    s += "ประเทศไทย中华Việt Nam";
    assert_eq!(s.as_str(), "abcประเทศไทย中华Việt Nam");
}

#[test]
fn test_from_char() {
    assert_eq!(&ImString::from('a'), "a");
    let s: ImString = 'x'.into();
    assert_eq!(&s, "x");
}

#[test]
fn test_str_concat() {
    let a: ImString = "hello".into();
    let b: ImString = "world".into();
    let s: ImString = format!("{a}{b}").into();
    assert_eq!(s.as_bytes()[9], 'd' as u8);
}

#[test]
fn test_extend_char() {
    let mut a: ImString = "foo".into();
    a.extend(['b', 'a', 'r']);
    assert_eq!(&a, "foobar");
}

#[test]
fn test_extend_char_ref() {
    let mut a: ImString = "foo".into();
    a.extend(&['b', 'a', 'r']);
    assert_eq!(&a, "foobar");
}

#[test]
fn test_str_clear() {
    let mut s = ImString::from("12345");
    s.clear();
    assert_eq!(s.len(), 0);
    assert_eq!(&s, "");
}

#[test]
fn test_str_add() {
    let a = ImString::from("12345");
    let b = a + "2";
    let b = b + "2";
    assert_eq!(b.len(), 7);
    assert_eq!(&b, "1234522");
}

#[test]
fn insert() {
    let mut s = ImString::from("foobar");
    s.insert(0, 'ệ');
    assert_eq!(s, "ệfoobar");
    s.insert(6, 'ย');
    assert_eq!(s, "ệfooยbar");
}

#[test]
#[should_panic]
fn insert_bad1() {
    ImString::from("").insert(1, 't');
}

#[test]
#[should_panic]
fn insert_bad2() {
    ImString::from("ệ").insert(1, 't');
}

#[test]
fn insert_str() {
    let mut s = ImString::from("foobar");
    s.insert_str(0, "ệ");
    assert_eq!(s, "ệfoobar");
    s.insert_str(6, "ย");
    assert_eq!(s, "ệfooยbar");
}

#[test]
#[should_panic]
fn insert_str_bad1() {
    ImString::from("").insert_str(1, "test");
}

#[test]
#[should_panic]
fn insert_str_bad2() {
    ImString::from("ệ").insert_str(1, "test");
}

#[test]
fn test_from_iterator() {
    let s = ImString::from("ศไทย中华Việt Nam");
    let t = "ศไทย中华";
    let u = "Việt Nam";

    let a: ImString = s.chars().collect();
    assert_eq!(s, a);

    let mut b: ImString = t.into();
    b.extend(u.chars());
    assert_eq!(s, b);

    let c: ImString = [t, u].into_iter().collect();
    assert_eq!(s, c);

    let mut d: ImString = t.into();
    d.extend(vec![u]);
    assert_eq!(s, d);
}

#[test]
fn test_from_cow_str() {
    assert_eq!(ImString::from(Cow::Borrowed("string")), "string");
    assert_eq!(ImString::from(Cow::Owned(String::from("string"))), "string");
}

#[test]
fn test_split_off_empty() {
    let orig = "Hello, world!";
    let mut split = ImString::from(orig);
    let empty: ImString = split.split_off(orig.len());
    assert!(empty.is_empty());
}

#[test]
#[should_panic]
fn test_split_off_past_end() {
    let orig = "Hello, world!";
    let mut split = ImString::from(orig);
    let _ = split.split_off(orig.len() + 1);
}

#[test]
#[should_panic]
fn test_split_off_mid_char() {
    let mut shan = ImString::from("山");
    let _broken_mountain = shan.split_off(1);
}

#[test]
fn test_split_off_ascii() {
    let mut ab = ImString::from("ABCD");
    let cd = ab.split_off(2);
    assert_eq!(ab, "AB");
    assert_eq!(cd, "CD");
}

#[test]
fn test_split_off_unicode() {
    let mut nihon = ImString::from("日本語");
    let go = nihon.split_off("日本".len());
    assert_eq!(nihon, "日本");
    assert_eq!(go, "語");
}

#[test]
fn test_lines() {
    let input = "data\nline\r\nabc\n\ndef\n";
    let string = ImString::from(input);
    for (left, right) in string.lines().zip(input.lines()) {
        assert_eq!(left, right);
    }
}

/*
pub trait IntoCow<'a, B: ?Sized>
where
    B: ToOwned,
{
    fn into_cow(self) -> Cow<'a, B>;
}

impl<'a> IntoCow<'a, str> for ImString {
    fn into_cow(self) -> Cow<'a, str> {
        Cow::Owned(self)
    }
}

impl<'a> IntoCow<'a, str> for &'a str {
    fn into_cow(self) -> Cow<'a, str> {
        Cow::Borrowed(self)
    }
}




#[test]
fn test_from_utf8_lossy() {
    let xs = b"hello";
    let ys: Cow<'_, str> = "hello".into_cow();
    assert_eq!(ImString::from_utf8_lossy(xs), ys);

    let xs = "ศไทย中华Việt Nam".as_bytes();
    let ys: Cow<'_, str> = "ศไทย中华Việt Nam".into_cow();
    assert_eq!(ImString::from_utf8_lossy(xs), ys);

    let xs = b"Hello\xC2 There\xFF Goodbye";
    assert_eq!(
        ImString::from_utf8_lossy(xs),
        ImString::from("Hello\u{FFFD} There\u{FFFD} Goodbye").into_cow()
    );

    let xs = b"Hello\xC0\x80 There\xE6\x83 Goodbye";
    assert_eq!(
        ImString::from_utf8_lossy(xs),
        ImString::from("Hello\u{FFFD}\u{FFFD} There\u{FFFD} Goodbye").into_cow()
    );

    let xs = b"\xF5foo\xF5\x80bar";
    assert_eq!(
        ImString::from_utf8_lossy(xs),
        ImString::from("\u{FFFD}foo\u{FFFD}\u{FFFD}bar").into_cow()
    );

    let xs = b"\xF1foo\xF1\x80bar\xF1\x80\x80baz";
    assert_eq!(
        ImString::from_utf8_lossy(xs),
        ImString::from("\u{FFFD}foo\u{FFFD}bar\u{FFFD}baz").into_cow()
    );

    let xs = b"\xF4foo\xF4\x80bar\xF4\xBFbaz";
    assert_eq!(
        ImString::from_utf8_lossy(xs),
        ImString::from("\u{FFFD}foo\u{FFFD}bar\u{FFFD}\u{FFFD}baz").into_cow()
    );

    let xs = b"\xF0\x80\x80\x80foo\xF0\x90\x80\x80bar";
    assert_eq!(
        ImString::from_utf8_lossy(xs),
        ImString::from("\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}foo\u{10000}bar").into_cow()
    );

    // surrogates
    let xs = b"\xED\xA0\x80foo\xED\xBF\xBFbar";
    assert_eq!(
        ImString::from_utf8_lossy(xs),
        ImString::from("\u{FFFD}\u{FFFD}\u{FFFD}foo\u{FFFD}\u{FFFD}\u{FFFD}bar").into_cow()
    );
}



#[test]
fn test_from_utf16() {
    let pairs = [
        (
            ImString::from("𐍅𐌿𐌻𐍆𐌹𐌻𐌰\n"),
            vec![
                0xd800, 0xdf45, 0xd800, 0xdf3f, 0xd800, 0xdf3b, 0xd800, 0xdf46, 0xd800, 0xdf39,
                0xd800, 0xdf3b, 0xd800, 0xdf30, 0x000a,
            ],
        ),
        (
            ImString::from("𐐒𐑉𐐮𐑀𐐲𐑋 𐐏𐐲𐑍\n"),
            vec![
                0xd801, 0xdc12, 0xd801, 0xdc49, 0xd801, 0xdc2e, 0xd801, 0xdc40, 0xd801, 0xdc32,
                0xd801, 0xdc4b, 0x0020, 0xd801, 0xdc0f, 0xd801, 0xdc32, 0xd801, 0xdc4d, 0x000a,
            ],
        ),
        (
            ImString::from("𐌀𐌖𐌋𐌄𐌑𐌉·𐌌𐌄𐌕𐌄𐌋𐌉𐌑\n"),
            vec![
                0xd800, 0xdf00, 0xd800, 0xdf16, 0xd800, 0xdf0b, 0xd800, 0xdf04, 0xd800, 0xdf11,
                0xd800, 0xdf09, 0x00b7, 0xd800, 0xdf0c, 0xd800, 0xdf04, 0xd800, 0xdf15, 0xd800,
                0xdf04, 0xd800, 0xdf0b, 0xd800, 0xdf09, 0xd800, 0xdf11, 0x000a,
            ],
        ),
        (
            ImString::from("𐒋𐒘𐒈𐒑𐒛𐒒 𐒕𐒓 𐒈𐒚𐒍 𐒏𐒜𐒒𐒖𐒆 𐒕𐒆\n"),
            vec![
                0xd801, 0xdc8b, 0xd801, 0xdc98, 0xd801, 0xdc88, 0xd801, 0xdc91, 0xd801, 0xdc9b,
                0xd801, 0xdc92, 0x0020, 0xd801, 0xdc95, 0xd801, 0xdc93, 0x0020, 0xd801, 0xdc88,
                0xd801, 0xdc9a, 0xd801, 0xdc8d, 0x0020, 0xd801, 0xdc8f, 0xd801, 0xdc9c, 0xd801,
                0xdc92, 0xd801, 0xdc96, 0xd801, 0xdc86, 0x0020, 0xd801, 0xdc95, 0xd801, 0xdc86,
                0x000a,
            ],
        ),
        // Issue #12318, even-numbered non-BMP planes
        (ImString::from("\u{20000}"), vec![0xD840, 0xDC00]),
    ];

    for p in &pairs {
        let (s, u) = (*p).clone();
        let s_as_utf16 = s.encode_utf16().collect::<Vec<u16>>();
        let u_as_string = ImString::from_utf16(&u).unwrap();

        assert!(core::char::decode_utf16(u.iter().cloned()).all(|r| r.is_ok()));
        assert_eq!(s_as_utf16, u);

        assert_eq!(u_as_string, s);
        assert_eq!(ImString::from_utf16_lossy(&u), s);

        assert_eq!(ImString::from_utf16(&s_as_utf16).unwrap(), s);
        assert_eq!(u_as_string.encode_utf16().collect::<Vec<u16>>(), u);
    }
}

#[test]
fn test_utf16_invalid() {
    // completely positive cases tested above.
    // lead + eof
    assert!(ImString::from_utf16(&[0xD800]).is_err());
    // lead + lead
    assert!(ImString::from_utf16(&[0xD800, 0xD800]).is_err());

    // isolated trail
    assert!(ImString::from_utf16(&[0x0061, 0xDC00]).is_err());

    // general
    assert!(ImString::from_utf16(&[0xD800, 0xd801, 0xdc8b, 0xD800]).is_err());
}

#[test]
fn test_from_utf16_lossy() {
    // completely positive cases tested above.
    // lead + eof
    assert_eq!(ImString::from_utf16_lossy(&[0xD800]), ImString::from("\u{FFFD}"));
    // lead + lead
    assert_eq!(ImString::from_utf16_lossy(&[0xD800, 0xD800]), ImString::from("\u{FFFD}\u{FFFD}"));

    // isolated trail
    assert_eq!(ImString::from_utf16_lossy(&[0x0061, 0xDC00]), ImString::from("a\u{FFFD}"));

    // general
    assert_eq!(
        ImString::from_utf16_lossy(&[0xD800, 0xd801, 0xdc8b, 0xD800]),
        ImString::from("\u{FFFD}𐒋\u{FFFD}")
    );
}

#[test]
fn test_push_bytes() {
    let mut s = ImString::from("ABC");
    unsafe {
        let mv = s.as_mut_vec();
        mv.extend_from_slice(&[b'D']);
    }
    assert_eq!(s, "ABCD");
}

#[test]
fn test_pop() {
    let mut data = ImString::from("ประเทศไทย中华b¢€𤭢");
    assert_eq!(data.pop().unwrap(), '𤭢'); // 4 bytes
    assert_eq!(data.pop().unwrap(), '€'); // 3 bytes
    assert_eq!(data.pop().unwrap(), '¢'); // 2 bytes
    assert_eq!(data.pop().unwrap(), 'b'); // 1 bytes
    assert_eq!(data.pop().unwrap(), '华');
    assert_eq!(data, "ประเทศไทย中");
}

#[test]
fn test_str_truncate() {
    let mut s = ImString::from("12345");
    s.truncate(5);
    assert_eq!(s, "12345");
    s.truncate(3);
    assert_eq!(s, "123");
    s.truncate(0);
    assert_eq!(s, "");

    let mut s = ImString::from("12345");
    let p = s.as_ptr();
    s.truncate(3);
    s.push_str("6");
    let p_ = s.as_ptr();
    assert_eq!(p_, p);
}

#[test]
fn test_str_truncate_invalid_len() {
    let mut s = ImString::from("12345");
    s.truncate(6);
    assert_eq!(s, "12345");
}

#[test]
#[should_panic]
fn test_str_truncate_split_codepoint() {
    let mut s = ImString::from("\u{FC}"); // ü
    s.truncate(1);
}

#[test]
fn remove() {
    let mut s = "ศไทย中华Việt Nam; foobar".to_string();
    assert_eq!(s.remove(0), 'ศ');
    assert_eq!(s.len(), 33);
    assert_eq!(s, "ไทย中华Việt Nam; foobar");
    assert_eq!(s.remove(17), 'ệ');
    assert_eq!(s, "ไทย中华Vit Nam; foobar");
}

#[test]
#[should_panic]
fn remove_bad() {
    "ศ".to_string().remove(1);
}

#[test]
fn test_retain() {
    let mut s = ImString::from("α_β_γ");

    s.retain(|_| true);
    assert_eq!(s, "α_β_γ");

    s.retain(|c| c != '_');
    assert_eq!(s, "αβγ");

    s.retain(|c| c != 'β');
    assert_eq!(s, "αγ");

    s.retain(|c| c == 'α');
    assert_eq!(s, "α");

    s.retain(|_| false);
    assert_eq!(s, "");

    let mut s = ImString::from("0è0");
    let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let mut count = 0;
        s.retain(|_| {
            count += 1;
            match count {
                1 => false,
                2 => true,
                _ => panic!(),
            }
        });
    }));
    assert!(std::str::from_utf8(s.as_bytes()).is_ok());
}

#[test]
fn test_slicing() {
    let s = "foobar".to_string();
    assert_eq!("foobar", &s[..]);
    assert_eq!("foo", &s[..3]);
    assert_eq!("bar", &s[3..]);
    assert_eq!("oob", &s[1..4]);
}

#[test]
fn test_simple_types() {
    assert_eq!(1.to_string(), "1");
    assert_eq!((-1).to_string(), "-1");
    assert_eq!(200.to_string(), "200");
    assert_eq!(2.to_string(), "2");
    assert_eq!(true.to_string(), "true");
    assert_eq!(false.to_string(), "false");
    assert_eq!(("hi".to_string()).to_string(), "hi");
}

#[test]
fn test_vectors() {
    let x: Vec<i32> = vec![];
    assert_eq!(format!("{x:?}"), "[]");
    assert_eq!(format!("{:?}", vec![1]), "[1]");
    assert_eq!(format!("{:?}", vec![1, 2, 3]), "[1, 2, 3]");
    assert!(format!("{:?}", vec![vec![], vec![1], vec![1, 1]]) == "[[], [1], [1, 1]]");
}

#[test]
fn test_drain() {
    let mut s = ImString::from("αβγ");
    assert_eq!(s.drain(2..4).collect::<ImString>(), "β");
    assert_eq!(s, "αγ");

    let mut t = ImString::from("abcd");
    t.drain(..0);
    assert_eq!(t, "abcd");
    t.drain(..1);
    assert_eq!(t, "bcd");
    t.drain(3..);
    assert_eq!(t, "bcd");
    t.drain(..);
    assert_eq!(t, "");
}

#[test]
#[should_panic]
fn test_drain_start_overflow() {
    let mut s = ImString::from("abc");
    s.drain((Excluded(usize::MAX), Included(0)));
}

#[test]
#[should_panic]
fn test_drain_end_overflow() {
    let mut s = ImString::from("abc");
    s.drain((Included(0), Included(usize::MAX)));
}

#[test]
fn test_replace_range() {
    let mut s = "Hello, world!".to_owned();
    s.replace_range(7..12, "世界");
    assert_eq!(s, "Hello, 世界!");
}

#[test]
#[should_panic]
fn test_replace_range_char_boundary() {
    let mut s = "Hello, 世界!".to_owned();
    s.replace_range(..8, "");
}

#[test]
fn test_replace_range_inclusive_range() {
    let mut v = ImString::from("12345");
    v.replace_range(2..=3, "789");
    assert_eq!(v, "127895");
    v.replace_range(1..=2, "A");
    assert_eq!(v, "1A895");
}

#[test]
#[should_panic]
fn test_replace_range_out_of_bounds() {
    let mut s = ImString::from("12345");
    s.replace_range(5..6, "789");
}

#[test]
#[should_panic]
fn test_replace_range_inclusive_out_of_bounds() {
    let mut s = ImString::from("12345");
    s.replace_range(5..=5, "789");
}

#[test]
#[should_panic]
fn test_replace_range_start_overflow() {
    let mut s = ImString::from("123");
    s.replace_range((Excluded(usize::MAX), Included(0)), "");
}

#[test]
#[should_panic]
fn test_replace_range_end_overflow() {
    let mut s = ImString::from("456");
    s.replace_range((Included(0), Included(usize::MAX)), "");
}

#[test]
fn test_replace_range_empty() {
    let mut s = ImString::from("12345");
    s.replace_range(1..2, "");
    assert_eq!(s, "1345");
}

#[test]
fn test_replace_range_unbounded() {
    let mut s = ImString::from("12345");
    s.replace_range(.., "");
    assert_eq!(s, "");
}

#[test]
fn test_replace_range_evil_start_bound() {
    struct EvilRange(Cell<bool>);

    impl RangeBounds<usize> for EvilRange {
        fn start_bound(&self) -> Bound<&usize> {
            Bound::Included(if self.0.get() {
                &1
            } else {
                self.0.set(true);
                &0
            })
        }
        fn end_bound(&self) -> Bound<&usize> {
            Bound::Unbounded
        }
    }

    let mut s = ImString::from("🦀");
    s.replace_range(EvilRange(Cell::new(false)), "");
    assert_eq!(Ok(""), str::from_utf8(s.as_bytes()));
}

#[test]
fn test_replace_range_evil_end_bound() {
    struct EvilRange(Cell<bool>);

    impl RangeBounds<usize> for EvilRange {
        fn start_bound(&self) -> Bound<&usize> {
            Bound::Included(&0)
        }
        fn end_bound(&self) -> Bound<&usize> {
            Bound::Excluded(if self.0.get() {
                &3
            } else {
                self.0.set(true);
                &4
            })
        }
    }

    let mut s = ImString::from("🦀");
    s.replace_range(EvilRange(Cell::new(false)), "");
    assert_eq!(Ok(""), str::from_utf8(s.as_bytes()));
}

#[test]
fn test_into_boxed_str() {
    let xs = ImString::from("hello my name is bob");
    let ys = xs.into_boxed_str();
    assert_eq!(&*ys, "hello my name is bob");
}

#[test]
fn test_reserve_exact() {
    // This is all the same as test_reserve

    let mut s = ImString::new();
    assert_eq!(s.capacity(), 0);

    s.reserve_exact(2);
    assert!(s.capacity() >= 2);

    for _i in 0..16 {
        s.push('0');
    }

    assert!(s.capacity() >= 16);
    s.reserve_exact(16);
    assert!(s.capacity() >= 32);

    s.push('0');

    s.reserve_exact(16);
    assert!(s.capacity() >= 33)
}

*/
