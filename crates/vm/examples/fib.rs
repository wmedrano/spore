use spore_vm::VmSettings;

fn main() {
    let mut vm = spore_vm::Vm::new(VmSettings {
        enable_optimizations: true,
    });
    vm.eval_str("(define (fib n) (if (< n 2) n (+ (fib (+ n -1)) (fib (+ n -2)))))")
        .unwrap();
    let start_time = std::time::Instant::now();
    let ret = vm.eval_str("(fib 35)").unwrap();
    println!("Evaluted in {:?}", start_time.elapsed());
    println!("{}", ret);
}
