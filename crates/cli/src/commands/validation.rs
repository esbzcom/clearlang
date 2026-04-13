pub(crate) fn validate_package_id(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("name is empty".to_string());
    }
    for segment in name.split("::") {
        validate_identifier_segment(segment)?;
    }
    Ok(())
}

pub(crate) fn validate_identifier_segment(segment: &str) -> Result<(), String> {
    if segment.is_empty() {
        return Err("contains empty `::` segment".to_string());
    }
    let mut chars = segment.chars();
    let first = chars.next().expect("segment non-empty");
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err(format!(
            "segment `{segment}` must start with ASCII letter or `_`"
        ));
    }
    for ch in chars {
        if !(ch == '_' || ch.is_ascii_alphanumeric()) {
            return Err(format!(
                "segment `{segment}` contains invalid character `{ch}`"
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_exact_semver(version: &str) -> Result<(), String> {
    if version.contains('-') || version.contains('+') {
        return Err(
            "must use exact MAJOR.MINOR.PATCH without pre-release/build metadata".to_string(),
        );
    }
    if !version.chars().all(|ch| ch.is_ascii_digit() || ch == '.') {
        return Err("must use exact MAJOR.MINOR.PATCH".to_string());
    }
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return Err("must use exact MAJOR.MINOR.PATCH".to_string());
    }
    for part in parts {
        if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(format!("version segment `{part}` is not numeric"));
        }
    }
    Ok(())
}

pub(crate) fn validate_semver_requirement(requirement: &str) -> Result<(), String> {
    let trimmed = requirement.trim();
    if trimmed.is_empty() {
        return Err("requirement is empty".to_string());
    }
    let core = if let Some(rest) = trimmed.strip_prefix('=') {
        rest
    } else if let Some(rest) = trimmed.strip_prefix('^') {
        rest
    } else if let Some(rest) = trimmed.strip_prefix('~') {
        rest
    } else {
        trimmed
    };
    validate_exact_semver(core).map_err(|_| {
        "requirement must use exact `=MAJOR.MINOR.PATCH` or range prefix (`^`/`~`) with MAJOR.MINOR.PATCH"
            .to_string()
    })
}

pub(crate) fn semver_requirement_matches_version(
    requirement: &str,
    version: &str,
) -> Result<bool, String> {
    validate_semver_requirement(requirement)?;
    validate_exact_semver(version)?;

    let trimmed = requirement.trim();
    let (kind, base_raw) = if let Some(rest) = trimmed.strip_prefix('=') {
        ('=', rest)
    } else if let Some(rest) = trimmed.strip_prefix('^') {
        ('^', rest)
    } else if let Some(rest) = trimmed.strip_prefix('~') {
        ('~', rest)
    } else {
        ('=', trimmed)
    };

    let base = parse_semver_triplet(base_raw)?;
    let candidate = parse_semver_triplet(version)?;
    Ok(match kind {
        '=' => candidate == base,
        '^' => {
            if base.0 > 0 {
                candidate.0 == base.0 && candidate >= base
            } else if base.1 > 0 {
                candidate.0 == 0 && candidate.1 == base.1 && candidate.2 >= base.2
            } else {
                candidate.0 == 0 && candidate.1 == 0 && candidate.2 == base.2
            }
        }
        '~' => candidate.0 == base.0 && candidate.1 == base.1 && candidate >= base,
        _ => false,
    })
}

pub(crate) fn validate_sha256_digest(digest: &str) -> Result<(), String> {
    const PREFIX: &str = "sha256:";
    if !digest.starts_with(PREFIX) {
        return Err("digest must start with `sha256:`".to_string());
    }
    let hex = &digest[PREFIX.len()..];
    if hex.len() != 64 {
        return Err("digest must have exactly 64 lowercase hex characters".to_string());
    }
    if !hex
        .chars()
        .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    {
        return Err("digest must be lowercase hex (`0-9`, `a-f`)".to_string());
    }
    Ok(())
}

pub(crate) fn validate_utc_rfc3339(field: &str, value: &str) -> Result<(), String> {
    parse_utc_timestamp_components(value)
        .map(|_| ())
        .map_err(|msg| format!("{field} must be a valid UTC RFC3339 timestamp: {msg}"))
}

pub(crate) fn parse_utc_timestamp_components(
    value: &str,
) -> Result<(u16, u8, u8, u8, u8, u8), String> {
    if value.len() != 20 {
        return Err("expected `YYYY-MM-DDTHH:MM:SSZ`".to_string());
    }
    let bytes = value.as_bytes();
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return Err("expected separators in `YYYY-MM-DDTHH:MM:SSZ`".to_string());
    }

    fn parse_u16(s: &str) -> Result<u16, String> {
        if !s.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(format!("`{s}` contains non-digit characters"));
        }
        s.parse::<u16>()
            .map_err(|_| format!("failed to parse `{s}`"))
    }
    fn parse_u8(s: &str) -> Result<u8, String> {
        if !s.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(format!("`{s}` contains non-digit characters"));
        }
        s.parse::<u8>()
            .map_err(|_| format!("failed to parse `{s}`"))
    }

    let year = parse_u16(&value[0..4])?;
    let month = parse_u8(&value[5..7])?;
    let day = parse_u8(&value[8..10])?;
    let hour = parse_u8(&value[11..13])?;
    let minute = parse_u8(&value[14..16])?;
    let second = parse_u8(&value[17..19])?;

    if !(1..=12).contains(&month) {
        return Err(format!("month `{month}` is out of range 1..=12"));
    }
    if !(1..=31).contains(&day) {
        return Err(format!("day `{day}` is out of range 1..=31"));
    }
    let max_day = days_in_month(year, month);
    if day > max_day {
        return Err(format!(
            "day `{day}` is out of range 1..={max_day} for month `{month}`"
        ));
    }
    if hour > 23 {
        return Err(format!("hour `{hour}` is out of range 0..=23"));
    }
    if minute > 59 {
        return Err(format!("minute `{minute}` is out of range 0..=59"));
    }
    if second > 59 {
        return Err(format!("second `{second}` is out of range 0..=59"));
    }

    Ok((year, month, day, hour, minute, second))
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn parse_semver_triplet(value: &str) -> Result<(u64, u64, u64), String> {
    let mut parts = value.split('.');
    let major = parts
        .next()
        .ok_or_else(|| "missing major segment".to_string())?
        .parse::<u64>()
        .map_err(|_| "major segment is not numeric".to_string())?;
    let minor = parts
        .next()
        .ok_or_else(|| "missing minor segment".to_string())?
        .parse::<u64>()
        .map_err(|_| "minor segment is not numeric".to_string())?;
    let patch = parts
        .next()
        .ok_or_else(|| "missing patch segment".to_string())?
        .parse::<u64>()
        .map_err(|_| "patch segment is not numeric".to_string())?;
    if parts.next().is_some() {
        return Err("too many semver segments".to_string());
    }
    Ok((major, minor, patch))
}
