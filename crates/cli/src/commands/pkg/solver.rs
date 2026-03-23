fn solve_deterministic_versions(
    catalog: &HashMap<String, Vec<ValidatedPackage>>,
    roots: &[StrictLockRootV1],
    advisories: &[AdvisoryEntry],
    advisory_as_of: Option<UtcTimestamp>,
) -> Result<HashMap<String, ValidatedPackage>, PkgLockError> {
    let mut advisories_by_package: HashMap<String, Vec<AdvisoryEntry>> = HashMap::new();
    for advisory in advisories {
        if let Some(as_of) = advisory_as_of {
            if !(advisory.issued_at <= as_of && as_of < advisory.expires_at) {
                continue;
            }
        }
        advisories_by_package
            .entry(advisory.package.clone())
            .or_default()
            .push(advisory.clone());
    }
    for entries in advisories_by_package.values_mut() {
        entries.sort_by(|a, b| a.id.cmp(&b.id));
    }

    let mut constraints: HashMap<String, Vec<ParsedRequirement>> = HashMap::new();
    for root in roots {
        for dep in &root.dependencies {
            let parsed = ParsedRequirement::parse(dep.requirement.as_str()).map_err(|_| {
                PkgLockError::new(
                    "C111",
                    format!(
                        "root `{}` dependency `{}` has invalid requirement `{}`",
                        root.name, dep.name, dep.requirement
                    ),
                )
            })?;
            constraints
                .entry(dep.name.clone())
                .or_default()
                .push(parsed);
        }
    }
    solve_step(catalog, constraints, HashMap::new(), &advisories_by_package)
}

