use spore_vm::Settings;

fn main() {
    let n = 35;

    let mut vm = spore_vm::Vm::new(Settings {
        enable_aggressive_inline: true,
        enable_source_maps: false,
    });
    let src = r#"
(define (fib n)
  (if (< n 2) (return n))
  (+ (fib (+ n -1)) (fib (+ n -2))))
"#;
    vm.eval_str(src).unwrap();
    let start_time = std::time::Instant::now();
    let ret = vm
        .eval_function_by_name("fib", std::iter::once(n.into()))
        .unwrap();
    println!("Evaluted in {:?}", start_time.elapsed());
    println!("{}", ret);
}
