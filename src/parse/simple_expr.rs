use super::*;

// 変数と配列とクォート系と括弧のみのexpr_primary
pub fn simple_expr_primary(input: &mut Input) -> ModalResult<Primary> {
    trace(
        "simple_expr_primary",
        dispatch! {peek(any);
            '\'' => quoted_string.map(Primary::String),
            '"' => double_quoted_string.map(Primary::String),
            '$' => preceded('$', alt((
                delimited(('(', space0), command_line.spanned(), (space0, ')'))
                    .map(|shell_command| Primary::CommandSubst(Box::new(shell_command))),
                special_var.map(Primary::SpecialVar),
                ident.cut().map(Primary::EnvVar),
            ))),
            '@' => preceded('@', ident).cut().map(Primary::ShellVar),
            '(' => alt((
                ('(', space0, ')').value(Primary::Unit),
                delimited(('(', space0), expr.spanned(), (space0, ')'))
                    .map(|expr| Primary::Paren(Box::new(expr)))
            )),
            'r' => raw_string.map(Primary::String),
            'p' => path_string.map(Primary::PathString),
            '[' => delimited(('[', space0), separated(0.., expr.spanned(), delimited(space0, ',', space0)), ']')
                .map(|exprs| {
                    Primary::Array(exprs)
                }),
            _ => fail,
        },
    )
    .parse_next(input)
}
pub fn simple_expr_postfix(input: &mut Input) -> ModalResult<ExprPostfix> {
    use ExprPostfix::*;
    trace(
        "simple_expr_postfix",
        dispatch! {any;
            '!' => empty.value(Unwrap),
            '?' => empty.value(IsSome),
            '@' => empty.value(Length),
            '[' => delimited(space0, expr.spanned(), (space0, ']'))
                .map(|index|Index(Box::new(index))),
            _ => fail,
        },
    )
    .parse_next(input)
}
pub fn simple_expr_infix(input: &mut Input) -> ModalResult<ExprInfix> {
    use ExprInfix::*;
    trace(
        "simple_expr_infix",
        dispatch! {any;
            '^' => empty.value(UnwrapOr),
            _ => fail,
        },
    )
    .parse_next(input)
}
#[inline(always)]
pub fn simple_expr(input: &mut Input) -> ModalResult<Expr> {
    trace("simple_expr", simple_expr_pratt.map(|expr| expr.inner))
        .parse_next(input)
}
pub fn simple_expr_pratt(input: &mut Input) -> ModalResult<Spanned<Expr>> {
    // 値
    let primary = simple_expr_primary.spanned().parse_next(input)?;
    let mut lhs = Spanned {
        span: primary.span.clone(),
        inner: Expr::Primary(primary),
    };

    loop {
        // 後置演算子
        if let Some(postfix) =
            opt(simple_expr_postfix.spanned()).parse_next(input)?
        {
            lhs = postfix.apply(lhs);
            continue;
        }

        // 中置演算子
        if let Some(infix) =
            opt(simple_expr_infix.spanned()).parse_next(input)?
        {
            let rhs = simple_expr_pratt.parse_next(input)?;
            lhs = infix.apply(lhs, rhs);
            continue;
        }

        break;
    }

    Ok(lhs)
}
