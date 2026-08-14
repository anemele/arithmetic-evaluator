use nom::{
    IResult, Parser,
    branch::alt,
    character::complete::{char, digit0, digit1, multispace0},
    combinator::{map, map_res},
    multi::many0,
    sequence::delimited,
};
use rust_decimal::Decimal;
use rust_decimal::Error as DecimalError;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Token {
    Add,
    Sub,
    Mul,
    Div,
    LP,
    RP,
    Number(Decimal),
}

#[derive(PartialEq, PartialOrd)]
pub enum Precedence {
    N,
    PM,
    MD,
}

pub fn precedence(op: &Token) -> Precedence {
    match op {
        Token::Add | Token::Sub => Precedence::PM,
        Token::Mul | Token::Div => Precedence::MD,
        _ => Precedence::N,
    }
}

pub type Tokens = Vec<Token>;

fn tokenize_number(input: &str) -> IResult<&str, Token> {
    type DecimalResult = Result<Token, DecimalError>;

    alt((
        map_res(
            (digit1, char('.'), digit0),
            |(int, dot, frac)| -> DecimalResult {
                let d = Decimal::from_str_exact(&format!("{int}{dot}{frac}"))?;
                Ok(Token::Number(d))
            },
        ),
        map_res((char('.'), digit1), |(dot, frac)| -> DecimalResult {
            let d = Decimal::from_str_exact(&format!("0{dot}{frac}"))?;
            Ok(Token::Number(d))
        }),
        map_res(digit1, |int| -> DecimalResult {
            let d = Decimal::from_str_exact(int)?;
            Ok(Token::Number(d))
        }),
    ))
    .parse(input)
}

fn tokenize_char(input: &str) -> IResult<&str, Token> {
    map(
        alt((
            char('+'),
            char('-'),
            char('*'),
            char('/'),
            char('('),
            char(')'),
        )),
        |c| -> Token {
            use Token::*;
            match c {
                '+' => Add,
                '-' => Sub,
                '*' => Mul,
                '/' => Div,
                '(' => LP,
                ')' => RP,
                _ => unreachable!(),
            }
        },
    )
    .parse(input)
}

#[derive(Debug, thiserror:: Error)]
pub enum TokenError {
    #[error("has tail: {0}")]
    HasTail(String),
    #[error("parse error")]
    ParseError,
}

pub fn tokenize(input: &str) -> Result<Tokens, TokenError> {
    match many0(alt((
        delimited(multispace0, tokenize_char, multispace0),
        delimited(multispace0, tokenize_number, multispace0),
    )))
    .parse(input)
    {
        Ok((s, ts)) => {
            if !s.is_empty() {
                return Err(TokenError::HasTail(s.to_string()));
            }
            Ok(ts)
        }
        Err(_) => Err(TokenError::ParseError),
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;

    #[inline]
    fn number(num: &str) -> Token {
        Token::Number(Decimal::from_str_exact(num).unwrap())
    }

    #[test]
    fn test_tokenize_number() {
        assert!(tokenize_number("").is_err());
        assert!(tokenize_number(".").is_err());
        assert_eq!(tokenize_number("1").unwrap().1, number("1"));
        assert_eq!(tokenize_number("1.").unwrap().1, number("1"));
        assert_eq!(tokenize_number(".1").unwrap().1, number("0.1"));
        assert_eq!(tokenize_number("1.23").unwrap().1, number("1.23"));
        assert_eq!(tokenize_number("1.2.3").unwrap().1, number("1.2"));
    }

    #[test]
    fn test_tokenize() {
        use super::Token::*;

        assert_eq!(tokenize("").unwrap(), Vec::new());
        assert_eq!(
            tokenize(" 2+  5 - - 4/4 *").unwrap(),
            vec![
                number("2"),
                Add,
                number("5"),
                Sub,
                Sub,
                number("4"),
                Div,
                number("4"),
                Mul,
            ]
        );

        assert_matches!(
            tokenize("$:;"),
            Err(TokenError::HasTail(s))
            if s == "$:;"
        );
        assert_matches!(
            tokenize("."),
            Err(TokenError::HasTail(s))
            if s == "."
        );
        assert_matches!(
            tokenize("1.1."),
            Err(TokenError::HasTail(s))
            if s == "."
        );

        assert_eq!(
            tokenize("1+1.1*2/3()1.1.1-.1.1").unwrap(),
            vec![
                number("1"),
                Add,
                number("1.1"),
                Mul,
                number("2"),
                Div,
                number("3"),
                LP,
                RP,
                number("1.1"),
                number("0.1"),
                Sub,
                number("0.1"),
                number("0.1"),
            ]
        );
    }
}
