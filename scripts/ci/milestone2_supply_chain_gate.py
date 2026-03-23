#!/usr/bin/env python3
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path


def run_cargo_metadata(root: Path) -> dict:
    metadata_path = os.environ.get("CLG_SUPPLY_CHAIN_METADATA_JSON")
    if metadata_path:
        return json.loads(Path(metadata_path).read_text(encoding="utf-8"))
    proc = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        cwd=root,
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(proc.stdout)


def build_dependency_report(metadata: dict) -> tuple[dict, list[dict]]:
    packages = metadata.get("packages", [])
    package_by_id = {pkg["id"]: pkg for pkg in packages}
    workspace_members = set(metadata.get("workspace_members", []))

    resolve = metadata.get("resolve") or {}
    node_ids = sorted(node["id"] for node in resolve.get("nodes", []))
    if not node_ids:
        node_ids = sorted(package_by_id.keys())

    records = []
    violations = []
    for pkg_id in node_ids:
        pkg = package_by_id[pkg_id]
        license_expr = (pkg.get("license") or "").strip()
        license_file = (pkg.get("license_file") or "").strip()
        in_workspace = pkg_id in workspace_members
        compliant = bool(license_expr or license_file)

        record = {
            "id": pkg_id,
            "name": pkg.get("name"),
            "version": pkg.get("version"),
            "source": pkg.get("source"),
            "license": license_expr or None,
            "license_file": license_file or None,
            "workspace_member": in_workspace,
            "compliant": compliant,
        }
        records.append(record)
        if not compliant:
            violations.append(record)

    report = {
        "schema_version": 1,
        "kind": "cargo_dependency_license_sbom",
        "packages": records,
    }
    return report, violations


def build_package_artifact_report(root: Path) -> tuple[dict, list[dict]]:
    metadata_files: set[Path] = set()
    tracked_override = os.environ.get("CLG_SUPPLY_CHAIN_TRACKED_METADATA")
    if tracked_override:
        for rel in tracked_override.splitlines():
            rel = rel.strip()
            if rel:
                metadata_files.add(root / rel)
    else:
        try:
            tracked = subprocess.run(
                ["git", "ls-files", "--", "**/clg.package-metadata.json"],
                cwd=root,
                capture_output=True,
                text=True,
                check=True,
            )
            for line in tracked.stdout.splitlines():
                rel = line.strip()
                if not rel:
                    continue
                metadata_files.add(root / rel)
        except Exception:
            ignored_roots = {"target", ".git"}
            for path in root.rglob("clg.package-metadata.json"):
                rel_parts = set(path.relative_to(root).parts)
                if rel_parts & ignored_roots:
                    continue
                metadata_files.add(path)
    for path in root.glob("tmp/std-core/**/clg.package-metadata.json"):
        metadata_files.add(path)

    metadata_files = sorted(path for path in metadata_files if path.exists())
    records = []
    violations = []

    for metadata_path in metadata_files:
        data = json.loads(metadata_path.read_text(encoding="utf-8"))
        schema_version = data.get("schema_version")
        packages = data.get("packages") or []
        for pkg in sorted(packages, key=lambda item: (item.get("name", ""), item.get("version", ""))):
            digest = (pkg.get("digest") or "").strip()
            artifact = pkg.get("artifact") or {}
            artifact_path = (artifact.get("path") or "").strip()
            artifact_exists = False
            if artifact_path:
                artifact_exists = (metadata_path.parent / artifact_path).exists()

            compliant = (
                schema_version in (0, 1)
                and digest.startswith("sha256:")
                and bool(artifact_path)
                and artifact_exists
            )
            record = {
                "metadata_file": str(metadata_path.relative_to(root)).replace("\\", "/"),
                "name": pkg.get("name"),
                "version": pkg.get("version"),
                "schema_version": schema_version,
                "digest": digest or None,
                "artifact_path": artifact_path or None,
                "artifact_exists": artifact_exists,
                "compliant": compliant,
            }
            records.append(record)
            if not compliant:
                violations.append(record)

    report = {
        "schema_version": 1,
        "kind": "package_metadata_artifact_compliance",
        "entries": records,
    }
    return report, violations


