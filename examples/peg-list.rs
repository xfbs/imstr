use imstr::ImString;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(u32),
    String(ImString),
}

peg::parser! {
  grammar list_parser() for ImString {
    rule number() -> u32
      = n:$(['0'..='9']+) {? n.parse().or(Err("u32")) }

    rule string() -> ImString
      = "\"" s:$(['a'..='z']+) "\"" { s }

    rule value() -> Value
      = n:number() { Value::Number(n) } / s:string() { Value::String(s) }

    pub rule list() -> Vec<Value>
      = "[" l:(value() ** ",") "]" { l }
  }
}

#[test]
fn test_list_parser() {
    assert_eq!(
        list_parser::list(&ImString::from("[1,1,2,3,5,8]")).unwrap(),
        vec![
            Value::Number(1),
            Value::Number(1),
            Value::Number(2),
            Value::Number(3),
            Value::Number(5),
            Value::Number(8)
        ]
    );

    assert_eq!(
        list_parser::list(&ImString::from("[\"hello\",\"world\",1,2,3]")).unwrap(),
        vec![
            Value::String("hello".into()),
            Value::String("world".into()),
            Value::Number(1),
            Value::Number(2),
            Value::Number(3),
        ]
    );
}

fn main() {
    let mut input = std::io::stdin();
    let mut data = Vec::new();
    std::io::copy(&mut input, &mut data).unwrap();
    let string = ImString::from_utf8_lossy(&data).trim();
    println!("{:?}", list_parser::list(&string));
}
