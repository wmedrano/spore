use spore_vm::Settings;

fn main() {
    let n = 35;

    let mut vm = spore_vm::Vm::new(Settings {
        enable_aggressive_inline: true,
        enable_source_maps: false,
    });
    vm.eval_str("(define (fib n) (if (< n 2) n (+ (fib (+ n -1)) (fib (+ n -2)))))")
        .unwrap();
    let start_time = std::time::Instant::now();
    let ret = vm
        .eval_function_by_name("fib", std::iter::once(n.into()))
        .unwrap();
    println!("Evaluted in {:?}", start_time.elapsed());
    println!("{}", ret);
}
