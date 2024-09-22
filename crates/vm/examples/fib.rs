use spore_vm::{DefaultDebugger, Settings};

fn main() {
    let n = 35;

    let mut vm = spore_vm::Vm::new(Settings {
        enable_aggressive_inline: true,
    });
    vm.eval_str(
        "(define (fib n) (if (< n 2) n (+ (fib (+ n -1)) (fib (+ n -2)))))",
        &mut DefaultDebugger,
    )
    .unwrap();
    let start_time = std::time::Instant::now();
    let ret = vm
        .eval_function_by_name("fib", std::iter::once(n.into()), &mut DefaultDebugger)
        .unwrap();
    println!("Evaluted in {:?}", start_time.elapsed());
    println!("{}", ret);
}
