use spore;

fn main() {
    let mut repl = spore::repl::Repl::new().unwrap();
    repl.run().unwrap();
}
