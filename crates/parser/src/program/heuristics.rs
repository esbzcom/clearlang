pub(super) fn has_unclosed_paren(src: &str) -> bool {
    let mut depth = 0u32;
    let mut in_string = false;
    for b in src.bytes() {
        if b == b'"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match b {
            b'(' => depth = depth.saturating_add(1),
            b')' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth > 0
}

pub(super) fn looks_like_missing_comma(src: &str) -> bool {
    let bytes = src.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'(' {
            let mut j = i + 1;
            let mut saw_gap = false;
            let mut saw_comma = false;
            let mut saw_operator = false;
            let mut prev_token = false;
            let mut pending_gap = false;
            while j < bytes.len() && bytes[j] != b')' {
                let b = bytes[j];
                if b == b'"' {
                    if pending_gap {
                        saw_gap = true;
                    }
                    prev_token = true;
                    pending_gap = false;
                    j += 1;
                    while j < bytes.len() && bytes[j] != b'"' {
                        j += 1;
                    }
                } else if b == b',' {
                    saw_comma = true;
                    prev_token = false;
                    pending_gap = false;
                } else if matches!(
                    b,
                    b'+' | b'-'
                        | b'*'
                        | b'/'
                        | b'%'
                        | b'<'
                        | b'>'
                        | b'='
                        | b'!'
                        | b'&'
                        | b'|'
                        | b'?'
                        | b':'
                        | b'.'
                ) {
                    saw_operator = true;
                    prev_token = false;
                    pending_gap = false;
                } else if b.is_ascii_whitespace() {
                    if prev_token {
                        pending_gap = true;
                    }
                } else if b.is_ascii_alphanumeric() || b == b'_' {
                    if pending_gap {
                        saw_gap = true;
                    }
                    prev_token = true;
                    pending_gap = false;
                } else {
                    prev_token = false;
                    pending_gap = false;
                }
                j += 1;
            }
            if j < bytes.len() && saw_gap && !saw_comma && !saw_operator {
                return true;
            }
            i = j;
        }
        i += 1;
    }
    false
}

