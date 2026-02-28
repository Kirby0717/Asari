use super::*;

use winnow::{ascii::digit1, stream::Offset};

pub fn bin_int(input: &mut Input) -> ModalResult<i64> {
    trace(
        "bin_int",
        take_while(1.., '0'..='1').try_map_with_span(|s| {
            i64::from_str_radix(s, 2).map_err(NumberError::Bin)
        }),
    )
    .cut()
    .parse_next(input)
}
pub fn oct_int(input: &mut Input) -> ModalResult<i64> {
    trace(
        "oct_int",
        take_while(1.., '0'..='7').try_map_with_span(|s| {
            i64::from_str_radix(s, 8).map_err(NumberError::Oct)
        }),
    )
    .cut()
    .parse_next(input)
}
pub fn hex_int(input: &mut Input) -> ModalResult<i64> {
    trace(
        "hex_int",
        take_while(1.., ('0'..='9', 'A'..='F', 'a'..='f')).try_map_with_span(
            |s| i64::from_str_radix(s, 16).map_err(NumberError::Hex),
        ),
    )
    .cut()
    .parse_next(input)
}
pub fn dec_number(input: &mut Input) -> ModalResult<Primary> {
    let start = input.checkpoint();
    let _int_part = digit1.parse_next(input)?;
    let int_checkpoint = input.checkpoint();
    let decimal_part = opt(preceded('.', digit1)).parse_next(input)?;
    let exp_part =
        opt(preceded(alt(('e', 'E')), (opt(alt(('+', '-'))), digit1)))
            .parse_next(input)?;
    let float_checkpoint = input.checkpoint();

    input.reset(&start);
    if decimal_part.is_some() || exp_part.is_some() {
        let l = float_checkpoint.offset_from(&start);
        let float = trace("float", take(l))
            .try_map_with_span(|s| s.parse::<f64>().map_err(NumberError::Float))
            .cut()
            .parse_next(input)?;
        Ok(Primary::Float(float))
    }
    else {
        let l = int_checkpoint.offset_from(&start);
        let int = trace("dec_int", take(l))
            .try_map_with_span(|s| s.parse::<i64>().map_err(NumberError::Dec))
            .cut()
            .parse_next(input)?;
        Ok(Primary::Int(int))
    }
}
pub fn number(input: &mut Input) -> ModalResult<Primary> {
    trace(
        "number",
        dispatch! {peek(opt(take(2_usize)));
            Some("0b") => preceded("0b", bin_int).map(Primary::Int),
            Some("0o") => preceded("0o", oct_int).map(Primary::Int),
            Some("0x") => preceded("0x", hex_int).map(Primary::Int),
            _ => dec_number,
        },
    )
    .parse_next(input)
}

