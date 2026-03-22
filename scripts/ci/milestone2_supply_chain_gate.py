#!/usr/bin/env python3
import json
import subprocess
import sys
from pathlib import Path


def run_cargo_metadata(root: Path) -> dict:
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


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    out_dir = root / "tmp" / "sbom"
    out_dir.mkdir(parents=True, exist_ok=True)

    metadata = run_cargo_metadata(root)
    dep_report, dep_violations = build_dependency_report(metadata)
    pkg_report, pkg_violations = build_package_artifact_report(root)

    write_json(out_dir / "milestone2-sbom.json", dep_report)
    write_json(out_dir / "milestone2-package-artifact-report.json", pkg_report)
    summary = {
        "schema_version": 1,
        "dependency_violations": len(dep_violations),
        "package_artifact_violations": len(pkg_violations),
        "ok": len(dep_violations) == 0 and len(pkg_violations) == 0,
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

    if dep_violations or pkg_violations:
        return 1
    print("milestone_2 supply-chain gate passed")
    print(f"artifacts written to {out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
