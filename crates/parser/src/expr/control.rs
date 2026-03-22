use chumsky::prelude::*;
use clg_ast::{Expr, MatchArm, MatchPat, Span};

use crate::path::path_name_p;
use crate::tokens::ident_p;
use crate::ErrTy;

enum IfLetKind {
    OptionSome(String),
    ResultOk(String),
    ResultErr(String),
}

pub(super) fn if_let_expr_p<'a, P, B>(
    expr: P,
    block_expr: B,
) -> impl Parser<'a, &'a str, Expr, ErrTy<'a>>
where
    P: Parser<'a, &'a str, Expr, ErrTy<'a>> + Clone + 'a,
    B: Parser<'a, &'a str, Expr, ErrTy<'a>> + Clone + 'a,
{
    let if_let_pat = choice((
        just("Some")
            .padded()
            .ignore_then(just('(').padded())
            .ignore_then(ident_p())
            .then_ignore(just(')').padded())
            .map(IfLetKind::OptionSome),
        just("Ok")
            .padded()
            .ignore_then(just('(').padded())
            .ignore_then(ident_p())
            .then_ignore(just(')').padded())
            .map(IfLetKind::ResultOk),
        just("Err")
            .padded()
            .ignore_then(just('(').padded())
            .ignore_then(ident_p())
            .then_ignore(just(')').padded())
            .map(IfLetKind::ResultErr),
    ));

    just("if")
        .padded()
        .ignore_then(just("let").padded())
        .ignore_then(if_let_pat)
        .then_ignore(just('=').padded())
        .then(expr)
        .then(block_expr.clone())
        .then_ignore(just("else").padded())
        .then(block_expr)
        .map_with(|(((pat_kind, scrutinee), then_br), else_br), e| {
            let sp = e.span();
            let (success_pat, failure_pat) = match pat_kind {
                IfLetKind::OptionSome(name) => (MatchPat::Some(name), MatchPat::None),
                IfLetKind::ResultOk(name) => {
                    let tmp = format!("__iflet_tmp{}", sp.start);
                    (MatchPat::Ok(name), MatchPat::Err(tmp))
                }
                IfLetKind::ResultErr(name) => {
                    let tmp = format!("__iflet_tmp{}", sp.start);
                    (MatchPat::Err(name), MatchPat::Ok(tmp))
                }
            };
            let arms = vec![
                MatchArm {
                    pat: success_pat,
                    expr: then_br,
                },
                MatchArm {
                    pat: failure_pat,
                    expr: else_br,
                },
            ];
            Expr::Match {
                scrutinee: Box::new(scrutinee),
                arms,
                span: Span {
                    start: sp.start,
                    end: sp.end,
                },
            }
        })
}

pub(super) fn if_expr_p<'a, P, B>(
    expr: P,
    block_expr: B,
) -> impl Parser<'a, &'a str, Expr, ErrTy<'a>>
where
    P: Parser<'a, &'a str, Expr, ErrTy<'a>> + Clone + 'a,
    B: Parser<'a, &'a str, Expr, ErrTy<'a>> + Clone + 'a,
{
    let elif_kw = just("else")
        .padded()
        .ignore_then(just("if").padded())
        .to(());
    let if_head = just("if")
        .padded()
        .ignore_then(expr.clone())
        .then(block_expr.clone());
    let elif_chain = (elif_kw.ignore_then(expr).then(block_expr.clone()))
        .repeated()
        .collect::<Vec<_>>();
    let else_clause = just("else").padded().ignore_then(block_expr);

    if_head
        .then(elif_chain)
        .then(else_clause.or_not().validate(|else_opt, extra, emit| {
            if else_opt.is_none() {
                emit.emit(Rich::custom(
                    extra.span(),
                    "missing `else` in expression-form `if`",
                ));
            }
            else_opt
        }))
        .map_with(|(((cond0, then0), mut elifs), else_opt), e| {
            let sp = e.span();
            let else_br = else_opt.unwrap_or({
                Expr::Bool(
                    false,
                    Span {
                        start: sp.start,
                        end: sp.end,
                    },
                )
            });
            let mut acc = else_br;
            while let Some((c, t)) = elifs.pop() {
                acc = Expr::If {
                    cond: Box::new(c),
                    then_br: Box::new(t),
                    else_br: Box::new(acc),
                    span: Span {
                        start: sp.start,
                        end: sp.end,
                    },
                };
            }
            Expr::If {
                cond: Box::new(cond0),
                then_br: Box::new(then0),
                else_br: Box::new(acc),
                span: Span {
                    start: sp.start,
                    end: sp.end,
                },
            }
        })
}

pub(super) fn match_expr_p<'a, P>(expr: P) -> impl Parser<'a, &'a str, Expr, ErrTy<'a>>
where
    P: Parser<'a, &'a str, Expr, ErrTy<'a>> + Clone + 'a,
{
    let some_pat = just("Some")
        .padded()
        .ignore_then(just('(').padded())
        .ignore_then(ident_p())
        .then_ignore(just(')').padded())
        .map(MatchPat::Some);
    let none_pat = just("None").padded().to(MatchPat::None);
    let ok_pat = just("Ok")
        .padded()
        .ignore_then(just('(').padded())
        .ignore_then(ident_p())
        .then_ignore(just(')').padded())
        .map(MatchPat::Ok);
    let err_pat = just("Err")
        .padded()
        .ignore_then(just('(').padded())
        .ignore_then(ident_p())
        .then_ignore(just(')').padded())
        .map(MatchPat::Err);
    let wildcard_pat = just('_').padded().to(MatchPat::Wildcard);
    let enum_binders = ident_p()
        .separated_by(just(',').padded().labelled("comma"))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just('(').padded(), just(')').padded());
    let enum_pat = path_name_p()
        .try_map(|path, span| {
            let parts: Vec<&str> = path.split("::").collect();
            if parts.len() < 2 {
                Err(Rich::custom(span, "enum pattern must be `Type::Variant`"))
            } else {
                let (enum_path, variant) = parts.split_at(parts.len() - 1);
                Ok((enum_path.join("::"), variant[0].to_string()))
            }
        })
        .then(enum_binders.or_not())
        .map(|((enum_name, variant), binders)| MatchPat::EnumVariant {
            enum_name,
            variant,
            binders: binders.unwrap_or_default(),
        });
    let pat = choice((wildcard_pat, some_pat, none_pat, ok_pat, err_pat, enum_pat))
        .labelled("match pattern");
    let arm = pat
        .then_ignore(just("=>").padded())
        .then(expr.clone())
        .map(|(p, e)| MatchArm { pat: p, expr: e });
    let arms = arm
        .separated_by(just(',').padded())
        .collect::<Vec<_>>()
        .delimited_by(just('{').padded(), just('}').padded());

    just("match")
        .padded()
        .ignore_then(expr)
        .then(arms)
        .map_with(|(scrut, arms), e| {
            let sp = e.span();
            Expr::Match {
                scrutinee: Box::new(scrut),
                arms,
                span: Span {
                    start: sp.start,
                    end: sp.end,
                },
            }
        })
}
