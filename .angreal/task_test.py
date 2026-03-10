"""Test commands for Arawn."""

import os
import subprocess

import angreal
from angreal.integrations.flox import Flox

test = angreal.command_group(name="test", about="Run tests")


@test()
@angreal.command(name="all", about="Run all tests (workspace + runtimes)")
def test_all():
    """Run workspace tests then runtime tests."""
    with Flox("."):
        _run_unit()
        _run_runtimes()


@test()
@angreal.command(name="unit", about="Run workspace unit tests")
def test_unit():
    """Run cargo test across the workspace."""
    with Flox("."):
        _run_unit()


@test()
@angreal.command(name="runtimes", about="Run runtime tests individually")
def test_runtimes():
    """Run tests for each WASM runtime crate."""
    with Flox("."):
        _run_runtimes()


@test()
@angreal.command(name="integration", about="Run integration tests (ignored tests)")
def test_integration():
    """Run tests marked with #[ignore]."""
    with Flox("."):
        subprocess.run(
            ["cargo", "test", "--workspace", "--", "--ignored", "--test-threads=1"],
            check=True,
        )


@test()
@angreal.command(name="coverage", about="Generate code coverage report")
@angreal.argument(
    name="open_report",
    long="open",
    is_flag=True,
    takes_value=False,
    help="Open HTML report in browser after generation",
)
def test_coverage(open_report=False):
    """Generate code coverage report with branch coverage using cargo-llvm-cov.

    Uses nightly toolchain for branch coverage support. Generates per-crate
    reports to work around an LLVM bug with --branch on large workspaces,
    then produces a combined HTML report without --branch for browsing.
    """
    with Flox("."):
        # Generate per-crate branch coverage (nightly required for --branch)
        crates = _find_workspace_crates()
        combined_lcov = os.path.join("coverage", "branch-combined.info")
        os.makedirs("coverage", exist_ok=True)

        # Clear previous combined file
        if os.path.exists(combined_lcov):
            os.remove(combined_lcov)

        print("=== Generating per-crate branch coverage ===\n")
        for crate in crates:
            lcov_path = os.path.join("coverage", f"branch-{crate}.info")
            result = subprocess.run(
                [
                    "cargo", "+nightly", "llvm-cov",
                    "-p", crate,
                    "--branch", "--lcov",
                    "--output-path", lcov_path,
                ],
                capture_output=True,
            )
            if result.returncode == 0 and os.path.exists(lcov_path):
                with open(combined_lcov, "a") as combined, open(lcov_path) as src:
                    combined.write(src.read())
                print(f"  {crate}: OK")
            else:
                print(f"  {crate}: SKIPPED (nightly compile issue)")

        # Print branch summary from LCOV data
        _print_branch_summary(combined_lcov)

        # Generate HTML report (stable, without --branch to avoid LLVM crash)
        print("\n=== Generating HTML report ===\n")
        subprocess.run(
            [
                "cargo", "llvm-cov",
                "--workspace", "--html",
                "--output-dir", "coverage/",
                "--", "--test-threads=1",
            ],
            check=True,
        )
        print("\nCoverage report generated in coverage/html/index.html")
        print(f"Branch coverage LCOV data in {combined_lcov}")
        if open_report:
            subprocess.run(["open", "coverage/html/index.html"], check=False)


def _find_workspace_crates():
    """Find all workspace crate names from crates/ directory."""
    crates_dir = os.path.join(os.getcwd(), "crates")
    crates = []
    for entry in sorted(os.listdir(crates_dir)):
        cargo_toml = os.path.join(crates_dir, entry, "Cargo.toml")
        if os.path.exists(cargo_toml):
            crates.append(entry)
    return crates


def _print_branch_summary(lcov_path):
    """Parse LCOV file and print branch coverage summary."""
    if not os.path.exists(lcov_path):
        print("\nNo branch coverage data generated.")
        return

    print("\n=== Branch Coverage Summary ===\n")
    current_file = None
    crate_stats = {}

    with open(lcov_path) as f:
        for line in f:
            line = line.strip()
            if line.startswith("SF:"):
                current_file = line[3:]
                # Extract crate name from path
                parts = current_file.split("/crates/")
                if len(parts) > 1:
                    crate_name = parts[1].split("/")[0]
                else:
                    crate_name = "other"
                if crate_name not in crate_stats:
                    crate_stats[crate_name] = {"br_total": 0, "br_hit": 0, "ln_total": 0, "ln_hit": 0}
            elif line.startswith("BRDA:"):
                parts = line[5:].split(",")
                if len(parts) >= 4 and crate_name in crate_stats:
                    crate_stats[crate_name]["br_total"] += 1
                    if parts[3] not in ("0", "-"):
                        crate_stats[crate_name]["br_hit"] += 1
            elif line.startswith("LF:"):
                if crate_name in crate_stats:
                    crate_stats[crate_name]["ln_total"] += int(line[3:])
            elif line.startswith("LH:"):
                if crate_name in crate_stats:
                    crate_stats[crate_name]["ln_hit"] += int(line[3:])

    total_br = total_br_hit = total_ln = total_ln_hit = 0
    for crate_name in sorted(crate_stats):
        s = crate_stats[crate_name]
        total_br += s["br_total"]
        total_br_hit += s["br_hit"]
        total_ln += s["ln_total"]
        total_ln_hit += s["ln_hit"]
        ln_pct = f"{s['ln_hit'] * 100 / s['ln_total']:.1f}" if s["ln_total"] > 0 else "N/A"
        br_pct = f"{s['br_hit'] * 100 / s['br_total']:.1f}" if s["br_total"] > 0 else "N/A"
        print(f"  {crate_name:<25s} Lines: {s['ln_hit']:>4d}/{s['ln_total']:<4d} ({ln_pct:>5s}%)  Branches: {s['br_hit']:>4d}/{s['br_total']:<4d} ({br_pct:>5s}%)")

    if total_br > 0:
        ln_pct = f"{total_ln_hit * 100 / total_ln:.1f}" if total_ln > 0 else "N/A"
        br_pct = f"{total_br_hit * 100 / total_br:.1f}"
        print(f"\n  {'TOTAL':<25s} Lines: {total_ln_hit:>4d}/{total_ln:<4d} ({ln_pct:>5s}%)  Branches: {total_br_hit:>4d}/{total_br:<4d} ({br_pct:>5s}%)")


def _run_unit():
    subprocess.run(
        ["cargo", "test", "--workspace", "--", "--test-threads=1"],
        check=True,
    )


def _run_runtimes():
    runtimes_dir = os.path.join(os.getcwd(), "runtimes")
    for entry in sorted(os.listdir(runtimes_dir)):
        runtime_path = os.path.join(runtimes_dir, entry)
        if os.path.isdir(runtime_path) and os.path.exists(
            os.path.join(runtime_path, "Cargo.toml")
        ):
            print(f"\n--- Testing runtime: {entry} ---")
            subprocess.run(["cargo", "test"], cwd=runtime_path, check=True)
