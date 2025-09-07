use chumsky::prelude::*;
use crate::ErrTy;
use crate::literals::{int_lit, bool_lit, str_lit};
use crate::path::path_name_p;
use crate::tokens::ident_p;
use lumi_ast::{Expr, BinOp, Span};

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

        let var_expr = ident_p().map_with(|name, e| {
            let sp: chumsky::span::SimpleSpan<usize> = e.span();
            Expr::Var(name, Span { start: sp.start, end: sp.end })
        });

        let atom = choice((
            int_lit(),
            bool_lit(),
            str_lit(),
            expr.clone().delimited_by(
                just('(').padded().labelled("'('") ,
                just(')').padded().labelled("')'")
            ),
            call_expr,
            var_expr,
        ))
        .padded()
        .labelled("expression")
        .boxed();

        // Helper to compute a span for composite expressions
        fn span_of(e: &Expr) -> (usize, usize) {
            match e {
                Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::Str(_, sp) | Expr::Var(_, sp) => (sp.start, sp.end),
                Expr::Bin { span, .. } | Expr::Call { span, .. } => (span.start, span.end),
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

