use asc::CompileError;

fn usage() {
    let program = std::env::args()
        .next()
        .map_or("asc".into(), std::borrow::Cow::Owned);
    println!("Usage: {} <file.as>", program);
}

fn main() -> Result<(), CompileError> {
    let Some(filename) = std::env::args().nth(1) else {
        usage();
        return Ok(());
    };

    let source = std::fs::read_to_string(&filename).map_err(|error| CompileError {
        message: format!("Cannot read {}: {}", filename, error),
        line: 0,
        column: 0,
    })?;

    let file = std::fs::File::create("test.swf").unwrap();
    let writer = std::io::BufWriter::new(file);
    let result = asc::compile(&source, writer);
    if let Err(error) = &result {
        let line = source.lines().nth(error.line - 1).unwrap();
        println!(
            "{}:{}:{}: {}:\n\t{}\n\t{}^",
            filename,
            error.line,
            error.column,
            error.message,
            line,
            " ".repeat(error.column - 1)
        );
    }
    result
}
