use apollo_parser::{ast, Parser};

const QUERY: &str = r#"
    {
        field1
        field2
        field3
        field4
        field5
        field6
    }
"#;

fn main() {
    let parser = Parser::new(QUERY)
        .recursion_limit(5)
        .parse();

    println!("{:?}", parser.errors());
    println!("{:?}", parser.document());
}
