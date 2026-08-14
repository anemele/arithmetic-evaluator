use crate::token::{Token, TokenError, Tokens, precedence, tokenize};
use rust_decimal::Decimal;

#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
    #[error("empty expression")]
    EmptyExpr,

    #[error("unmatched left parentheses")]
    UnmatchedLP,
    #[error("unmatched right parentheses")]
    UnmatchedRP,
    #[error("empty parentheses")]
    EmptyParen,

    #[error("consecutive operator")]
    ConsecutiveOperator,
    #[error("consecutive number")]
    ConsecutiveNumber,

    #[error("end with operator")]
    EndWithOperator,
    #[error("start with operator")]
    StartWithOperator,

    #[error("(*")]
    LPOperator,
    #[error("*)")]
    OperatorRP,

    #[error("1(")]
    NumberLP,
    #[error(")1")]
    RPNumber,

    #[error(")(")]
    RPWithLP,
}

fn normalize(ts: &Tokens) -> Result<Tokens, SyntaxError> {
    enum LastKind {
        Start, // Minus, Num, LP
        Op,    // Num, LP
        Num,   // Op, RP
        LP,    // Num, LP
        RP,    // Op, RP
    }
    let mut last_kind = LastKind::Start;
    let mut ret = Vec::<Token>::new();

    use Token::*;

    for t in ts {
        match last_kind {
            LastKind::Num => match t {
                Number(_) => return Err(SyntaxError::ConsecutiveNumber),
                LP => return Err(SyntaxError::NumberLP),
                RP => last_kind = LastKind::RP,
                Add | Sub | Mul | Div => last_kind = LastKind::Op,
            },
            LastKind::Op => match t {
                Number(_) => last_kind = LastKind::Num,
                LP => last_kind = LastKind::LP,
                RP => return Err(SyntaxError::OperatorRP),
                Add | Sub | Mul | Div => return Err(SyntaxError::ConsecutiveOperator),
            },
            LastKind::LP => match t {
                Number(_) => last_kind = LastKind::Num,
                LP => last_kind = LastKind::LP,
                RP => return Err(SyntaxError::EmptyParen),
                Sub => {
                    last_kind = LastKind::Op;
                    ret.push(Number(Decimal::ZERO));
                }
                Add | Mul | Div => return Err(SyntaxError::LPOperator),
            },
            LastKind::RP => match t {
                Number(_) => return Err(SyntaxError::RPNumber),
                LP => return Err(SyntaxError::RPWithLP),
                RP => last_kind = LastKind::RP,
                Add | Sub | Mul | Div => last_kind = LastKind::Op,
            },
            LastKind::Start => match t {
                Number(_) => last_kind = LastKind::Num,
                LP => last_kind = LastKind::LP,
                RP => return Err(SyntaxError::UnmatchedRP),
                Sub => {
                    last_kind = LastKind::Op;
                    ret.push(Number(Decimal::ZERO));
                }
                Add | Mul | Div => return Err(SyntaxError::StartWithOperator),
            },
        }
        ret.push(*t);
    }

    match last_kind {
        LastKind::Num | LastKind::RP => Ok(ret),
        LastKind::Op => Err(SyntaxError::EndWithOperator),
        LastKind::LP => Err(SyntaxError::UnmatchedLP),
        LastKind::Start => Err(SyntaxError::EmptyExpr),
    }
}

fn infix_to_postfix(ts: &Tokens) -> Result<Tokens, SyntaxError> {
    use Token::*;

    let mut ret = Vec::<Token>::new();
    let mut op_stack = Vec::<Token>::new();

    for token in ts {
        match token {
            Number(_) => ret.push(*token),
            LP => op_stack.push(*token),
            RP => loop {
                let top = op_stack.pop().ok_or(SyntaxError::UnmatchedRP)?;
                if matches!(top, LP) {
                    break;
                }
                ret.push(top);
            },
            op @ (Add | Sub | Mul | Div) => {
                while let Some(top) = op_stack.last() {
                    if matches!(top, LP) || precedence(top) < precedence(op) {
                        break;
                    }
                    ret.push(op_stack.pop().unwrap());
                }
                op_stack.push(*op);
            }
        }
    }

    while let Some(top) = op_stack.pop() {
        if matches!(top, LP) {
            return Err(SyntaxError::UnmatchedLP);
        }
        ret.push(top);
    }

    Ok(ret)
}

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("overflow")]
    Overflow,
    #[error("divided by zero")]
    DivByZero,
    #[error("bad rpn")]
    BadRpn,
}

