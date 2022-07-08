mod compiler;
mod scanner;

fn usage() {
    let program = std::env::args()
        .next()
        .map_or("asc".into(), std::borrow::Cow::Owned);
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

    let file = std::fs::File::create("test.swf").unwrap();
    let writer = std::io::BufWriter::new(file);
    if let Err(error) = compiler::compile(&source, writer) {
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
