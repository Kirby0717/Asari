use super::*;

use winnow::ascii::float;
pub fn bin_int(input: &mut Input) -> ModalResult<i64> {
    take_while(1.., '0'..='1')
        .try_map_with_span(|s| {
            i64::from_str_radix(s, 2).map_err(ParseErrorKind::ParseBinError)
        })
        .parse_next(input)
}
pub fn oct_int(input: &mut Input) -> ModalResult<i64> {
    take_while(1.., '0'..='7')
        .try_map_with_span(|s| {
            i64::from_str_radix(s, 8).map_err(ParseErrorKind::ParseOctError)
        })
        .parse_next(input)
}
pub fn dec_int(input: &mut Input) -> ModalResult<i64> {
    take_while(1.., '0'..='9')
        .try_map_with_span(|s| {
            i64::from_str_radix(s, 10).map_err(ParseErrorKind::ParseDecError)
        })
        .parse_next(input)
}
pub fn hex_int(input: &mut Input) -> ModalResult<i64> {
    take_while(1.., ('0'..='9', 'A'..='F', 'a'..='f'))
        .try_map_with_span(|s| {
            i64::from_str_radix(s, 16).map_err(ParseErrorKind::ParseHexError)
        })
        .parse_next(input)
}
pub fn int(input: &mut Input) -> ModalResult<i64> {
    dispatch! {peek(take(2_usize));
        "0b" => preceded("0b", bin_int),
        "0o" => preceded("0o", oct_int),
        "0x" => preceded("0c", hex_int),
        _ => dec_int,
    }
    .parse_next(input)
}
pub fn number(input: &mut Input) -> ModalResult<Primary> {
    alt((int.map(Primary::Int), float.map(Primary::Float))).parse_next(input)
}

pub fn expr_primary(input: &mut Input) -> SpannedResult<Primary> {
    dispatch!(peek(any);
        '\'' => quoted_string.map(Primary::String),
        '"' => double_quoted_string.map(Primary::String),
        '$' => preceded('$', alt((
            special_var.map(Primary::SpecialVar),
            ident.map(Primary::EnvVar),
        ))),
        '@' => preceded('@', ident).map(Primary::ShellVar),
        '(' => alt((
            ('(', space0, ')').value(Primary::Unit),
            delimited(('(', space0), expr, (space0, ')'))
                .map(|expr| Primary::Paren(Box::new(expr)))
        )),
        'r' => raw_string.map(Primary::String),
        'p' => path_string.map(Primary::PathString),
        '[' => delimited(('[', space0), separated(0.., expr, delimited(space0, ',', space0)), ']')
            .map(|exprs| {
                Primary::Array(exprs)
            }),
        't' => "true".value(Primary::Bool(true)),
        'f' => "false".value(Primary::Bool(false)),
        '0'..='9' => number,
        'n' => "none".value(Primary::Option(None)),
        's' => delimited(("some", space0, '(', space0), expr, (space0, ')'))
            .map(|expr| Primary::Option(Some(Box::new(expr)))),
        _ => fail,
    )
    .spanned()
    .parse_next(input)
}
pub fn expr_prefix(input: &mut Input) -> SpannedResult<ExprPrefix> {
    use ExprPrefix::*;
    dispatch! {any;
        '!' => empty.value(Not),
        '-' => empty.value(Neg),
        _ => fail,
    }
    .spanned()
    .parse_next(input)
}
pub fn expr_infix(input: &mut Input) -> SpannedResult<ExprInfix> {
    use ExprInfix::*;
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
    }
    .spanned()
    .parse_next(input)
}
pub fn expr_postfix(input: &mut Input) -> SpannedResult<ExprPostfix> {
    use ExprPostfix::*;
    dispatch! {any;
        '!' => empty.value(Unwrap),
        '?' => empty.value(IsSome),
        '@' => empty.value(Length),
        '[' => delimited(space0, expr, (space0, ']'))
            .map(|index|Index(Box::new(index))),
        _ => fail,
    }
    .spanned()
    .parse_next(input)
}
pub fn expr(input: &mut Input) -> SpannedResult<Expr> {
    expr_pratt(input, 0)
}
pub fn expr_pratt(input: &mut Input, min_power: i32) -> SpannedResult<Expr> {
    // 前置演算子 or 値
    let mut lhs = if let Some(prefix) = opt(expr_prefix).parse_next(input)? {
        let _ = space0.parse_next(input)?;
        let rhs = expr_pratt(input, prefix.inner.power())?;
        prefix.apply(rhs)
    }
    else {
        let primary = expr_primary.parse_next(input)?;
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
        let _ = space0.parse_next(input)?;
        let checkpoint = input.checkpoint();

        // 中置演算子
        if let Some(infix) = opt(expr_infix).parse_next(input)? {
            let (l_power, r_power) = infix.inner.power();
            if l_power < min_power {
                input.reset(&checkpoint);
                break;
            }
            let _ = space0.parse_next(input)?;
            let rhs = expr_pratt(input, r_power)?;
            lhs = infix.apply(lhs, rhs);
            continue;
        }

        // 後置演算子
        if let Some(postfix) = opt(expr_postfix).parse_next(input)? {
            let power = postfix.inner.power();
            if power < min_power {
                input.reset(&checkpoint);
                break;
            }
            lhs = postfix.apply(lhs);
            continue;
        }

        break;
    }

    Ok(lhs)
}
