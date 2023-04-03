#![cfg(feature = "peg")]

use imstr::ImString;

#[test]
fn test_peg_easy() {
    peg::parser! {
        grammar list_parser() for ImString {
            rule number() -> u32 = n:$(['0'..='9']+) {? n.parse().or(Err("u32")) }

            pub rule list() -> Vec<u32> = "[" l:(number() ** ",") "]" { l }
        }
    }

    assert_eq!(
        list_parser::list(&"[1,1,2,3,5,8]".into()),
        Ok(vec![1, 1, 2, 3, 5, 8])
    );
}

#[derive(PartialEq, Debug)]
pub enum Atom {
    Number(ImString),
}

#[test]
fn test_peg_medium() {
    peg::parser! {
        grammar list_parser() for ImString {
            rule number() -> Atom = n:$(['0'..='9']+) {? n.parse::<u32>().map(|_| Atom::Number(n.into())).or(Err("u32")) }

            pub rule list() -> Vec<Atom> = "[" l:(number() ** ",") "]" { l }
        }
    }

    assert_eq!(
        list_parser::list(&"[1,2,3]".into()),
        Ok(vec![
            Atom::Number("1".into()),
            Atom::Number("2".into()),
            Atom::Number("3".into())
        ])
    );
}
