"""
Patch generation and application utilities.

Handles:
- Unified diff generation
- Patch validation
- Safe patch application
"""

from __future__ import annotations

import re
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .client import FixResponse


@dataclass
class PatchResult:
    """Result of patch application."""

    success: bool
    files_patched: list[str]
    errors: list[str]
    backup_created: bool = False


class PatchGenerator:
    """Generates and applies code patches."""

    def __init__(self, project_root: Path):
        self.project_root = project_root

    def validate_patch(self, patch_content: str) -> tuple[bool, list[str]]:
        """
        Validate a patch before application.

        Returns:
            Tuple of (is_valid, list of validation errors)
        """
        errors = []

        if not patch_content.strip():
            errors.append("Patch is empty")
            return False, errors

        # Check for basic unified diff structure
        has_file_header = bool(re.search(r"^---\s+", patch_content, re.MULTILINE))
        has_new_header = bool(re.search(r"^\+\+\+\s+", patch_content, re.MULTILINE))
        has_hunk = bool(re.search(r"^@@\s+-\d+", patch_content, re.MULTILINE))

        if not has_file_header:
            errors.append("Missing --- file header")
        if not has_new_header:
            errors.append("Missing +++ file header")
        if not has_hunk:
            errors.append("Missing @@ hunk header")

        # Extract and validate file paths
        file_paths = self._extract_file_paths(patch_content)
        for file_path in file_paths:
            full_path = self.project_root / file_path
            if not full_path.exists():
                errors.append(f"File does not exist: {file_path}")

        # Check for dangerous patterns
        dangerous_patterns = [
            (r"rm\s+-rf", "Contains dangerous rm -rf"),
            (r"DROP\s+TABLE", "Contains SQL DROP statement"),
            (r"\.unwrap\(\)", "Introduces unwrap() (discouraged)"),
            (r"panic!\s*\(", "Introduces panic! macro"),
        ]

        for pattern, message in dangerous_patterns:
            if re.search(pattern, patch_content, re.IGNORECASE):
                errors.append(f"Warning: {message}")

        return len(errors) == 0 or all("Warning:" in e for e in errors), errors

    def apply_patch(
        self,
        patch_content: str,
        dry_run: bool = False,
    ) -> PatchResult:
        """
        Apply a patch to the project.

        Args:
            patch_content: Unified diff patch content.
            dry_run: If True, validate but don't apply.

        Returns:
            PatchResult with application status.
        """
        is_valid, errors = self.validate_patch(patch_content)

        if not is_valid:
            return PatchResult(
                success=False,
                files_patched=[],
                errors=errors,
            )

        if dry_run:
            return PatchResult(
                success=True,
                files_patched=self._extract_file_paths(patch_content),
                errors=[],
            )

        # Write patch to temp file
        patch_file = self.project_root / ".ai_fix.patch"
        patch_file.write_text(patch_content, encoding="utf-8")

        try:
            # Apply patch using git apply
            result = subprocess.run(
                ["git", "apply", "--check", str(patch_file)],
                cwd=self.project_root,
                capture_output=True,
                text=True,
            )

            if result.returncode != 0:
                return PatchResult(
                    success=False,
                    files_patched=[],
                    errors=[f"Patch check failed: {result.stderr}"],
                )

            # Actually apply
            result = subprocess.run(
                ["git", "apply", str(patch_file)],
                cwd=self.project_root,
                capture_output=True,
                text=True,
            )

            if result.returncode != 0:
                return PatchResult(
                    success=False,
                    files_patched=[],
                    errors=[f"Patch application failed: {result.stderr}"],
                )

            return PatchResult(
                success=True,
                files_patched=self._extract_file_paths(patch_content),
                errors=[e for e in errors if e.startswith("Warning:")],
            )

        finally:
            # Clean up
            if patch_file.exists():
                patch_file.unlink()

    def create_fix_branch(
        self,
        branch_name: str,
        patch_content: str,
        commit_message: str,
    ) -> tuple[bool, str]:
        """
        Create a new branch with the fix applied.

        Args:
            branch_name: Name for the fix branch.
            patch_content: Patch to apply.
            commit_message: Commit message for the fix.

        Returns:
            Tuple of (success, error_message or branch_name)
        """
        try:
            # Create and checkout new branch
            subprocess.run(
                ["git", "checkout", "-b", branch_name],
                cwd=self.project_root,
                capture_output=True,
                check=True,
            )

            # Apply patch
            result = self.apply_patch(patch_content)
            if not result.success:
                # Rollback
                subprocess.run(
                    ["git", "checkout", "-"],
                    cwd=self.project_root,
                    capture_output=True,
                )
                subprocess.run(
                    ["git", "branch", "-D", branch_name],
                    cwd=self.project_root,
                    capture_output=True,
                )
                return False, "; ".join(result.errors)

            # Stage and commit
            subprocess.run(
                ["git", "add", "-A"],
                cwd=self.project_root,
                capture_output=True,
                check=True,
            )

            subprocess.run(
                ["git", "commit", "-m", commit_message],
                cwd=self.project_root,
                capture_output=True,
                check=True,
            )

            return True, branch_name

        except subprocess.CalledProcessError as e:
            return False, f"Git operation failed: {e.stderr}"

    def _extract_file_paths(self, patch_content: str) -> list[str]:
        """Extract file paths from a patch."""
        paths = []

        # Match --- a/path/to/file or +++ b/path/to/file
        for match in re.finditer(r"^(?:---|\+\+\+)\s+[ab]/(.+)$", patch_content, re.MULTILINE):
            path = match.group(1).strip()
            if path and path not in paths:
                paths.append(path)

        return paths


def format_fix_as_pr_comment(
    fix_response: "FixResponse",
    failures_summary: str,
) -> str:
    """Format a fix response as a GitHub PR comment."""
    lines = [
        "## AI-Generated Fix Suggestion",
        "",
        "The following fix has been automatically generated for the CI failures.",
        "",
        "### Failure Summary",
        failures_summary,
        "",
        f"### Confidence: {fix_response.confidence:.0%}",
        "",
        "### Explanation",
        fix_response.explanation,
        "",
        "### Files Modified",
    ]

    for file_path in fix_response.files_modified:
        lines.append(f"- `{file_path}`")

    lines.extend([
        "",
        "### Suggested Patch",
        "```diff",
        fix_response.patch,
        "```",
        "",
        "---",
        "*This fix was generated by AI and requires human review before merging.*",
    ])

    return "\n".join(lines)
