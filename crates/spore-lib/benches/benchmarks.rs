use std::rc::Rc;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use spore_lib::parser::ast::Ast;
use spore_lib::vm::ir::{CodeBlock, CodeBlockArgs};
use spore_lib::vm::module::ModuleSource;
use spore_lib::vm::types::instruction::Instruction;
use spore_lib::vm::types::proc::bytecode::ByteCodeIter;
use spore_lib::vm::Vm;

const FIB_SRC: &str = "(define (fib n) (if (<= n 2) 1 (+ (fib (- n 1)) (fib (- n 2)))))";
const MODULE: ModuleSource = ModuleSource::Virtual("bench");

pub fn eval_benchmarks(c: &mut Criterion) {
    let mut vm = Vm::new();
    c.bench_function("eval_fib_20", |b| {
        vm.eval_str(MODULE, FIB_SRC).unwrap();
        let ast: &Ast = &Ast::from_sexp_str("(fib 20)").unwrap()[0];
        let ir = CodeBlock::with_ast(CodeBlockArgs::default(), std::iter::once(ast)).unwrap();
        let bytecode = Rc::new(ir.to_proc(MODULE).unwrap());
        b.iter(|| {
            vm.eval_bytecode(black_box(bytecode.clone()), &[], &mut ())
                .unwrap()
        })
    })
    .bench_function("eval_add_20_elements", |b| {
        let ast =
            Ast::from_sexp_str("(+ 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20)").unwrap();
        let ir = CodeBlock::with_ast(CodeBlockArgs::default(), ast.iter()).unwrap();
        let bytecode = Rc::new(ir.to_proc(MODULE).unwrap());
        b.iter(|| {
            vm.eval_bytecode(black_box(bytecode.clone()), &[], &mut ())
                .unwrap()
        })
    });
}

pub fn eval_microbenchmarks(c: &mut Criterion) {
    let ast = Ast::from_sexp_str(FIB_SRC).unwrap();
    let ir = CodeBlock::with_ast(CodeBlockArgs::default(), ast.iter()).unwrap();
    let proc = Rc::new(ir.to_proc(MODULE).unwrap());
    c.bench_function("iter_bytecode", |b| {
        let iter = ByteCodeIter::from_proc(proc.clone());
        b.iter(|| {
            let mut iter = black_box(iter.clone());
            let mut count = 0;
            while iter.next_instruction() != &Instruction::Return {
                count += 1;
            }
            count
        })
    });
    c.bench_function("iter_bytecode_and_clone", |b| {
        let iter = ByteCodeIter::from_proc(proc.clone());
        let len = proc.bytecode.len();
        b.iter(|| {
            let mut ret = Vec::with_capacity(len);
            let mut iter = black_box(iter.clone());
            loop {
                let instruction = iter.next_instruction();
                if instruction == &Instruction::Return {
                    return ret;
                }
                ret.push(instruction.clone());
            }
        })
    });
}

pub fn compile_benchmarks(c: &mut Criterion) {
    c.bench_function("init_vm", |b| b.iter(Vm::new))
        .bench_function("ast_fib", |b| {
            b.iter(|| Ast::from_sexp_str(FIB_SRC).unwrap())
        })
        .bench_function("compile_fib", |b| {
            let fib_ast = black_box(Ast::from_sexp_str(FIB_SRC).unwrap());
            b.iter(|| {
                let ir = CodeBlock::with_ast(CodeBlockArgs::default(), fib_ast.iter()).unwrap();
                ir.to_proc(MODULE).unwrap()
            })
        });
}

criterion_group!(
    benches,
    eval_benchmarks,
    eval_microbenchmarks,
    compile_benchmarks
);
criterion_main!(benches);
