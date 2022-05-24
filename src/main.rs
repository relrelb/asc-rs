mod compiler;
mod scanner;

fn main() {
    let filename = std::env::args().nth(1).expect("missing filename");
    let source = std::fs::read_to_string(&filename).expect("read failed");
    if let Err(error) = compiler::compile(&source) {
        let line = source.lines().nth(error.line - 1).unwrap();
        println!(
            "{}:{}:{}: {}.\n\t{}\n\t{}^",
            filename,
            error.line,
            error.column,
            error.message,
            line,
            " ".repeat(error.column - 1)
        );
    }
}
