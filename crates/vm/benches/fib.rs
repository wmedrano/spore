use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let n = 30;
    c.bench_function(&format!("fib-{n}"), |b| {
        let mut vm = spore_vm::Vm::new(spore_vm::Settings {
            enable_aggressive_inline: true,
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

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
