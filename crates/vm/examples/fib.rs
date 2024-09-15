fn main() {
    let mut vm = spore_vm::Vm::new();
    vm.eval_str("(define (fib n) (if (< n 2) n (+ (fib (+ n -1)) (fib (+ n -2)))))")
        .unwrap();
    let ret = vm.eval_str("(fib 35)").unwrap();
    println!("{}", ret);
}
