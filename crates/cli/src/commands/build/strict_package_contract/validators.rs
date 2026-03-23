fn validate_signature_hex(signature: &str) -> Result<(), String> {
    if signature.len() != 128 {
        return Err("signature must have exactly 128 lowercase hex characters".to_string());
    }
    if !signature
        .chars()
        .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    {
        return Err("signature must be lowercase hex (`0-9`, `a-f`)".to_string());
    }
    Ok(())
}

fn validate_relative_artifact_path(path: &str) -> Result<(), String> {
    let artifact_path = Path::new(path);
    if artifact_path.is_absolute() {
        return Err("artifact path must be relative".to_string());
    }
    for component in artifact_path.components() {
        match component {
            Component::ParentDir => {
                return Err(
                    "artifact path must not contain parent-directory traversal (`..`)".to_string(),
                )
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err("artifact path must be relative".to_string())
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    Ok(())
}

