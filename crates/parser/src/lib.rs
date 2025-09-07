use chumsky::prelude::*;
use chumsky::text;
use lumi_ast::*; // assumes Expr, BinOp, Effect, Type, Param, Func, Program are defined

type ErrTy<'a> = extra::Err<Rich<'a, char>>;

fn ident_start<'a>() -> impl Parser<'a, &'a str, char, ErrTy<'a>> {
    any().filter(|c: &char| c.is_alphabetic() || *c == '_')
}

fn ident_continue<'a>() -> impl Parser<'a, &'a str, char, ErrTy<'a>> {
    any().filter(|c: &char| c.is_alphanumeric() || *c == '_')
}

fn kw<'a>(s: &'static str) -> impl Parser<'a, &'a str, &'static str, ErrTy<'a>> {
    just(s)
        .then(ident_continue().or_not())
        .try_map(move |(kwd, next), span| {
            if next.is_some() {
                Err(Rich::custom(
                    span,
                    format!("keyword `{kwd}` must be a word"),
                ))
            } else {
                Ok(s)
            }
        })
        .padded()
}

fn effect_p<'a>() -> impl Parser<'a, &'a str, Effect, ErrTy<'a>> {
    choice((
        kw("pure").to(Effect::Pure),
        kw("mut").to(Effect::Mut),
        kw("io").to(Effect::Io),
    ))
    .padded()
}

fn ty_p<'a>() -> impl Parser<'a, &'a str, Type, ErrTy<'a>> {
    choice((
        kw("Int").to(Type::Int),
        kw("Bool").to(Type::Bool),
    ))
    .padded()
    .labelled("type")
}

fn ident_p<'a>() -> impl Parser<'a, &'a str, String, ErrTy<'a>> {
    ident_start()
        .then(ident_continue().repeated().collect::<String>())
        .map(|(h, rest)| {
            let mut s = String::with_capacity(1 + rest.len());
            s.push(h);
            s.push_str(&rest);
            s
        })
        .try_map(|s: String, span| match s.as_str() {
            "fn" | "pure" | "mut" | "io" | "Int" | "Bool" | "true" | "false" => Err(Rich::custom(
                span,
                format!("`{s}` is a reserved keyword"),
            )),
            _ => Ok(s),
        })
        .padded()
        .labelled("identifier")
}

// Parse a namespaced path like `std::str::len` and return it as a single String.
// This is only used for callees in function calls; variables remain simple idents.
fn path_name_p<'a>() -> impl Parser<'a, &'a str, String, ErrTy<'a>> {
    ident_p()
        .then(
            (just("::").padded().ignore_then(ident_p()))
                .repeated()
                .collect::<Vec<_>>()
        )
        .map(|(head, tail)| {
            if tail.is_empty() { return head; }
            let mut s = head;
            for part in tail { s.push_str("::"); s.push_str(&part); }
            s
        })
        .padded()
}

fn func_name_p<'a>() -> impl Parser<'a, &'a str, String, ErrTy<'a>> {
    // Use an explicit charset to produce a clearer expectation than a generic filter.
    let start = one_of("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_")
        .labelled("function name");
    start
        .then(ident_continue().repeated().collect::<String>())
        .map(|(h, rest)| {
            let mut s = String::with_capacity(1 + rest.len());
            s.push(h);
            s.push_str(&rest);
            s
        })
        .try_map(|s: String, span| match s.as_str() {
            "fn" | "pure" | "mut" | "io" | "Int" | "Bool" | "true" | "false" => Err(Rich::custom(
                span,
                format!("`{s}` is a reserved keyword"),
            )),
            _ => Ok(s),
        })
        .padded()
}

fn int_lit<'a>() -> impl Parser<'a, &'a str, Expr, ErrTy<'a>> {
    text::int(10)
        .from_str::<i64>()
        .unwrapped()
        .map_with(|n, e| {
            let sp: chumsky::span::SimpleSpan<usize> = e.span();
            Expr::Int(n, Span { start: sp.start, end: sp.end })
        })
        .padded()
        .labelled("int literal")
}

fn bool_lit<'a>() -> impl Parser<'a, &'a str, Expr, ErrTy<'a>> {
    choice((
        kw("true").map_with(|_, e| { let sp: chumsky::span::SimpleSpan<usize> = e.span(); Expr::Bool(true, Span { start: sp.start, end: sp.end }) }),
        kw("false").map_with(|_, e| { let sp: chumsky::span::SimpleSpan<usize> = e.span(); Expr::Bool(false, Span { start: sp.start, end: sp.end }) }),
    ))
    .labelled("bool literal")
}

fn expr_p<'a>() -> impl Parser<'a, &'a str, Expr, ErrTy<'a>> {
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

        // multiplicative (*, /)
        fn span_of(e: &Expr) -> (usize, usize) {
            match e {
                Expr::Int(_, sp) | Expr::Bool(_, sp) | Expr::Var(_, sp) => (sp.start, sp.end),
                Expr::Bin { span, .. } | Expr::Call { span, .. } => (span.start, span.end),
            }
        }

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

fn param_p<'a>() -> impl Parser<'a, &'a str, Param, ErrTy<'a>> {
    ident_p()
        .then_ignore(just(':').padded())
        .then(ty_p())
        .map(|(name, ty)| Param { name, ty })
        .padded()
}

fn params_p<'a>() -> impl Parser<'a, &'a str, Vec<Param>, ErrTy<'a>> {
    param_p()
        .separated_by(just(',').padded().labelled("comma"))
        .allow_trailing()
        .collect::<Vec<_>>() // required in chumsky 0.10
        .delimited_by(
            just('(').padded().labelled("'('") ,
            just(')').padded().labelled("')'")
        )
}

fn func_p<'a>() -> impl Parser<'a, &'a str, Func, ErrTy<'a>> {
    effect_p()
        .or_not()
        .then_ignore(kw("fn"))
        .then(func_name_p())
        .then(params_p())
        .then_ignore(just("->").padded())
        .then(ty_p())
        .then(expr_p().delimited_by(just('{').padded(), just('}').padded()))
        .map(|((((eff_opt, name), params), ret), body)| Func {
            effect: eff_opt.unwrap_or(Effect::None),
            name,
            params,
            ret,
            body,
        })
}

fn program_p<'a>() -> impl Parser<'a, &'a str, Program, ErrTy<'a>> {
    func_p()
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()            // 👈 collect into Vec<Func>
        .map(|funcs| Program { funcs }) // wrap
        .then_ignore(end())             // consume EOF
}

pub fn parse(src: &str) -> Result<Program, String> {
    program_p().parse(src).into_result().map_err(|errs| {
        errs
            .into_iter()
            .map(|e| {
                let span = e.span();
                // Collect expected labels/tokens to enrich the message.
                let expected: Vec<String> = e.expected().map(|p| p.to_string()).collect();
                if expected.is_empty() {
                    format!("error at {}..{}: {}", span.start, span.end, e)
                } else {
                    format!(
                        "error at {}..{}: {}; expected: {}",
                        span.start,
                        span.end,
                        e,
                        expected.join(", ")
                    )
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    })
}
