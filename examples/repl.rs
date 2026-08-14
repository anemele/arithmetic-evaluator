use std::io::{self, BufRead, Write, stdout};

use arithmetic_evaluator::eval_expr;

#[inline]
fn calc(expr: &str) {
    match eval_expr(expr) {
        Ok(res) => println!("{res}"),
        Err(e) => eprintln!("{e}"),
    }
}

fn main() {
    println!("arithmetic_evaluator");
    let stdin = io::stdin();
    let mut buf = String::new();
    loop {
        print!("> ");
        if stdout().flush().is_err() {
            break;
        }
        buf.clear();
        if stdin.lock().read_line(&mut buf).is_err() {
            break;
        }
        let s = buf.trim();
        if s.is_empty() {
            continue;
        }
        calc(s);
    }
}
