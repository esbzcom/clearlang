use chumsky::prelude::*;
use crate::ErrTy;
use crate::literals::{int_lit, bool_lit, str_lit};
use crate::path::path_name_p;
use crate::tokens::{ident_p, ctor_name_p};
use lumi_ast::{Expr, BinOp, Span, MatchPat, MatchArm};

pub(crate) fn expr_p<'a>() -> impl Parser<'a, &'a str, Expr, ErrTy<'a>> {
    recursive(|expr| {
        let call_args = expr
            .clone()
            .separated_by(just(',').padded().labelled("comma"))
            .collect::<Vec<_>>()
            .delimited_by(
                just('(').padded().labelled("'('") ,
                just(')').padded().labelled("')'")
            );

        // Prefer parsing a namespaced call when parentheses follow a path
        let call_expr = path_name_p()
            .then(call_args.clone())
            .map_with(|(name, args), e| {
                let sp: chumsky::span::SimpleSpan<usize> = e.span();
                Expr::Call { callee: name, args, span: Span { start: sp.start, end: sp.end } }
            });

        // Constructors: Some(x), Ok(x), Err(e); None (with or without parentheses)
        let ctor_call = ctor_name_p()
            .then(call_args.clone().or_not())
            .map_with(|(name, maybe_args), e| {
                let args = maybe_args.unwrap_or_default();
                let sp: chumsky::span::SimpleSpan<usize> = e.span();
                Expr::Call { callee: name, args, span: Span { start: sp.start, end: sp.end } }
            });

        let var_expr = ident_p().map_with(|name, e| {
            let sp: chumsky::span::SimpleSpan<usize> = e.span();
            Expr::Var(name, Span { start: sp.start, end: sp.end })
        });

        // return expression
        let ret_expr = just("return").padded()
            .ignore_then(expr.clone())
            .map_with(|e_inner, e| {
                let sp = e.span();
                Expr::Return { expr: Box::new(e_inner), span: Span { start: sp.start, end: sp.end } }
            });

        // match expression: match <expr> { <pat> => <expr>, ... }
        let some_pat = just("Some").padded()
            .ignore_then(just('(').padded())
            .ignore_then(ident_p())
            .then_ignore(just(')').padded())
            .map(MatchPat::Some);
        let none_pat = just("None").padded().to(MatchPat::None);
        let ok_pat = just("Ok").padded()
            .ignore_then(just('(').padded())
            .ignore_then(ident_p())
            .then_ignore(just(')').padded())
            .map(MatchPat::Ok);
        let err_pat = just("Err").padded()
            .ignore_then(just('(').padded())
            .ignore_then(ident_p())
            .then_ignore(just(')').padded())
            .map(MatchPat::Err);
        let pat = choice((some_pat, none_pat, ok_pat, err_pat)).labelled("match pattern");
        let arm = pat.then_ignore(just("=>").padded()).then(expr.clone()).map(|(p, e)| MatchArm { pat: p, expr: e });
        let arms = arm
            .separated_by(just(',').padded())
            .collect::<Vec<_>>()
            .delimited_by(just('{').padded(), just('}').padded());
        let match_expr = just("match").padded()
            .ignore_then(expr.clone())
            .then(arms)
            .map_with(|(scrut, arms), e| {
                let sp = e.span();
                Expr::Match { scrutinee: Box::new(scrut), arms, span: Span { start: sp.start, end: sp.end } }
            });

        let atom = choice((
            int_lit(),
            bool_lit(),
            str_lit(),
            expr.clone().delimited_by(
                just('(').padded().labelled("'('") ,
                just(')').padded().labelled("')'")
            ),
            ret_expr,
            match_expr,
            ctor_call,
            call_expr,
            var_expr,
        ))
        .padded()
        .labelled("expression")
        .boxed();

        // Helper to compute a span for composite expressions
        fn span_of(e: &Expr) -> (usize, usize) {
            match e {
                Expr::Int(_, sp)
                | Expr::Bool(_, sp)
                | Expr::String(_, sp)
                | Expr::Var(_, sp) => (sp.start, sp.end),
                Expr::Bin { span, .. }
                | Expr::Call { span, .. }
                | Expr::Match { span, .. }
                | Expr::Return { span, .. } => (span.start, span.end),
            }
        }

        // multiplicative (*, /)
        let mul = atom.clone().foldl(
            (one_of("*/").padded().then(atom.clone().boxed())).repeated(),
            |lhs, (op, rhs)| {
                let op = if op == '*' { BinOp::Mul } else { BinOp::Div };
                let (ls, _) = span_of(&lhs);
                let (_, re) = span_of(&rhs);
                Expr::Bin {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span: Span { start: ls, end: re },
                }
            },
        );

        // additive (+, -)
        let add = mul.clone().foldl(
            (one_of("+-").padded().then(mul.clone().boxed())).repeated(),
            |lhs, (op, rhs)| {
                let op = if op == '+' { BinOp::Add } else { BinOp::Sub };
                let (ls, _) = span_of(&lhs);
                let (_, re) = span_of(&rhs);
                Expr::Bin {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span: Span { start: ls, end: re },
                }
            },
        );

        add
    })
    .padded()
    .labelled("expression")
}
