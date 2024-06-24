use spore_repl::Repl;

fn main() {
    let mut repl = Repl::new().unwrap();
    repl.run().unwrap();
}
