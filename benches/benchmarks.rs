use criterion::{black_box, criterion_group, criterion_main, Criterion};
use spore::parser::ast::Ast;
use spore::vm::compiler::Compiler;
use spore::vm::Vm;
use std::sync::Arc;

const FIB_SRC: &str = "(def fib (lambda (n) (if (<= n 2) 1 (+ (fib (- n 1)) (fib (- n 2))))))";

pub fn eval_benchmarks(c: &mut Criterion) {
    let mut env = spore::vm::Vm::singleton().env();
    c.bench_function("eval fib 20", |b| {
        env.eval_str(FIB_SRC).unwrap();
        let bytecode = Arc::new(
            Compiler::new()
                .compile_and_finalize(&Ast::from_sexp_str("(fib 20)").unwrap()[0])
                .unwrap(),
        );
        b.iter(|| env.eval_bytecode(black_box(bytecode.clone())).unwrap())
    })
    .bench_function("eval add 20 elements", |b| {
        let bytecode = Arc::new(
            Compiler::new()
                .compile_and_finalize(
                    &Ast::from_sexp_str("(+ 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20)")
                        .unwrap()[0],
                )
                .unwrap(),
        );
        b.iter(|| env.eval_bytecode(black_box(bytecode.clone())).unwrap())
    });
}

pub fn compile_benchmarks(c: &mut Criterion) {
    let fib_ast = &Ast::from_sexp_str(FIB_SRC).unwrap()[0];
    c.bench_function("init env", |b| {
        let vm = Vm::singleton();
        b.iter(|| vm.env())
    })
    .bench_function("ast fib", |b| {
        b.iter(|| Ast::from_sexp_str(FIB_SRC).unwrap())
    })
    .bench_function("compile fib", |b| {
        b.iter(|| Compiler::new().compile_and_finalize(fib_ast).unwrap())
    });
}

criterion_group!(benches, eval_benchmarks, compile_benchmarks);
criterion_main!(benches);
