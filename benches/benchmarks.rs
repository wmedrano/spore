use criterion::{black_box, criterion_group, criterion_main, Criterion};
use spore::parser::ast::Ast;
use spore::vm::compiler::Compiler;
use spore::vm::types::proc::ByteCodeProc;
use std::sync::Arc;

pub fn criterion_benchmark(c: &mut Criterion) {
    let expr = r#"
(def fib (lambda (n)
  (if ( <= n 2 )
      1
      (+ (fib (- n 1)) (fib (- n 2))))))
(fib 20)
(+ 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20)
"#;
    let bytecode: Vec<_> = Ast::from_sexp_str(expr)
        .unwrap()
        .into_iter()
        .map(|ast| -> ByteCodeProc { Compiler::new().compile_and_finalize(&ast).unwrap() })
        .map(Arc::new)
        .collect();
    let mut env = spore::vm::Vm::singleton().env();
    env.eval_bytecode(bytecode[0].clone()).unwrap();
    c.bench_function("fib 20", |b| {
        b.iter(|| env.eval_bytecode(black_box(bytecode[1].clone())).unwrap())
    })
    .bench_function("add 20 elements", |b| {
        b.iter(|| env.eval_bytecode(black_box(bytecode[2].clone())).unwrap())
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
