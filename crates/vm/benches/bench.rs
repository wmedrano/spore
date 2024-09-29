use criterion::{criterion_group, criterion_main, Criterion};

fn fib_benchmark(c: &mut Criterion) {
    let n = 30;
    c.bench_function(&format!("fib-{n}"), |b| {
        let mut vm = spore_vm::Vm::new(spore_vm::Settings {
            enable_aggressive_inline: true,
            ..Default::default()
        });
        vm.eval_str("(define (fib n) (if (< n 2) n (+ (fib (+ n -1)) (fib (+ n -2)))))")
            .unwrap();
        b.iter(move || {
            vm.eval_function_by_name("fib", std::iter::once(n.into()))
                .unwrap()
                .try_int()
                .unwrap()
        })
    });
}

fn arithmetic_benchmark(c: &mut Criterion) {
    c.bench_function("arithmetic", |b| {
        let mut vm = spore_vm::Vm::new(spore_vm::Settings {
            enable_aggressive_inline: true,
            ..Default::default()
        });
        let src = r#"
(define (benchmark)
  (let ([x (+ 10 20 30)]
        [y (- x 4 5 8)]
        [z (- x y x y x y x y)])
    (+ x y z)))
"#;
        vm.eval_str(src).unwrap();
        b.iter(move || {
            vm.eval_function_by_name("benchmark", std::iter::empty())
                .unwrap()
                .try_int()
                .unwrap()
        })
    });
}

fn struct_benchmark(c: &mut Criterion) {
    c.bench_function("struct", |b| {
        let mut vm = spore_vm::Vm::new(spore_vm::Settings {
            enable_aggressive_inline: true,
            ..Default::default()
        });
        let src = r#"
(define my-struct (struct "x" 0 "y" 10 "width" 20 "height" 30))
(struct-set! my-struct "right" (+ (struct-get my-struct "x") (struct-get my-struct "width")))
(struct-set! my-struct "bottom" (+ (struct-get my-struct "y") (struct-get my-struct "height")))
(struct-set! my-struct "self" my-struct)
(+ (struct-get my-struct "x")
   (struct-get my-struct "y")
   (struct-get my-struct "width")
   (struct-get my-struct "height"))
"#;
        b.iter(move || vm.eval_str(src).unwrap().try_int().unwrap())
    });
}

criterion_group!(
    benches,
    fib_benchmark,
    arithmetic_benchmark,
    struct_benchmark
);
criterion_main!(benches);
