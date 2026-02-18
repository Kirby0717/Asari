use super::*;

use winnow::{ascii::digit1, stream::Offset};
pub fn bin_int(input: &mut Input) -> ModalResult<i64> {
    trace(
        "bin_int",
        take_while(1.., '0'..='1').try_map_with_span(|s| {
            i64::from_str_radix(s, 2).map_err(ParseErrorKind::ParseBinError)
        }),
    )
    .cut()
    .parse_next(input)
}
pub fn oct_int(input: &mut Input) -> ModalResult<i64> {
    trace(
        "oct_int",
        take_while(1.., '0'..='7').try_map_with_span(|s| {
            i64::from_str_radix(s, 8).map_err(ParseErrorKind::ParseOctError)
        }),
    )
    .cut()
    .parse_next(input)
}
pub fn hex_int(input: &mut Input) -> ModalResult<i64> {
    trace(
        "hex_int",
        take_while(1.., ('0'..='9', 'A'..='F', 'a'..='f')).try_map_with_span(
            |s| {
                i64::from_str_radix(s, 16)
                    .map_err(ParseErrorKind::ParseHexError)
            },
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
        let float = take(l)
            .try_map_with_span(|s| {
                s.parse::<f64>().map_err(ParseErrorKind::ParseFloatError)
            })
            .parse_next(input)?;
        Ok(Primary::Float(float))
    }
    else {
        let l = int_checkpoint.offset_from(&start);
        let int = take(l)
            .try_map_with_span(|s| {
                s.parse::<i64>().map_err(ParseErrorKind::ParseDecError)
            })
            .parse_next(input)?;
        Ok(Primary::Int(int))
    }
}
pub fn number(input: &mut Input) -> ModalResult<Primary> {
    dispatch! {peek(opt(take(2_usize)));
        Some("0b") => preceded("0b", bin_int).map(Primary::Int),
        Some("0o") => preceded("0o", oct_int).map(Primary::Int),
        Some("0x") => preceded("0x", hex_int).map(Primary::Int),
        _ => dec_number,
    }
    .parse_next(input)
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
        't' => keyword("true").value(Primary::Bool(true)),
        'f' => keyword("false").value(Primary::Bool(false)),
        '0'..='9' => number,
        'n' => keyword("none").value(Primary::Option(None)),
        's' => delimited((keyword("some"), space0, '(', space0), expr, (space0, ')'))
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
            .map(|index| Index(Box::new(index))),
        'a' => preceded(('s', space0), expr_type)
            .map(Cast),
        _ => fail,
    }
    .spanned()
    .parse_next(input)
}
pub fn expr_type(input: &mut Input) -> SpannedResult<Type> {
    use Type::*;
    let mut type_name = take_while(1.., 'a'..='z');
    dispatch! {type_name;
        "string" => empty.value(String),
        "int" => empty.value(Int),
        "float" => empty.value(Float),
        "bool" => empty.value(Bool),
        "array" => delimited((space0, '<', space0), expr_type, (space0, '>', space0))
            .map(|t| Array(Box::new(t.inner))),
        "option" => delimited((space0, '<', space0), expr_type, (space0, '>', space0))
            .map(|t| Option(Box::new(t.inner))),
        "unit" => empty.value(Unit),
        _ => fail,
    }
    .spanned()
    .parse_next(input)
}
pub fn expr(input: &mut Input) -> SpannedResult<Expr> {
    expr_pratt(input, 0)
}
fn expr_pratt(input: &mut Input, min_power: i32) -> SpannedResult<Expr> {
    // 前置演算子 or 値
    let mut lhs = if let Some(prefix) = opt(expr_prefix).parse_next(input)? {
        let _ = space0.parse_next(input)?;
        let rhs = expr_pratt(input, prefix.inner.power())?;
        prefix.apply(rhs)
    }
    else {
        let primary = expr_primary.parse_next(input)?;
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
