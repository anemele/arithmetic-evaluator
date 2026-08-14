use crate::token::{Token, TokenError, Tokens, precedence, tokenize};
use rust_decimal::Decimal;

#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
    #[error("empty expression")]
    EmptyExpr,

    #[error("unmatched left parentheses")]
    UnmatchedLeftParen,
    #[error("unmatched right parentheses")]
    UnmatchedRightParen,
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

fn check_syntax(ts: &Tokens) -> Result<(), SyntaxError> {
    enum LastKind {
        Start, // Minus, Num, LP
        Op,    // Num, LP
        Num,   // Op, RP
        LP,    // Num, LP
        RP,    // Op, RP
    }
    let mut last_kind = LastKind::Start;

    use Token::*;

    for t in ts {
        match last_kind {
            LastKind::Num => match t {
                Number(_) => return Err(SyntaxError::ConsecutiveNumber),
                LP => return Err(SyntaxError::NumberLP),
                RP => last_kind = LastKind::RP,
                Plus | Minus | Mul | Div => last_kind = LastKind::Op,
            },
            LastKind::Op => match t {
                Number(_) => last_kind = LastKind::Num,
                LP => last_kind = LastKind::LP,
                RP => return Err(SyntaxError::OperatorRP),
                Plus | Minus | Mul | Div => return Err(SyntaxError::ConsecutiveOperator),
            },
            LastKind::LP => match t {
                Number(_) => last_kind = LastKind::Num,
                LP => last_kind = LastKind::LP,
                RP => return Err(SyntaxError::EmptyParen),
                Plus | Minus | Mul | Div => return Err(SyntaxError::LPOperator),
            },
            LastKind::RP => match t {
                Number(_) => return Err(SyntaxError::RPNumber),
                LP => return Err(SyntaxError::RPWithLP),
                RP => last_kind = LastKind::RP,
                Plus | Minus | Mul | Div => last_kind = LastKind::Op,
            },
            LastKind::Start => match t {
                Number(_) => last_kind = LastKind::Num,
                LP => last_kind = LastKind::LP,
                RP => return Err(SyntaxError::UnmatchedRightParen),
                Minus => last_kind = LastKind::Op,
                Plus | Mul | Div => return Err(SyntaxError::StartWithOperator),
            },
        }
    }

    match last_kind {
        LastKind::Num | LastKind::RP => Ok(()),
        LastKind::Op => Err(SyntaxError::EndWithOperator),
        LastKind::LP => Err(SyntaxError::UnmatchedLeftParen),
        LastKind::Start => Err(SyntaxError::EmptyExpr),
    }
}

fn infix_to_postfix(ts: &Tokens) -> Result<Tokens, SyntaxError> {
    use Token::*;

    let mut ret = Vec::<Token>::new();
    if let Some(t) = ts.first()
        && matches!(t, Minus)
    {
        ret.push(Number(Decimal::ZERO));
    }
    let mut op_stack = Vec::<Token>::new();

    for token in ts {
        match token {
            Number(_) => ret.push(*token),
            LP => op_stack.push(*token),
            RP => loop {
                let top = op_stack.pop().ok_or(SyntaxError::UnmatchedRightParen)?;
                if matches!(top, LP) {
                    break;
                }
                ret.push(top);
            },
            op @ (Plus | Minus | Mul | Div) => {
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
            return Err(SyntaxError::UnmatchedLeftParen);
        }
        ret.push(top);
    }

    Ok(ret)
}

#[derive(Debug, thiserror::Error)]
pub enum CalcError {
    #[error("divided by zero")]
    DivByZero,
    #[error("bad rpn")]
    BadRpn,
}

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("token error: {0}")]
    Token(#[from] TokenError),
    #[error("syntax error: {0}")]
    Syntax(#[from] SyntaxError),
    #[error("calc error: {0}")]
    Calc(#[from] CalcError),
}

pub fn eval_expr(input: &str) -> Result<Decimal, EvalError> {
    let ts = match tokenize(input) {
        Ok(ts) => ts,
        Err(e) => return Err(EvalError::Token(e)),
    };

    check_syntax(&ts)?;
    let ts = infix_to_postfix(&ts)?;

    use Token::*;

    let mut stack = Vec::<Decimal>::new();
    for token in ts {
        match token {
            Number(num) => stack.push(num),
            Plus => {
                let rhs = stack.pop().ok_or(EvalError::Calc(CalcError::BadRpn))?;
                let lhs = stack.pop().ok_or(EvalError::Calc(CalcError::BadRpn))?;
                stack.push(lhs + rhs);
            }
            Minus => {
                let rhs = stack.pop().ok_or(EvalError::Calc(CalcError::BadRpn))?;
                let lhs = stack.pop().ok_or(EvalError::Calc(CalcError::BadRpn))?;
                stack.push(lhs - rhs);
            }
            Mul => {
                let rhs = stack.pop().ok_or(EvalError::Calc(CalcError::BadRpn))?;
                let lhs = stack.pop().ok_or(EvalError::Calc(CalcError::BadRpn))?;
                stack.push(lhs * rhs);
            }
            Div => {
                let rhs = stack.pop().ok_or(EvalError::Calc(CalcError::BadRpn))?;
                let lhs = stack.pop().ok_or(EvalError::Calc(CalcError::BadRpn))?;
                if rhs == Decimal::ZERO {
                    return Err(EvalError::Calc(CalcError::DivByZero));
                }
                stack.push(lhs / rhs);
            }
            LP | RP => unreachable!(),
        }
    }

    if stack.len() != 1 {
        return Err(EvalError::Calc(CalcError::BadRpn));
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
        assert!(infix_to_postfix(&tokenize("((1+3)*5").unwrap()).is_err());

        assert_eq!(
            infix_to_postfix(&tokenize("1+2").unwrap()).unwrap(),
            vec![token_number("1"), token_number("2"), Plus]
        );
        assert_eq!(
            infix_to_postfix(&tokenize("-1+2").unwrap()).unwrap(),
            vec![
                token_number("0"),
                token_number("1"),
                Minus,
                token_number("2"),
                Plus
            ]
        );
        assert_eq!(
            infix_to_postfix(&tokenize("1+2*3-4/5").unwrap()).unwrap(),
            vec![
                token_number("1"),
                token_number("2"),
                token_number("3"),
                Mul,
                Plus,
                token_number("4"),
                token_number("5"),
                Div,
                Minus,
            ]
        );
    }

    #[test]
    fn test_eval_expr() {
        assert_matches!(
            eval_expr(""),
            Err(EvalError::Syntax(SyntaxError::EmptyExpr))
        );

        assert_matches!(eval_expr("1/0"), Err(EvalError::Calc(CalcError::DivByZero)));

        assert_eq!(eval_expr("1+1").unwrap(), number("2"));

        assert_eq!(eval_expr("1+2*3-4/5").unwrap(), number("6.2"));
        assert_eq!(eval_expr("(1+2)*3-4/5").unwrap(), number("8.2"));
        assert_eq!(eval_expr("(1+2)*(3)-4/5").unwrap(), number("8.2"));
        assert_matches!(
            eval_expr("(1+2)*()3-4/5"),
            Err(EvalError::Syntax(SyntaxError::EmptyParen))
        );

        assert_eq!(eval_expr("(1+2)*(3-4)/5").unwrap(), number("-0.6"));
    }
}
