mod parser;

fn main() {
    let filename = std::env::args().nth(1).expect("missing filename");
    let source = std::fs::read_to_string(filename).expect("read failed");
    let mut parser = parser::Parser::new(&source);

    loop {
        match parser.read_token() {
            Ok(token) if token.is_eof() => break,
            Ok(token) => println!("{:?}", token),
            Err(error) => println!("{:?}", error),
        }
    }
}
