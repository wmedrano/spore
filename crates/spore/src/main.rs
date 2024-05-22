use spore_repl;

fn main() {
    let mut repl = spore_repl::Repl::new().unwrap();
    repl.run().unwrap();
}
