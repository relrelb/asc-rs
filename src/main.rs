mod compiler;
mod scanner;

fn main() {
    let filename = std::env::args().nth(1).expect("missing filename");
    let source = std::fs::read_to_string(filename).expect("read failed");
    let mut compiler = compiler::Compiler::new(&source);
    if let Err(error) = compiler.compile() {
        println!("{:?}", error);
    }
}
