use super::*;

// 変数と配列とクォート系と括弧のみのexpr_primary
pub fn simple_expr_primary(input: &mut Input) -> ModalResult<Primary> {
    trace(
        "simple_expr_primary",
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
                paren.map(|expr| Primary::Paren(Box::new(expr)))
            )).cut(),
            'r' => raw_string.map(Primary::String),
            'p' => path_string.map(Primary::PathString),
            '[' => array.cut().map(Primary::Array),
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
            _ => fail,
        },
    )
    .parse_next(input)
}
pub fn simple_expr_index(input: &mut Input) -> ModalResult<Spanned<Expr>> {
    trace(
        "simple_expr_index",
        delimited(('[', space0), expr.spanned(), (space0, ']')),
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
    trace("simple_expr", simple_expr_pratt).parse_next(input)
}
pub fn simple_expr_pratt(input: &mut Input) -> ModalResult<Expr> {
    // 値
    let primary = simple_expr_primary.spanned().parse_next(input)?;
    let mut lhs = Expr::Primary(primary);

    loop {
        // 後置演算子
        if let Some(postfix) =
            opt(simple_expr_postfix.spanned()).parse_next(input)?
        {
            lhs = postfix.apply(lhs);
            continue;
        }

        // Index
        if let Some(index) = opt(simple_expr_index).parse_next(input)? {
            lhs = Expr::Index(Box::new(lhs), Box::new(index));
            continue;
        }

        // 中置演算子
        if let Some(infix) =
            opt(simple_expr_infix.spanned()).parse_next(input)?
        {
            let cursor = input.current_token_start();
            let rhs = simple_expr_pratt(input).map_err(|_| {
                winnow::error::ErrMode::Cut(ParseError {
                    kind: ParseErrorKind::Expr(ExprError::NoRhs),
                    span: cursor..cursor + 1,
                })
            })?;
            lhs = infix.apply(lhs, rhs);
            continue;
        }

        break;
    }

    Ok(lhs)
}