def sha256_hex(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        while True:
            chunk = fh.read(65536)
            if not chunk:
                break
            h.update(chunk)
    return h.hexdigest()


def collect_runtime_link_files(root: Path) -> list[Path]:
    runtime_files: set[Path] = set()
    runtime_override = os.environ.get("CLG_SUPPLY_CHAIN_RUNTIME_LINKS")
    if runtime_override:
        for rel in runtime_override.splitlines():
            rel = rel.strip()
            if rel:
                runtime_files.add(root / rel)
    else:
        try:
            tracked = subprocess.run(
                ["git", "ls-files", "--", "**/clg.runtime-link.json"],
                cwd=root,
                capture_output=True,
                text=True,
                check=True,
            )
            for line in tracked.stdout.splitlines():
                rel = line.strip()
                if rel:
                    runtime_files.add(root / rel)
        except Exception:
            pass
    for path in root.glob("tmp/perf/**/clg.runtime-link.json"):
        runtime_files.add(path)
    return sorted(path for path in runtime_files if path.exists())


def build_runtime_dependency_report(root: Path) -> tuple[dict, list[dict]]:
    runtime_link_files = collect_runtime_link_files(root)
    records: list[dict] = []
    violations: list[dict] = []

    if not runtime_link_files:
        record = {
            "entry_kind": "runtime_link_presence",
            "runtime_link_file": None,
            "compliant": False,
            "reason": "missing_runtime_link_artifacts",
        }
        records.append(record)
        violations.append(record)

    for runtime_link_path in runtime_link_files:
        runtime_data = json.loads(runtime_link_path.read_text(encoding="utf-8"))
        schema_version = runtime_data.get("schema_version")
        packages = runtime_data.get("packages") or []
        bindings = runtime_data.get("bindings") or []

        lock_path = runtime_link_path.parent / "clg.lock.json"
        lock_packages: dict[str, str] = {}
        if lock_path.exists():
            lock_data = json.loads(lock_path.read_text(encoding="utf-8"))
            for pkg in lock_data.get("packages") or []:
                package_id = (pkg.get("id") or "").strip()
                digest = (pkg.get("digest") or "").strip()
                if package_id:
                    lock_packages[package_id] = digest

        store_index_path = runtime_link_path.parent / "clg.package-store-index.json"
        store_artifacts: dict[str, str] = {}
        if store_index_path.exists():
            store_data = json.loads(store_index_path.read_text(encoding="utf-8"))
            for artifact in store_data.get("artifacts") or []:
                package_id = (artifact.get("id") or "").strip()
                digest = (artifact.get("digest") or "").strip()
                if package_id:
                    store_artifacts[package_id] = digest

        package_ids = {
            (pkg.get("id") or "").strip()
            for pkg in packages
            if (pkg.get("id") or "").strip()
        }
        for pkg in sorted(packages, key=lambda item: item.get("id", "")):
            package_id = (pkg.get("id") or "").strip()
            digest = (pkg.get("digest") or "").strip()
            artifact_path = (pkg.get("artifact_path") or "").strip()
            artifact_exists = False
            digest_matches_artifact = False
            if artifact_path:
                artifact_full_path = runtime_link_path.parent / artifact_path
                artifact_exists = artifact_full_path.exists()
                if artifact_exists and digest.startswith("sha256:"):
                    digest_matches_artifact = sha256_hex(artifact_full_path) == digest[7:]

            lock_digest_matches = bool(
                package_id and lock_packages.get(package_id) == digest and lock_path.exists()
            )
            store_digest_matches = bool(
                package_id
                and store_artifacts.get(package_id) == digest
                and store_index_path.exists()
            )
            compliant = (
                schema_version in (0, 1)
                and bool(package_id)
                and digest.startswith("sha256:")
                and bool(artifact_path)
                and artifact_exists
                and digest_matches_artifact
                and lock_digest_matches
                and store_digest_matches
            )

            record = {
                "entry_kind": "runtime_package",
                "runtime_link_file": str(runtime_link_path.relative_to(root)).replace("\\", "/"),
                "lockfile_present": lock_path.exists(),
                "store_index_present": store_index_path.exists(),
                "schema_version": schema_version,
                "package_id": package_id or None,
                "digest": digest or None,
                "artifact_path": artifact_path or None,
                "artifact_exists": artifact_exists,
                "digest_matches_artifact": digest_matches_artifact,
                "lock_digest_matches": lock_digest_matches,
                "store_digest_matches": store_digest_matches,
                "compliant": compliant,
            }
            records.append(record)
            if not compliant:
                violations.append(record)

        for binding in sorted(
            bindings,
            key=lambda item: (
                item.get("import_module", ""),
                item.get("import_name", ""),
                item.get("provider_package_id", ""),
            ),
        ):
            provider_id = (binding.get("provider_package_id") or "").strip()
            compliant = bool(provider_id) and provider_id in package_ids
            record = {
                "entry_kind": "runtime_binding",
                "runtime_link_file": str(runtime_link_path.relative_to(root)).replace("\\", "/"),
                "import_module": binding.get("import_module"),
                "import_name": binding.get("import_name"),
                "provider_package_id": provider_id or None,
                "compliant": compliant,
            }
            records.append(record)
            if not compliant:
                violations.append(record)

    report = {
        "schema_version": 1,
        "kind": "runtime_dependency_compliance",
        "entries": records,
    }
    return report, violations


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    root_env = os.environ.get("CLG_SUPPLY_CHAIN_ROOT")
    if root_env:
        root = Path(root_env).resolve()
    else:
        root = Path(__file__).resolve().parents[2]
    out_dir_env = os.environ.get("CLG_SUPPLY_CHAIN_OUT_DIR")
    if out_dir_env:
        out_dir = Path(out_dir_env).resolve()
    else:
        out_dir = root / "tmp" / "sbom"
    out_dir.mkdir(parents=True, exist_ok=True)

    metadata = run_cargo_metadata(root)
    dep_report, dep_violations = build_dependency_report(metadata)
    pkg_report, pkg_violations = build_package_artifact_report(root)
    runtime_report, runtime_violations = build_runtime_dependency_report(root)

    write_json(out_dir / "milestone2-sbom.json", dep_report)
    write_json(out_dir / "milestone2-package-artifact-report.json", pkg_report)
    write_json(out_dir / "milestone2-runtime-dependency-report.json", runtime_report)
    summary = {
        "schema_version": 1,
        "dependency_violations": len(dep_violations),
        "package_artifact_violations": len(pkg_violations),
        "runtime_dependency_violations": len(runtime_violations),
        "ok": len(dep_violations) == 0
        and len(pkg_violations) == 0
        and len(runtime_violations) == 0,
    }
    write_json(out_dir / "milestone2-supply-chain-summary.json", summary)

    if dep_violations:
        print("dependency license compliance violations detected:", file=sys.stderr)
        for item in dep_violations[:10]:
            print(
                f"- {item['name']} {item['version']} missing license/license_file",
                file=sys.stderr,
            )
    if pkg_violations:
        print("package metadata artifact compliance violations detected:", file=sys.stderr)
        for item in pkg_violations[:10]:
            print(
                f"- {item['metadata_file']}::{item['name']} {item['version']} "
                f"(digest={item['digest']}, artifact_path={item['artifact_path']}, "
                f"artifact_exists={item['artifact_exists']})",
                file=sys.stderr,
            )
    if runtime_violations:
        print("runtime dependency compliance violations detected:", file=sys.stderr)
        for item in runtime_violations[:10]:
            print(f"- {json.dumps(item, sort_keys=True)}", file=sys.stderr)

    if dep_violations or pkg_violations or runtime_violations:
        return 1
    print("milestone_2 supply-chain gate passed")
    print(f"artifacts written to {out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