pub(super) fn keyword_missing_brace(line: &str, keyword: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let key = keyword.as_bytes();
    let mut idx = 0usize;
    while idx + key.len() <= bytes.len() {
        let Some(rel) = line[idx..].find(keyword) else {
            break;
        };
        let start = idx + rel;
        let end = start + key.len();
        let before_ok =
            start == 0 || !bytes[start - 1].is_ascii_alphanumeric() && bytes[start - 1] != b'_';
        let after_ok =
            end == bytes.len() || !bytes[end].is_ascii_alphanumeric() && bytes[end] != b'_';
        if before_ok && after_ok {
            let mut j = end;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j >= bytes.len() || bytes[j] != b'{' {
                return Some(start);
            }
        }
        idx = end;
    }
    None
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn prev_non_ws_byte(bytes: &[u8], mut idx: usize) -> Option<u8> {
    while idx > 0 {
        idx -= 1;
        let b = bytes[idx];
        if !b.is_ascii_whitespace() {
            return Some(b);
        }
    }
    None
}

fn can_start_lambda_like(bytes: &[u8], idx: usize) -> bool {
    match prev_non_ws_byte(bytes, idx) {
        None => true,
        Some(prev) => !(is_ident_byte(prev) || prev == b')' || prev == b']'),
    }
}

pub(super) fn find_untyped_lambda(src: &str) -> Option<usize> {
    let bytes = src.as_bytes();
    let mut i = 0usize;
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut block_comment_depth = 0usize;
    while i < bytes.len() {
        if in_line_comment {
            if bytes[i] == b'\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }
        if block_comment_depth > 0 {
            if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                block_comment_depth += 1;
                i += 2;
                continue;
            }
            if i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                block_comment_depth -= 1;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if in_string {
            if bytes[i] == b'\\' {
                i = (i + 2).min(bytes.len());
                continue;
            }
            if bytes[i] == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            in_line_comment = true;
            i += 2;
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            block_comment_depth = 1;
            i += 2;
            continue;
        }
        if bytes[i] == b'"' {
            in_string = true;
            i += 1;
            continue;
        }

        if bytes[i] == b'(' && can_start_lambda_like(bytes, i) {
            let start = i;
            let mut j = i + 1;
            let mut saw_ident = false;
            let mut saw_colon = false;
            let mut valid = true;
            while j < bytes.len() && bytes[j] != b')' {
                let b = bytes[j];
                if b.is_ascii_whitespace() || b == b',' {
                    j += 1;
                    continue;
                }
                if b == b':' {
                    saw_colon = true;
                    j += 1;
                    continue;
                }
                if b.is_ascii_alphabetic() || b == b'_' {
                    saw_ident = true;
                    j += 1;
                    while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_')
                    {
                        j += 1;
                    }
                    continue;
                }
                valid = false;
                break;
            }
            if valid && j < bytes.len() && bytes[j] == b')' && saw_ident && !saw_colon {
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j + 1 < bytes.len() && bytes[j] == b'=' && bytes[j + 1] == b'>' {
                    return Some(start);
                }
            }
        }
        i += 1;
    }
    None
}

pub(super) fn find_capture_list_lambda(src: &str) -> Option<usize> {
    let bytes = src.as_bytes();
    let mut i = 0usize;
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut block_comment_depth = 0usize;
    while i < bytes.len() {
        if in_line_comment {
            if bytes[i] == b'\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }
        if block_comment_depth > 0 {
            if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                block_comment_depth += 1;
                i += 2;
                continue;
            }
            if i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                block_comment_depth -= 1;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if in_string {
            if bytes[i] == b'\\' {
                i = (i + 2).min(bytes.len());
                continue;
            }
            if bytes[i] == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            in_line_comment = true;
            i += 2;
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            block_comment_depth = 1;
            i += 2;
            continue;
        }
        if bytes[i] == b'"' {
            in_string = true;
            i += 1;
            continue;
        }

        if bytes[i] == b'[' && can_start_lambda_like(bytes, i) {
            let start = i;
            let mut j = i + 1;
            let mut valid = true;
            while j < bytes.len() && bytes[j] != b']' {
                let b = bytes[j];
                if b.is_ascii_whitespace() || b == b',' {
                    j += 1;
                    continue;
                }
                if b.is_ascii_alphabetic() || b == b'_' {
                    j += 1;
                    while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_')
                    {
                        j += 1;
                    }
                    continue;
                }
                valid = false;
                break;
            }
            if valid && j < bytes.len() && bytes[j] == b']' {
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j >= bytes.len() || bytes[j] != b'(' {
                    i += 1;
                    continue;
                }
                while j < bytes.len() && bytes[j] != b')' {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b')' {
                    j += 1;
                    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    if j + 1 < bytes.len() && bytes[j] == b'=' && bytes[j + 1] == b'>' {
                        return Some(start);
                    }
                }
            }
        }
        i += 1;
    }
    None
}

pub(super) fn find_export_import(src: &str) -> Option<(usize, usize)> {
    const PREFIX: &str = "export import";
    let mut offset = 0usize;
    for line in src.split_inclusive('\n') {
        let line_no_nl = line.strip_suffix('\n').unwrap_or(line);
        let line_text = line_no_nl.strip_suffix('\r').unwrap_or(line_no_nl);
        let trimmed = line_text.trim_start();
        if let Some(rest) = trimmed.strip_prefix(PREFIX) {
            let boundary_ok = rest
                .chars()
                .next()
                .map(|c| c.is_whitespace() || c == ';')
                .unwrap_or(true);
            if boundary_ok {
                let leading = line_text.len().saturating_sub(trimmed.len());
                let start = offset + leading;
                let end = start + PREFIX.len();
                return Some((start, end));
            }
        }
        offset += line.len();
    }
    None
}