#[derive(Debug, thiserror::Error)]
pub enum CalcError {
    #[error("token error: {0}")]
    Token(#[from] TokenError),
    #[error("syntax error: {0}")]
    Syntax(#[from] SyntaxError),
    #[error("calc error: {0}")]
    Eval(#[from] EvalError),
}

pub fn eval_expr(input: &str) -> Result<Decimal, CalcError> {
    let ts = tokenize(input)?;
    let ts = normalize(&ts)?;
    let ts = infix_to_postfix(&ts)?;

    use Token::*;

    let mut stack = Vec::<Decimal>::new();
    for token in ts {
        match token {
            Number(num) => stack.push(num),
            Add => {
                let rhs = stack.pop().ok_or(CalcError::Eval(EvalError::BadRpn))?;
                let lhs = stack.pop().ok_or(CalcError::Eval(EvalError::BadRpn))?;
                let res = lhs
                    .checked_add(rhs)
                    .ok_or(CalcError::Eval(EvalError::Overflow))?;
                stack.push(res);
            }
            Sub => {
                let rhs = stack.pop().ok_or(CalcError::Eval(EvalError::BadRpn))?;
                let lhs = stack.pop().ok_or(CalcError::Eval(EvalError::BadRpn))?;
                let res = lhs
                    .checked_sub(rhs)
                    .ok_or(CalcError::Eval(EvalError::Overflow))?;
                stack.push(res);
            }
            Mul => {
                let rhs = stack.pop().ok_or(CalcError::Eval(EvalError::BadRpn))?;
                let lhs = stack.pop().ok_or(CalcError::Eval(EvalError::BadRpn))?;
                let res = lhs
                    .checked_mul(rhs)
                    .ok_or(CalcError::Eval(EvalError::Overflow))?;
                stack.push(res);
            }
            Div => {
                let rhs = stack.pop().ok_or(CalcError::Eval(EvalError::BadRpn))?;
                let lhs = stack.pop().ok_or(CalcError::Eval(EvalError::BadRpn))?;
                if rhs == Decimal::ZERO {
                    return Err(CalcError::Eval(EvalError::DivByZero));
                }
                let res = lhs
                    .checked_div(rhs)
                    .ok_or(CalcError::Eval(EvalError::Overflow))?;
                stack.push(res);
            }
            LP | RP => unreachable!(),
        }
    }

    if stack.len() != 1 {
        return Err(CalcError::Eval(EvalError::BadRpn));
    }

    Ok(stack[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{Token::*, tokenize};
    use std::assert_matches;

    #[inline]
    fn number(num: &str) -> Decimal {
        Decimal::from_str_exact(num).unwrap()
    }
    #[inline]
    fn token_number(num: &str) -> Token {
        Number(number(num))
    }

    #[test]
    fn test_infix_to_postfix() {
        #[inline]
        fn norm(s: &str) -> Result<Tokens, SyntaxError> {
            infix_to_postfix(&normalize(&tokenize(s).unwrap()).unwrap())
        }

        assert_matches!(norm("((1+3)*5"), Err(SyntaxError::UnmatchedLP));

        assert_eq!(
            norm("1+2").unwrap(),
            vec![token_number("1"), token_number("2"), Add]
        );
        assert_eq!(
            norm("-1+2").unwrap(),
            vec![
                token_number("0"),
                token_number("1"),
                Sub,
                token_number("2"),
                Add
            ]
        );
        assert_eq!(
            norm("1+2*3-4/5").unwrap(),
            vec![
                token_number("1"),
                token_number("2"),
                token_number("3"),
                Mul,
                Add,
                token_number("4"),
                token_number("5"),
                Div,
                Sub,
            ]
        );
    }

    #[test]
    fn test_eval_expr() {
        assert_matches!(
            eval_expr(""),
            Err(CalcError::Syntax(SyntaxError::EmptyExpr))
        );

        assert_matches!(eval_expr("1/0"), Err(CalcError::Eval(EvalError::DivByZero)));

        assert_eq!(eval_expr("1+1").unwrap(), number("2"));

        assert_eq!(eval_expr("1+2*3-4/5").unwrap(), number("6.2"));
        assert_eq!(eval_expr("(1+2)*3-4/5").unwrap(), number("8.2"));
        assert_eq!(eval_expr("(1+2)*(3)-4/5").unwrap(), number("8.2"));
        assert_matches!(
            eval_expr("(1+2)*()3-4/5"),
            Err(CalcError::Syntax(SyntaxError::EmptyParen))
        );

        assert_eq!(eval_expr("(1+2)*(3-4)/5").unwrap(), number("-0.6"));
    }
}
