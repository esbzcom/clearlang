#!/usr/bin/env python3
import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


def load_gate_module():
    module_path = Path(__file__).resolve().with_name("milestone2_supply_chain_gate.py")
    spec = importlib.util.spec_from_file_location("milestone2_supply_chain_gate", module_path)
    if spec is None or spec.loader is None:
        raise RuntimeError("failed to load milestone2_supply_chain_gate module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


gate = load_gate_module()


class Milestone2SupplyChainGateTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def write_json(self, path: Path, value: dict) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")

    def write_runtime_loader_fixture(self, base_dir: Path, digest: str | None = None) -> Path:
        runtime_link_path = base_dir / "clg.runtime-link.json"
        artifact_path = base_dir / "store" / "pkg-a.wasm"
        artifact_path.parent.mkdir(parents=True, exist_ok=True)
        artifact_path.write_bytes(b"\x00asm\x01\x00\x00\x00")
        if digest is None:
            digest = gate.sha256_hex(artifact_path)
        digest_value = f"sha256:{digest}"

        self.write_json(
            runtime_link_path,
            {
                "schema_version": 0,
                "packages": [
                    {
                        "id": "pkg::a@1.0.0",
                        "digest": digest_value,
                        "artifact_path": "store/pkg-a.wasm",
                    }
                ],
                "bindings": [
                    {
                        "import_module": "pkg::a",
                        "import_name": "add",
                        "provider_package_id": "pkg::a@1.0.0",
                    }
                ],
            },
        )
        self.write_json(
            base_dir / "clg.lock.json",
            {
                "schema_version": 1,
                "packages": [
                    {
                        "id": "pkg::a@1.0.0",
                        "digest": digest_value,
                    }
                ],
            },
        )
        self.write_json(
            base_dir / "clg.package-store-index.json",
            {
                "schema_version": 0,
                "artifacts": [
                    {
                        "id": "pkg::a@1.0.0",
                        "digest": digest_value,
                        "path": "store/pkg-a.wasm",
                    }
                ],
            },
        )
        return runtime_link_path

    def test_build_dependency_report_flags_missing_license(self):
        metadata = {
            "packages": [
                {
                    "id": "pkg_a 1.0.0 (path+file:///pkg_a)",
                    "name": "pkg_a",
                    "version": "1.0.0",
                    "license": "",
                    "license_file": "",
                },
                {
                    "id": "pkg_b 1.0.0 (path+file:///pkg_b)",
                    "name": "pkg_b",
                    "version": "1.0.0",
                    "license": "MIT",
                    "license_file": "",
                },
            ],
            "workspace_members": ["pkg_a 1.0.0 (path+file:///pkg_a)"],
            "resolve": {
                "nodes": [
                    {"id": "pkg_a 1.0.0 (path+file:///pkg_a)"},
                    {"id": "pkg_b 1.0.0 (path+file:///pkg_b)"},
                ]
            },
        }
        report, violations = gate.build_dependency_report(metadata)
        self.assertEqual(report["schema_version"], 1)
        self.assertEqual(len(report["packages"]), 2)
        self.assertEqual(len(violations), 1)
        self.assertEqual(violations[0]["name"], "pkg_a")

    def test_build_package_artifact_report_respects_tracked_override(self):
        metadata_path = self.root / "fixtures" / "clg.package-metadata.json"
        artifact_rel = "artifact/pkg-a.wasm"
        (metadata_path.parent / "artifact").mkdir(parents=True)
        (metadata_path.parent / artifact_rel).write_bytes(b"\x00asm\x01\x00\x00\x00")
        self.write_json(
            metadata_path,
            {
                "schema_version": 1,
                "packages": [
                    {
                        "name": "pkg::a",
                        "version": "1.0.0",
                        "digest": "sha256:abc",
                        "artifact": {"path": artifact_rel},
                    }
                ],
            },
        )
        rel_path = str(metadata_path.relative_to(self.root)).replace("\\", "/")
        with patch.dict(
            os.environ,
            {"CLG_SUPPLY_CHAIN_TRACKED_METADATA": rel_path},
            clear=False,
        ):
            report, violations = gate.build_package_artifact_report(self.root)
        self.assertEqual(report["schema_version"], 1)
        self.assertEqual(len(report["entries"]), 1)
        self.assertEqual(len(violations), 0)

    def test_main_writes_summary_and_fails_closed_on_violation(self):
        metadata_path = self.root / "metadata.json"
        package_metadata_path = self.root / "fixtures" / "clg.package-metadata.json"
        artifact_rel = "artifact/pkg-a.wasm"
        (package_metadata_path.parent / "artifact").mkdir(parents=True)
        (package_metadata_path.parent / artifact_rel).write_bytes(b"\x00asm\x01\x00\x00\x00")
        self.write_json(
            package_metadata_path,
            {
                "schema_version": 1,
                "packages": [
                    {
                        "name": "pkg::a",
                        "version": "1.0.0",
                        "digest": "sha256:abc",
                        "artifact": {"path": artifact_rel},
                    }
                ],
            },
        )
        rel_path = str(package_metadata_path.relative_to(self.root)).replace("\\", "/")
        out_dir = self.root / "out"
        runtime_fixture = self.root / "tmp" / "perf" / "runtime_loader"
        runtime_link_path = self.write_runtime_loader_fixture(runtime_fixture)

        self.write_json(
            metadata_path,
            {
                "packages": [
                    {
                        "id": "pkg_a 1.0.0 (path+file:///pkg_a)",
                        "name": "pkg_a",
                        "version": "1.0.0",
                        "license": "",
                        "license_file": "",
                    }
                ],
                "workspace_members": ["pkg_a 1.0.0 (path+file:///pkg_a)"],
                "resolve": {"nodes": [{"id": "pkg_a 1.0.0 (path+file:///pkg_a)"}]},
            },
        )

        env = {
            "CLG_SUPPLY_CHAIN_ROOT": str(self.root),
            "CLG_SUPPLY_CHAIN_OUT_DIR": str(out_dir),
            "CLG_SUPPLY_CHAIN_METADATA_JSON": str(metadata_path),
            "CLG_SUPPLY_CHAIN_TRACKED_METADATA": rel_path,
            "CLG_SUPPLY_CHAIN_RUNTIME_LINKS": str(
                runtime_link_path.relative_to(self.root)
            ).replace("\\", "/"),
        }

        with patch.dict(os.environ, env, clear=False):
            rc = gate.main()
        self.assertEqual(rc, 1)
        summary = json.loads(
            (out_dir / "milestone2-supply-chain-summary.json").read_text(encoding="utf-8")
        )
        self.assertFalse(summary["ok"])
        self.assertGreater(summary["dependency_violations"], 0)

        self.write_json(
            metadata_path,
            {
                "packages": [
                    {
                        "id": "pkg_a 1.0.0 (path+file:///pkg_a)",
                        "name": "pkg_a",
                        "version": "1.0.0",
                        "license": "MIT",
                        "license_file": "",
                    }
                ],
                "workspace_members": ["pkg_a 1.0.0 (path+file:///pkg_a)"],
                "resolve": {"nodes": [{"id": "pkg_a 1.0.0 (path+file:///pkg_a)"}]},
            },
        )
        with patch.dict(os.environ, env, clear=False):
            rc = gate.main()
        self.assertEqual(rc, 0)
        summary = json.loads(
            (out_dir / "milestone2-supply-chain-summary.json").read_text(encoding="utf-8")
        )
        self.assertTrue(summary["ok"])
        self.assertEqual(summary["dependency_violations"], 0)
        self.assertEqual(summary["package_artifact_violations"], 0)
        self.assertEqual(summary["runtime_dependency_violations"], 0)

    def test_runtime_dependency_report_fails_closed_on_digest_mismatch(self):
        runtime_fixture = self.root / "tmp" / "perf" / "runtime_loader"
        runtime_link_path = self.write_runtime_loader_fixture(
            runtime_fixture,
            "0000000000000000000000000000000000000000000000000000000000000000",
        )
        with patch.dict(
            os.environ,
            {
                "CLG_SUPPLY_CHAIN_RUNTIME_LINKS": str(
                    runtime_link_path.relative_to(self.root)
                ).replace("\\", "/")
            },
            clear=False,
        ):
            report, violations = gate.build_runtime_dependency_report(self.root)
        self.assertEqual(report["schema_version"], 1)
        self.assertGreater(len(report["entries"]), 0)
        self.assertGreater(len(violations), 0)
        self.assertTrue(
            any(item.get("entry_kind") == "runtime_package" for item in violations)
        )


if __name__ == "__main__":
    unittest.main()
