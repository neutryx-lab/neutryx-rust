"""
CI log parser for extracting structured failure information.

Handles various CI failure types:
- Rust compilation errors
- Clippy warnings/errors
- Test failures (including numerical NaN issues)
- Formatting violations
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from enum import Enum, auto
from pathlib import Path


class FailureType(Enum):
    """Categories of CI failures."""

    COMPILATION = auto()
    CLIPPY = auto()
    TEST = auto()
    FORMAT = auto()
    ENZYME = auto()  # Enzyme AD-specific failures
    UNKNOWN = auto()


@dataclass
class FileLocation:
    """Source file location for an error."""

    file_path: str
    line: int | None = None
    column: int | None = None

    def __str__(self) -> str:
        loc = self.file_path
        if self.line:
            loc += f":{self.line}"
            if self.column:
                loc += f":{self.column}"
        return loc


@dataclass
class FailureInfo:
    """Structured information about a CI failure."""

    failure_type: FailureType
    error_code: str | None = None
    message: str = ""
    locations: list[FileLocation] = field(default_factory=list)
    raw_output: str = ""
    suggestion: str | None = None  # Compiler/Clippy suggestion if available
    affected_crates: list[str] = field(default_factory=list)

    def to_prompt_context(self) -> str:
        """Format failure info for LLM prompt."""
        lines = [
            f"## Failure Type: {self.failure_type.name}",
        ]
        if self.error_code:
            lines.append(f"Error Code: {self.error_code}")
        lines.append(f"Message: {self.message}")

        if self.locations:
            lines.append("Locations:")
            for loc in self.locations:
                lines.append(f"  - {loc}")

        if self.affected_crates:
            lines.append(f"Affected Crates: {', '.join(self.affected_crates)}")

        if self.suggestion:
            lines.append(f"Compiler Suggestion: {self.suggestion}")

        return "\n".join(lines)


class CILogParser:
    """Parser for CI failure logs."""

    # Rust error pattern: error[E0XXX]: message
    RE_RUST_ERROR = re.compile(
        r"error\[(?P<code>E\d+)\]:\s*(?P<message>.+?)(?:\n|$)"
    )

    # Rust location pattern: --> path/to/file.rs:line:col
    RE_LOCATION = re.compile(
        r"-->\s*(?P<path>[^:]+):(?P<line>\d+):(?P<col>\d+)"
    )

    # Clippy warning pattern: warning: message
    RE_CLIPPY_WARNING = re.compile(
        r"warning:\s*(?P<message>.+?)(?:\n|$)"
    )

    # Clippy lint pattern: #[warn(clippy::lint_name)]
    RE_CLIPPY_LINT = re.compile(
        r"#\[(?:warn|deny)\(clippy::(?P<lint>\w+)\)\]"
    )

    # Test failure pattern
    RE_TEST_FAILURE = re.compile(
        r"(?:FAILED|test .+ \.\.\. FAILED|panicked at)"
    )

    # NaN detection pattern
    RE_NAN_ERROR = re.compile(
        r"(?:NaN|assertion.*failed.*NaN|not a number)",
        re.IGNORECASE,
    )

    # Cargo format check failure
    RE_FORMAT_DIFF = re.compile(
        r"Diff in (?P<path>[^\s]+)|would have made changes to"
    )

    # Crate name extraction
    RE_CRATE_NAME = re.compile(
        r"(?:Compiling|Checking|Building|Testing)\s+(?P<crate>\w+)"
    )

    def __init__(self, log_content: str):
        self.log_content = log_content
        self.failures: list[FailureInfo] = []

    def parse(self) -> list[FailureInfo]:
        """Parse the log content and extract all failures."""
        self.failures = []

        # Try each failure type
        self._parse_compilation_errors()
        self._parse_clippy_warnings()
        self._parse_test_failures()
        self._parse_format_errors()

        # If no specific failures found, create a generic one
        if not self.failures:
            self.failures.append(
                FailureInfo(
                    failure_type=FailureType.UNKNOWN,
                    message="CI failure detected but specific cause not parsed",
                    raw_output=self.log_content[-5000:],  # Last 5KB
                )
            )

        return self.failures

    def _parse_compilation_errors(self) -> None:
        """Extract Rust compilation errors."""
        for match in self.RE_RUST_ERROR.finditer(self.log_content):
            code = match.group("code")
            message = match.group("message").strip()

            # Find associated location
            start_pos = match.start()
            context = self.log_content[start_pos : start_pos + 1000]

            locations = []
            for loc_match in self.RE_LOCATION.finditer(context):
                locations.append(
                    FileLocation(
                        file_path=loc_match.group("path"),
                        line=int(loc_match.group("line")),
                        column=int(loc_match.group("col")),
                    )
                )

            # Extract affected crates
            crates = self._extract_crates(context)

            # Look for compiler suggestion
            suggestion = self._extract_suggestion(context)

            # Check if Enzyme-related
            failure_type = FailureType.COMPILATION
            if "enzyme" in context.lower() or "autodiff" in context.lower():
                failure_type = FailureType.ENZYME

            self.failures.append(
                FailureInfo(
                    failure_type=failure_type,
                    error_code=code,
                    message=message,
                    locations=locations,
                    raw_output=context,
                    suggestion=suggestion,
                    affected_crates=crates,
                )
            )

    def _parse_clippy_warnings(self) -> None:
        """Extract Clippy warnings treated as errors."""
        # Find "warning: ... note: `-D warnings`" pattern
        clippy_blocks = re.findall(
            r"(warning:.*?(?=warning:|error:|$))",
            self.log_content,
            re.DOTALL,
        )

        for block in clippy_blocks:
            if "clippy::" not in block:
                continue

            msg_match = self.RE_CLIPPY_WARNING.search(block)
            if not msg_match:
                continue

            message = msg_match.group("message").strip()

            # Extract lint name
            lint_match = self.RE_CLIPPY_LINT.search(block)
            error_code = f"clippy::{lint_match.group('lint')}" if lint_match else None

            # Extract location
            locations = []
            for loc_match in self.RE_LOCATION.finditer(block):
                locations.append(
                    FileLocation(
                        file_path=loc_match.group("path"),
                        line=int(loc_match.group("line")),
                        column=int(loc_match.group("col")),
                    )
                )

            if locations:  # Only add if we have a location
                self.failures.append(
                    FailureInfo(
                        failure_type=FailureType.CLIPPY,
                        error_code=error_code,
                        message=message,
                        locations=locations,
                        raw_output=block[:2000],
                        affected_crates=self._extract_crates(block),
                    )
                )

    def _parse_test_failures(self) -> None:
        """Extract test failures including numerical issues."""
        if not self.RE_TEST_FAILURE.search(self.log_content):
            return

        # Find test failure blocks
        test_sections = re.findall(
            r"(---- .+? ----.*?(?=---- |$))",
            self.log_content,
            re.DOTALL,
        )

        for section in test_sections:
            if "FAILED" not in section and "panicked" not in section:
                continue

            # Extract test name
            test_name_match = re.search(r"---- (.+?) ----", section)
            test_name = test_name_match.group(1) if test_name_match else "unknown"

            # Check for NaN issues
            is_nan_error = bool(self.RE_NAN_ERROR.search(section))
            message = f"Test failed: {test_name}"
            if is_nan_error:
                message += " (NaN detected - numerical instability)"

            # Extract location
            locations = []
            for loc_match in self.RE_LOCATION.finditer(section):
                locations.append(
                    FileLocation(
                        file_path=loc_match.group("path"),
                        line=int(loc_match.group("line")),
                        column=int(loc_match.group("col")),
                    )
                )

            self.failures.append(
                FailureInfo(
                    failure_type=FailureType.TEST,
                    message=message,
                    locations=locations,
                    raw_output=section[:3000],
                    affected_crates=self._extract_crates(section),
                )
            )

    def _parse_format_errors(self) -> None:
        """Extract formatting violations."""
        if "cargo fmt" not in self.log_content.lower():
            return

        format_matches = self.RE_FORMAT_DIFF.findall(self.log_content)
        if not format_matches:
            return

        files = [m for m in format_matches if m]

        self.failures.append(
            FailureInfo(
                failure_type=FailureType.FORMAT,
                message=f"Formatting violations in {len(files)} file(s)",
                locations=[FileLocation(file_path=f) for f in files[:10]],
                raw_output="Run `cargo fmt --all` to fix formatting",
            )
        )

    def _extract_crates(self, text: str) -> list[str]:
        """Extract crate names from error context."""
        crates = set()
        for match in self.RE_CRATE_NAME.finditer(text):
            crates.add(match.group("crate"))
        return list(crates)

    def _extract_suggestion(self, text: str) -> str | None:
        """Extract compiler/Clippy suggestion if available."""
        # Look for "help: ..." or "suggestion: ..."
        suggestion_match = re.search(
            r"(?:help|suggestion):\s*(.+?)(?:\n|$)",
            text,
            re.IGNORECASE,
        )
        return suggestion_match.group(1).strip() if suggestion_match else None


def parse_log_file(log_path: Path) -> list[FailureInfo]:
    """Convenience function to parse a log file."""
    content = log_path.read_text(encoding="utf-8", errors="replace")
    parser = CILogParser(content)
    return parser.parse()
