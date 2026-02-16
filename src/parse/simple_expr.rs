use super::*;

pub fn simple_expr_primary(input: &mut Input) -> SpannedResult<Primary> {
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
        _ => fail,
    )
    .spanned()
    .parse_next(input)
}
pub fn simple_expr_postfix(input: &mut Input) -> SpannedResult<ExprPostfix> {
    use ExprPostfix::*;
    dispatch!(any;
        '!' => empty.value(Unwrap),
        '?' => empty.value(IsSome),
        '@' => empty.value(Length),
        '[' => delimited(space0, expr, (space0, ']'))
            .map(|index|Index(Box::new(index))),
        _ => fail,
    )
    .spanned()
    .parse_next(input)
}
pub fn simple_expr_infix(input: &mut Input) -> SpannedResult<ExprInfix> {
    use ExprInfix::*;
    dispatch!(any;
        '^' => empty.value(UnwrapOr),
        _ => fail,
    )
    .spanned()
    .parse_next(input)
}
pub fn simple_expr(input: &mut Input) -> SpannedResult<Expr> {
    // 値
    let primary = simple_expr_primary.parse_next(input)?;
    let mut lhs = Spanned {
        span: primary.span.clone(),
        inner: Expr::Primary(primary),
    };

    loop {
        // 後置演算子
        if let Some(postfix) = opt(simple_expr_postfix).parse_next(input)? {
            lhs = postfix.apply(lhs);
            continue;
        }

        // 中置演算子
        if let Some(infix) = opt(simple_expr_infix).parse_next(input)? {
            let rhs = simple_expr.parse_next(input)?;
            lhs = infix.apply(lhs, rhs);
            continue;
        }

        break;
    }

    Ok(lhs)
}
