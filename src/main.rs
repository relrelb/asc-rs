use std::borrow::Cow;

mod compiler;
mod scanner;

fn usage() {
    let program = std::env::args()
        .nth(0)
        .map_or(Cow::Borrowed("asc"), Cow::Owned);
    println!("Usage: {} <file.as>", program);
}

fn main() {
    let filename = match std::env::args().nth(1) {
        Some(filename) => filename,
        None => {
            usage();
            return;
        }
    };

    let source = match std::fs::read_to_string(&filename) {
        Ok(source) => source,
        Err(error) => {
            println!("Cannot read {}: {}", filename, error);
            return;
        }
    };

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
