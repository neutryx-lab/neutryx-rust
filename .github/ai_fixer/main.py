#!/usr/bin/env python3
"""
AI Fixer CLI - Self-healing CI for neutryx-rust.

Usage:
    python -m ai_fixer --log failed_log.txt
    python -m ai_fixer --log failed_log.txt --output fix.patch
    python -m ai_fixer --log failed_log.txt --create-pr
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

from .parser import CILogParser, FailureType
from .context import ProjectContext
from .client import GeminiClient, FixResponse
from .patch import PatchGenerator, format_fix_as_pr_comment


def find_project_root() -> Path:
    """Find the project root by looking for Cargo.toml."""
    current = Path.cwd()
    while current != current.parent:
        if (current / "Cargo.toml").exists():
            return current
        current = current.parent

    # Fallback to script location
    return Path(__file__).parent.parent.parent


def main() -> int:
    parser = argparse.ArgumentParser(
        description="AI-powered CI failure fixer for neutryx-rust",
    )
    parser.add_argument(
        "--log",
        type=Path,
        required=True,
        help="Path to the CI failure log file",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="Output path for the generated patch (default: stdout)",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output as JSON including metadata",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Parse and analyse without generating fix",
    )
    parser.add_argument(
        "--create-pr",
        action="store_true",
        help="Create a PR with the fix (requires gh CLI)",
    )
    parser.add_argument(
        "--use-flash",
        action="store_true",
        help="Use Gemini Flash model (faster, less accurate)",
    )
    parser.add_argument(
        "--verbose",
        "-v",
        action="store_true",
        help="Verbose output",
    )

    args = parser.parse_args()

    # Validate log file
    if not args.log.exists():
        print(f"Error: Log file not found: {args.log}", file=sys.stderr)
        return 1

    # Find project root
    project_root = find_project_root()
    if args.verbose:
        print(f"Project root: {project_root}")

    # Parse CI log
    log_content = args.log.read_text(encoding="utf-8", errors="replace")
    log_parser = CILogParser(log_content)
    failures = log_parser.parse()

    if not failures:
        print("No failures detected in log file.")
        return 0

    if args.verbose:
        print(f"Detected {len(failures)} failure(s):")
        for i, f in enumerate(failures, 1):
            print(f"  {i}. {f.failure_type.name}: {f.message[:80]}")

    if args.dry_run:
        # Just output analysis
        for failure in failures:
            print(failure.to_prompt_context())
            print()
        return 0

    # Collect project context
    affected_paths = []
    for failure in failures:
        for loc in failure.locations:
            affected_paths.append(loc.file_path)

    context = ProjectContext(project_root=project_root)
    for path in affected_paths:
        context.add_affected_file(path)

    # Generate fix
    try:
        client = GeminiClient(use_flash=args.use_flash)
    except ValueError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1

    if args.verbose:
        print("Generating fix with Gemini API...")

    try:
        fix_response = client.generate_fix(failures, context)
    except Exception as e:
        print(f"Error generating fix: {e}", file=sys.stderr)
        return 1

    # Validate patch
    patch_gen = PatchGenerator(project_root)
    is_valid, validation_errors = patch_gen.validate_patch(fix_response.patch)

    if not is_valid:
        print("Generated patch has validation errors:", file=sys.stderr)
        for err in validation_errors:
            print(f"  - {err}", file=sys.stderr)

    # Output
    if args.json:
        output = {
            "explanation": fix_response.explanation,
            "confidence": fix_response.confidence,
            "files_modified": fix_response.files_modified,
            "requires_review": fix_response.requires_review,
            "patch": fix_response.patch,
            "validation_errors": validation_errors,
            "failures_count": len(failures),
            "failure_types": [f.failure_type.name for f in failures],
        }
        output_text = json.dumps(output, indent=2)
    else:
        output_text = fix_response.patch

    if args.output:
        args.output.write_text(output_text, encoding="utf-8")
        if args.verbose:
            print(f"Patch written to: {args.output}")
    elif not args.create_pr:
        print(output_text)

    # Create PR if requested
    if args.create_pr:
        return create_fix_pr(
            project_root,
            fix_response,
            failures,
            verbose=args.verbose,
        )

    return 0


def create_fix_pr(
    project_root: Path,
    fix_response: FixResponse,
    failures: list,
    verbose: bool = False,
) -> int:
    """Create a GitHub PR with the generated fix."""
    import datetime

    # Generate branch name
    timestamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    failure_type = failures[0].failure_type.name.lower() if failures else "fix"
    branch_name = f"ai-fix/{failure_type}-{timestamp}"

    patch_gen = PatchGenerator(project_root)

    # Create fix branch
    commit_msg = f"fix: AI-generated fix for CI {failure_type} failure\n\nAuto-generated by AI Fixer.\nConfidence: {fix_response.confidence:.0%}"

    success, result = patch_gen.create_fix_branch(
        branch_name,
        fix_response.patch,
        commit_msg,
    )

    if not success:
        print(f"Error creating fix branch: {result}", file=sys.stderr)
        return 1

    if verbose:
        print(f"Created branch: {branch_name}")

    # Push branch
    try:
        subprocess.run(
            ["git", "push", "-u", "origin", branch_name],
            cwd=project_root,
            capture_output=True,
            check=True,
        )
    except subprocess.CalledProcessError as e:
        print(f"Error pushing branch: {e.stderr}", file=sys.stderr)
        return 1

    # Create PR using gh CLI
    failures_summary = "\n".join(
        f"- **{f.failure_type.name}**: {f.message[:100]}"
        for f in failures[:5]
    )

    pr_body = format_fix_as_pr_comment(fix_response, failures_summary)

    try:
        result = subprocess.run(
            [
                "gh", "pr", "create",
                "--draft",
                "--title", f"[AI Fix] Resolve CI {failure_type} failure",
                "--body", pr_body,
            ],
            cwd=project_root,
            capture_output=True,
            text=True,
            check=True,
        )
        print(f"PR created: {result.stdout.strip()}")
    except subprocess.CalledProcessError as e:
        print(f"Error creating PR: {e.stderr}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
