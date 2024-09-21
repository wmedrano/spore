use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let src = "(fib 30)";
    c.bench_function(src, |b| {
        let mut vm = spore_vm::Vm::new(spore_vm::VmSettings {
            enable_aggressive_inline: true,
        });
        vm.eval_str("(define (fib n) (if (< n 2) n (+ (fib (+ n -1)) (fib (+ n -2)))))")
            .unwrap();
        b.iter(move || vm.eval_str(src).unwrap().try_int().unwrap())
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