fn solve_step(
    catalog: &HashMap<String, Vec<ValidatedPackage>>,
    constraints: HashMap<String, Vec<ParsedRequirement>>,
    selected: HashMap<String, ValidatedPackage>,
    advisories_by_package: &HashMap<String, Vec<AdvisoryEntry>>,
) -> Result<HashMap<String, ValidatedPackage>, PkgLockError> {
    let mut unresolved: Vec<&str> = constraints
        .keys()
        .filter(|name| !selected.contains_key(name.as_str()))
        .map(|name| name.as_str())
        .collect();
    unresolved.sort();
    let Some(pkg_name) = unresolved.first().copied() else {
        return Ok(selected);
    };
    let reqs = constraints
        .get(pkg_name)
        .expect("unresolved package should have constraints");
    let candidates = catalog.get(pkg_name).ok_or_else(|| {
        PkgLockError::new(
            "C113",
            format!(
                "deterministic semver solver found no satisfiable version set for `{}`",
                pkg_name
            ),
        )
    })?;

    let matching: Vec<&ValidatedPackage> = candidates
        .iter()
        .filter(|candidate| reqs.iter().all(|req| req.matches(&candidate.semver)))
        .collect();
    if matching.is_empty() {
        return Err(PkgLockError::new(
            "C113",
            format!(
                "deterministic semver solver found no satisfiable version set for `{}` with constraints [{}]",
                pkg_name,
                reqs.iter()
                    .map(|req| req.raw.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }

    let mut allowed_candidates = Vec::new();
    let mut deny_hits: Vec<(String, String, String)> = Vec::new();
    let mut force_hits: Vec<(String, String, String, String)> = Vec::new();
    for candidate in matching {
        if let Some(pkg_advisories) = advisories_by_package.get(pkg_name) {
            let mut denied_by: Option<(&str, &str)> = None;
            let mut forced_by: Option<(&str, &str, SemVer)> = None;
            for advisory in pkg_advisories {
                if !advisory.affected.matches(&candidate.semver) {
                    continue;
                }
                match advisory.action {
                    AdvisoryAction::Deny => {
                        denied_by = Some((advisory.id.as_str(), advisory.severity.as_str()));
                        break;
                    }
                    AdvisoryAction::ForceUpgrade => {
                        let min = advisory
                            .minimum_safe_version
                            .expect("force-upgrade advisories require minimum safe version");
                        if candidate.semver < min {
                            forced_by =
                                Some((advisory.id.as_str(), advisory.severity.as_str(), min));
                            break;
                        }
                    }
                    AdvisoryAction::Warn => {}
                }
            }
            if let Some((advisory_id, severity)) = denied_by {
                deny_hits.push((
                    advisory_id.to_string(),
                    severity.to_string(),
                    candidate.version.clone(),
                ));
                continue;
            }
            if let Some((advisory_id, severity, min_safe)) = forced_by {
                force_hits.push((
                    advisory_id.to_string(),
                    severity.to_string(),
                    candidate.version.clone(),
                    format!("{}.{}.{}", min_safe.major, min_safe.minor, min_safe.patch),
                ));
                continue;
            }
        }
        allowed_candidates.push(candidate);
    }

    if allowed_candidates.is_empty() {
        deny_hits.sort();
        force_hits.sort();
        if let Some((advisory_id, severity, version)) = deny_hits.first() {
            return Err(PkgLockError::new(
                "C115",
                format!(
                    "advisory deny policy rejected package `{}` version `{}` (advisory `{}`, severity `{}`)",
                    pkg_name, version, advisory_id, severity
                ),
            ));
        }
        if let Some((advisory_id, severity, version, min_safe)) = force_hits.first() {
            return Err(PkgLockError::new(
                "C116",
                format!(
                    "advisory forced-upgrade policy could not find compliant version for `{}` (selected `{}` requires >= `{}` via advisory `{}` severity `{}`)",
                    pkg_name, version, min_safe, advisory_id, severity
                ),
            ));
        }
        return Err(PkgLockError::new(
            "C113",
            format!(
                "deterministic semver solver found no satisfiable version set for `{}` with constraints [{}]",
                pkg_name,
                reqs.iter()
                    .map(|req| req.raw.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }

    let mut best_err: Option<PkgLockError> = None;
    for candidate in allowed_candidates {
        let mut next_selected = selected.clone();
        next_selected.insert(pkg_name.to_string(), candidate.clone());
        let mut next_constraints = constraints.clone();
        let mut branch_valid = true;
        for dep in &candidate.dependencies {
            let parsed = ParsedRequirement::parse(dep.requirement.as_str())?;
            next_constraints
                .entry(dep.name.clone())
                .or_default()
                .push(parsed);
            if let Some(existing_dep) = next_selected.get(dep.name.as_str()) {
                let dep_constraints = next_constraints
                    .get(dep.name.as_str())
                    .expect("constraint list should exist for selected dependency");
                if !dep_constraints
                    .iter()
                    .all(|constraint| constraint.matches(&existing_dep.semver))
                {
                    branch_valid = false;
                    break;
                }
            }
        }
        if !branch_valid {
            continue;
        }
        match solve_step(
            catalog,
            next_constraints,
            next_selected,
            advisories_by_package,
        ) {
            Ok(result) => return Ok(result),
            Err(err) => {
                best_err = match best_err {
                    None => Some(err),
                    Some(prev) => Some(prefer_solver_error(prev, err)),
                };
            }
        }
    }

    if let Some(err) = best_err {
        Err(err)
    } else {
        Err(PkgLockError::new(
            "C113",
            format!(
                "deterministic semver solver found no satisfiable version set for `{}` with constraints [{}]",
                pkg_name,
                reqs.iter()
                    .map(|req| req.raw.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ))
    }
}

fn prefer_solver_error(left: PkgLockError, right: PkgLockError) -> PkgLockError {
    let left_rank = solver_error_rank(left.code());
    let right_rank = solver_error_rank(right.code());
    if right_rank < left_rank || (right_rank == left_rank && right.message <= left.message) {
        right
    } else {
        left
    }
}

fn solver_error_rank(code: &str) -> u8 {
    match code {
        "C115" => 0,
        "C116" => 1,
        "C113" => 2,
        _ => 3,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DfsState {
    Visiting,
    Done,
}

fn detect_resolved_cycle(selected: &HashMap<String, ValidatedPackage>) -> Option<String> {
    let mut ids = Vec::with_capacity(selected.len());
    for pkg in selected.values() {
        ids.push(format!("{}@{}", pkg.name, pkg.version));
    }
    ids.sort();

    let mut edges: HashMap<String, Vec<String>> = HashMap::new();
    for pkg in selected.values() {
        let from = format!("{}@{}", pkg.name, pkg.version);
        let mut deps = Vec::new();
        for dep in &pkg.dependencies {
            if let Some(dep_pkg) = selected.get(dep.name.as_str()) {
                deps.push(format!("{}@{}", dep_pkg.name, dep_pkg.version));
            }
        }
        deps.sort();
        edges.insert(from, deps);
    }
    let mut state: HashMap<String, DfsState> = HashMap::new();
    let mut stack = Vec::new();
    let mut stack_set: HashSet<String> = HashSet::new();
    for id in ids {
        if matches!(state.get(id.as_str()), Some(DfsState::Done)) {
            continue;
        }
        if let Some(cycle_ids) =
            dfs_cycle(id.as_str(), &edges, &mut state, &mut stack, &mut stack_set)
        {
            return Some(canonical_cycle_path(cycle_ids));
        }
    }
    None
}

fn dfs_cycle(
    node: &str,
    edges: &HashMap<String, Vec<String>>,
    state: &mut HashMap<String, DfsState>,
    stack: &mut Vec<String>,
    stack_set: &mut HashSet<String>,
) -> Option<Vec<String>> {
    state.insert(node.to_string(), DfsState::Visiting);
    stack.push(node.to_string());
    stack_set.insert(node.to_string());

    if let Some(neighbors) = edges.get(node) {
        for neighbor in neighbors {
            if matches!(state.get(neighbor.as_str()), Some(DfsState::Done)) {
                continue;
            }
            if stack_set.contains(neighbor.as_str()) {
                let start = stack.iter().position(|id| id == neighbor).unwrap_or(0);
                let mut cycle = stack[start..].to_vec();
                cycle.push(neighbor.clone());
                return Some(cycle);
            }
            if let Some(cycle) = dfs_cycle(neighbor, edges, state, stack, stack_set) {
                return Some(cycle);
            }
        }
    }

    stack.pop();
    stack_set.remove(node);
    state.insert(node.to_string(), DfsState::Done);
    None
}

fn canonical_cycle_path(mut cycle_ids: Vec<String>) -> String {
    if cycle_ids.len() <= 2 {
        return cycle_ids.join(" -> ");
    }
    cycle_ids.pop();
    let mut smallest_idx = 0usize;
    for (idx, id) in cycle_ids.iter().enumerate().skip(1) {
        if id < &cycle_ids[smallest_idx] {
            smallest_idx = idx;
        }
    }
    let mut ordered = Vec::with_capacity(cycle_ids.len() + 1);
    ordered.extend(cycle_ids[smallest_idx..].iter().cloned());
    ordered.extend(cycle_ids[..smallest_idx].iter().cloned());
    if let Some(first) = ordered.first().cloned() {
        ordered.push(first);
    }
    ordered.join(" -> ")
}

