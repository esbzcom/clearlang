use crate::tokens::{ident_p, kw};
use crate::ErrTy;
use chumsky::prelude::*;

// Parse a namespaced path like `std::str::len` and return it as a single String.
// This is only used for callees in function calls; variables remain simple idents.
fn path_segment_p<'a>() -> impl Parser<'a, &'a str, String, ErrTy<'a>> {
    choice((kw("contract").to("contract".to_string()), ident_p()))
}

pub(crate) fn path_name_p<'a>() -> impl Parser<'a, &'a str, String, ErrTy<'a>> {
    path_segment_p()
        .then(
            (just("::").padded().ignore_then(path_segment_p()))
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(head, tail)| {
            if tail.is_empty() {
                return head;
            }
            let mut s = head;
            for part in tail {
                s.push_str("::");
                s.push_str(&part);
            }
            s
        })
        .padded()
}

pub(crate) fn path_segments_p<'a>() -> impl Parser<'a, &'a str, Vec<String>, ErrTy<'a>> {
    path_segment_p()
        .then(
            (just("::").padded().ignore_then(path_segment_p()))
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(head, tail)| {
            let mut out = Vec::with_capacity(1 + tail.len());
            out.push(head);
            out.extend(tail);
            out
        })
        .padded()
}