pub fn expr_primary(input: &mut Input) -> ModalResult<Primary> {
    trace(
        "expr_primary",
        dispatch! {peek(any);
            '\'' => quoted_string.map(Primary::String),
            '"' => double_quoted_string.map(Primary::String),
            '$' => alt((
                subst.spanned().map(|subst| Primary::CommandSubst(Box::new(subst))),
                special_var.map(Primary::SpecialVar),
                ident.map(Primary::EnvVar),
            )).cut(),
            '@' => shell_var.cut().map(Primary::ShellVar),
            '(' => alt((
                unit.value(Primary::Unit),
                paren.spanned().map(|expr| Primary::Paren(Box::new(expr)))
            )).cut(),
            'r' => raw_string.map(Primary::String),
            'p' => path_string.map(Primary::PathString),
            '[' => array.cut().map(Primary::Array),
            't' => keyword("true").value(Primary::Bool(true)),
            'f' => keyword("false").value(Primary::Bool(false)),
            '0'..='9' => number,
            'n' => keyword("none").value(Primary::Option(None)),
            's' => preceded(
                keyword("some"),
                paren.spanned().map(|expr| Primary::Option(Some(Box::new(expr)))),
            ).cut(),
            _ => fail,
        },
    )
    .parse_next(input)
}
pub fn expr_prefix(input: &mut Input) -> ModalResult<ExprPrefix> {
    use ExprPrefix::*;
    trace(
        "expr_prefix",
        dispatch! {any;
            '!' => empty.value(Not),
            '-' => empty.value(Neg),
            _ => fail,
        },
    )
    .parse_next(input)
}
pub fn expr_infix(input: &mut Input) -> ModalResult<ExprInfix> {
    use ExprInfix::*;
    trace(
        "expr_infix",
        dispatch! {any;
            '^' => empty.value(UnwrapOr),
            '+' => empty.value(Add),
            '-' => empty.value(Sub),
            '*' => empty.value(Mul),
            '/' => empty.value(Div),
            '%' => empty.value(Rem),
            '=' => '='.value(Equal),
            '!' => '='.value(NotEqual),
            '<' => opt('='.value(LessEqual))
                    .map(|c| c.unwrap_or(Less)),
            '>' => opt('='.value(GreaterEqual))
                    .map(|c| c.unwrap_or(Greater)),
            '&' => '&'.value(And),
            '|' => '|'.value(Or),
            _ => fail,
        },
    )
    .parse_next(input)
}
pub fn expr_postfix(input: &mut Input) -> ModalResult<ExprPostfix> {
    use ExprPostfix::*;
    trace(
        "expr_prefix",
        dispatch! {any;
            '!' => empty.value(Unwrap),
            '?' => empty.value(IsSome),
            '@' => empty.value(Length),
            '[' => delimited(space0, expr.spanned(), (space0, ']'))
                .map(|index| Index(Box::new(index))),
            'a' => preceded(('s', space0), ast_type.spanned())
                .map(Cast),
            _ => fail,
        },
    )
    .parse_next(input)
}
pub fn ast_type(input: &mut Input) -> ModalResult<AstType> {
    use AstType::*;
    trace("expr_type", |input: &mut Input| {
        if opt('_').parse_next(input)?.is_some() {
            return Ok(Unknown);
        }
        let name = ident.cut().parse_next(input)?;
        if opt((space0, '<')).parse_next(input)?.is_some() {
            let generics = preceded(space0, ast_type)
                .or_err_with_span(TypeError::NoType)
                .cut()
                .parse_next(input)?;
            let _ = (space0, '>')
                .or_err_with_span(TypeError::UnclosedTypeParam)
                .cut()
                .parse_next(input)?;
            Ok(Generics(name, Box::new(generics)))
        }
        else {
            Ok(Normal(name))
        }
    })
    .parse_next(input)
}
pub fn expr(input: &mut Input) -> ModalResult<Expr> {
    trace("expr", |input: &mut Input| {
        expr_pratt(input, 0).map(|expr| expr.inner)
    })
    .parse_next(input)
}
fn expr_pratt(input: &mut Input, min_power: i32) -> ModalResult<Spanned<Expr>> {
    // 前置演算子 or 値
    let mut lhs =
        if let Some(prefix) = opt(expr_prefix.spanned()).parse_next(input)? {
            let _ = space0.parse_next(input)?;
            let rhs = expr_pratt(input, prefix.inner.power())?;
            prefix.apply(rhs)
        }
        else {
            let primary = expr_primary.spanned().parse_next(input)?;
            // (expr) の括弧を剥がず
            // エラーなどは中身に対して行う
            match primary.inner {
                Primary::Paren(expr) => Spanned {
                    span: primary.span,
                    inner: expr.inner,
                },
                _ => Spanned {
                    span: primary.span.clone(),
                    inner: Expr::Primary(primary),
                },
            }
        };

    loop {
        let checkpoint = input.checkpoint();
        let _ = space0.parse_next(input)?;

        // 中置演算子
        if let Some(infix) = opt(expr_infix.spanned()).parse_next(input)? {
            let (l_power, r_power) = infix.inner.power();
            if l_power < min_power {
                input.reset(&checkpoint);
                break;
            }
            let _ = space0.parse_next(input)?;
            let cursor = input.current_token_start();
            let rhs = expr_pratt(input, r_power).map_err(|_| {
                winnow::error::ErrMode::Cut(ParseError {
                    kind: ParseErrorKind::Expr(ExprError::NoRhs),
                    span: cursor..cursor + 1,
                })
            })?;
            lhs = infix.apply(lhs, rhs);
            continue;
        }

        // 後置演算子
        if let Some(postfix) = opt(expr_postfix.spanned()).parse_next(input)? {
            let power = postfix.inner.power();
            if power < min_power {
                input.reset(&checkpoint);
                break;
            }
            lhs = postfix.apply(lhs);
            continue;
        }

        input.reset(&checkpoint);
        break;
    }

    Ok(lhs)
}
