#!/usr/bin/env bash

doctor_fixture_healthy_not_needed() {
    cat <<'EOF'
{
  "activeTupleId": "tuple-active",
  "buildManifest": {"builder_sha256": "builderhash", "changed_byte_count": 42, "patch_policy": "dns-fd33-only-v1"},
  "checks": {
    "build_manifest": true,
    "bundled_bwrap": true,
    "bwrap_exec": true,
    "cert": true,
    "current_in_store": true,
    "current_pointer": true,
    "current_verified_match": true,
    "dns_only_patch": true,
    "manager": true,
    "network_boundary": true,
    "path_bwrap": true,
    "raw": true,
    "raw_hash": true,
    "raw_in_store": true,
    "raw_pointer": true,
    "registry": true,
    "registry_active_tuple": true,
    "registry_current_match": true,
    "registry_verified_match": true,
    "resolv": true,
    "rg": true,
    "rg_exec": true,
    "rg_real": true,
    "runtime": true,
    "runtime_hash": true,
    "runtime_store": true,
    "raw_store": true,
    "state": true,
    "support_bwrap_match": true,
    "support_rg_match": true,
    "verified_in_store": true,
    "verified_pointer": true,
    "zsh": true
  },
  "migration": {
    "imported": [],
    "legacyStore": "/legacy-store",
    "report": "/migration-report.json",
    "skipped": [],
    "status": "not-needed"
  },
  "networkBoundary": {
    "checks": {
      "baseline_socket": true,
      "network_off": true,
      "network_on": true,
      "network_reset": true
    },
    "overallStatus": "ok"
  },
  "overallStatus": "ok",
  "paths": {
    "current": "/current",
    "current_target": "/store/runtime/current",
    "manager": "/manager",
    "raw": "/raw",
    "raw_store": "/store/raw",
    "raw_target": "/store/raw/current",
    "raw_vendor": "/raw/vendor",
    "registry": "/state/registry.json",
    "runtime": "/runtime",
    "runtime_store": "/store/runtime",
    "state": "/state/state.json",
    "verified": "/verified",
    "verified_target": "/store/runtime/current"
  },
  "raw_sha256": "rawhash",
  "runtime_sha256": "runtimehash",
  "verifiedTupleId": "tuple-verified",
  "version": "test"
}
EOF
}

doctor_fixture_healthy_issues() {
    cat <<'EOF'
{
  "activeTupleId": "tuple-active",
  "buildManifest": {"builder_sha256": "builderhash", "changed_byte_count": 42, "patch_policy": "dns-fd33-only-v1"},
  "checks": {
    "build_manifest": true,
    "bundled_bwrap": true,
    "bwrap_exec": true,
    "cert": true,
    "current_in_store": true,
    "current_pointer": true,
    "current_verified_match": true,
    "dns_only_patch": true,
    "manager": true,
    "network_boundary": true,
    "path_bwrap": true,
    "raw": true,
    "raw_hash": true,
    "raw_in_store": true,
    "raw_pointer": true,
    "registry": true,
    "registry_active_tuple": true,
    "registry_current_match": true,
    "registry_verified_match": true,
    "resolv": true,
    "rg": true,
    "rg_exec": true,
    "rg_real": true,
    "runtime": true,
    "runtime_hash": true,
    "runtime_store": true,
    "raw_store": true,
    "state": true,
    "support_bwrap_match": true,
    "support_rg_match": true,
    "verified_in_store": true,
    "verified_pointer": true,
    "zsh": true
  },
  "migration": {
    "imported": [],
    "legacyStore": "/legacy-store",
    "report": "/migration-report.json",
    "skipped": [{"reason": "raw source is outside legacy raw store", "tuple_id": "tuple-active"}],
    "status": "issues"
  },
  "networkBoundary": {
    "checks": {
      "baseline_socket": true,
      "network_off": true,
      "network_on": true,
      "network_reset": true
    },
    "overallStatus": "ok"
  },
  "overallStatus": "ok",
  "paths": {
    "current": "/current",
    "current_target": "/store/runtime/current",
    "manager": "/manager",
    "raw": "/raw",
    "raw_store": "/store/raw",
    "raw_target": "/store/raw/current",
    "raw_vendor": "/raw/vendor",
    "registry": "/state/registry.json",
    "runtime": "/runtime",
    "runtime_store": "/store/runtime",
    "state": "/state/state.json",
    "verified": "/verified",
    "verified_target": "/store/runtime/current"
  },
  "raw_sha256": "rawhash",
  "runtime_sha256": "runtimehash",
  "verifiedTupleId": "tuple-verified",
  "version": "test"
}
EOF
}

doctor_fixture_broken_current() {
    cat <<'EOF'
{
  "activeTupleId": "tuple-active",
  "buildManifest": {"builder_sha256": "builderhash", "changed_byte_count": 42, "patch_policy": "dns-fd33-only-v1"},
  "checks": {
    "build_manifest": true,
    "bundled_bwrap": true,
    "bwrap_exec": true,
    "cert": true,
    "current_in_store": false,
    "current_pointer": false,
    "current_verified_match": true,
    "dns_only_patch": true,
    "manager": true,
    "network_boundary": true,
    "path_bwrap": true,
    "raw": true,
    "raw_hash": true,
    "raw_in_store": true,
    "raw_pointer": true,
    "registry": true,
    "registry_active_tuple": true,
    "registry_current_match": false,
    "registry_verified_match": true,
    "resolv": true,
    "rg": true,
    "rg_exec": true,
    "rg_real": true,
    "runtime": true,
    "runtime_hash": true,
    "runtime_store": true,
    "raw_store": true,
    "state": true,
    "support_bwrap_match": true,
    "support_rg_match": true,
    "verified_in_store": true,
    "verified_pointer": true,
    "zsh": true
  },
  "migration": {
    "imported": ["tuple-active"],
    "legacyStore": "/legacy-store",
    "report": "/migration-report.json",
    "skipped": [],
    "status": "completed"
  },
  "networkBoundary": {
    "checks": {
      "baseline_socket": true,
      "network_off": true,
      "network_on": true,
      "network_reset": true
    },
    "overallStatus": "ok"
  },
  "overallStatus": "fail",
  "paths": {
    "current": "/current",
    "current_target": "/broken/current",
    "manager": "/manager",
    "raw": "/raw",
    "raw_store": "/store/raw",
    "raw_target": "/store/raw/current",
    "raw_vendor": "/raw/vendor",
    "registry": "/state/registry.json",
    "runtime": "/runtime",
    "runtime_store": "/store/runtime",
    "state": "/state/state.json",
    "verified": "/verified",
    "verified_target": "/store/runtime/current"
  },
  "raw_sha256": "rawhash",
  "runtime_sha256": "runtimehash",
  "verifiedTupleId": "tuple-verified",
  "version": "test"
}
EOF
}
