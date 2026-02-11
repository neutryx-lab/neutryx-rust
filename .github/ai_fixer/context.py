"""
Project context collector for AI-assisted code fixing.

Gathers relevant context from:
- CLAUDE.md (project instructions)
- .kiro/steering/*.md (project guidelines)
- Affected source files
- Cargo.toml configurations
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterator


@dataclass
class ProjectContext:
    """Aggregated project context for LLM prompts."""

    project_root: Path
    claude_md: str = ""
    steering_docs: dict[str, str] = field(default_factory=dict)
    affected_files: dict[str, str] = field(default_factory=dict)
    cargo_config: str = ""

    def __post_init__(self):
        """Load project context on initialisation."""
        self._load_claude_md()
        self._load_steering_docs()
        self._load_cargo_config()

    def _load_claude_md(self) -> None:
        """Load CLAUDE.md if present."""
        claude_path = self.project_root / "CLAUDE.md"
        if claude_path.exists():
            self.claude_md = claude_path.read_text(encoding="utf-8")

    def _load_steering_docs(self) -> None:
        """Load relevant steering documents."""
        steering_dir = self.project_root / ".kiro" / "steering"
        if not steering_dir.exists():
            return

        # Priority documents for code fixing
        priority_docs = [
            "code-quality.md",
            "error-handling.md",
            "ai_rules.md",
            "tech.md",
        ]

        for doc_name in priority_docs:
            doc_path = steering_dir / doc_name
            if doc_path.exists():
                self.steering_docs[doc_name] = doc_path.read_text(encoding="utf-8")

    def _load_cargo_config(self) -> None:
        """Load workspace Cargo.toml for lint configuration."""
        cargo_path = self.project_root / "Cargo.toml"
        if cargo_path.exists():
            self.cargo_config = cargo_path.read_text(encoding="utf-8")

    def add_affected_file(self, file_path: str | Path) -> None:
        """Load an affected source file into context."""
        path = Path(file_path)
        if not path.is_absolute():
            path = self.project_root / path

        if path.exists() and path.suffix in (".rs", ".toml", ".json"):
            try:
                content = path.read_text(encoding="utf-8")
                # Limit file size to 50KB
                if len(content) > 50_000:
                    content = content[:50_000] + "\n... (truncated)"
                rel_path = path.relative_to(self.project_root)
                self.affected_files[str(rel_path)] = content
            except (OSError, ValueError):
                pass

    def add_affected_files_from_locations(
        self, locations: list["FileLocation"]  # noqa: F821
    ) -> None:
        """Load files from failure locations."""
        for loc in locations:
            self.add_affected_file(loc.file_path)

    def to_system_prompt(self) -> str:
        """Generate system prompt section from project context."""
        parts = [
            "# Project Context: neutryx-rust",
            "",
            "You are a Senior Rust Engineer specialising in Quantitative Finance.",
            "You are fixing CI failures for a production-grade derivatives pricing library.",
            "",
            "## Key Constraints",
            "- Use British English spelling (optimiser, serialisation, behaviour)",
            "- Use thiserror for error types",
            "- Avoid panic! / unwrap() / expect() in library code",
            "- Handle NaN and numerical instability properly",
            "- Follow A-I-P-S architecture (Adapter -> Infra -> Pricer -> Service)",
            "",
        ]

        # Add CLAUDE.md summary
        if self.claude_md:
            parts.extend([
                "## Project Instructions (CLAUDE.md)",
                "",
                self._summarise_text(self.claude_md, max_lines=50),
                "",
            ])

        # Add relevant steering docs
        if self.steering_docs:
            parts.append("## Coding Guidelines")
            parts.append("")

            for doc_name, content in self.steering_docs.items():
                parts.append(f"### {doc_name}")
                parts.append("")
                parts.append(self._summarise_text(content, max_lines=30))
                parts.append("")

        return "\n".join(parts)

    def to_file_context(self) -> str:
        """Generate file context section for LLM prompt."""
        if not self.affected_files:
            return ""

        parts = ["## Affected Source Files", ""]

        for file_path, content in self.affected_files.items():
            parts.append(f"### {file_path}")
            parts.append("```rust" if file_path.endswith(".rs") else "```")
            parts.append(content)
            parts.append("```")
            parts.append("")

        return "\n".join(parts)

    def _summarise_text(self, text: str, max_lines: int = 50) -> str:
        """Truncate text to a maximum number of lines."""
        lines = text.split("\n")
        if len(lines) <= max_lines:
            return text
        return "\n".join(lines[:max_lines]) + f"\n... ({len(lines) - max_lines} more lines)"


@dataclass
class ContextBuilder:
    """Builder for incrementally constructing project context."""

    project_root: Path

    def build(
        self,
        failure_files: list[str] | None = None,
        include_neighbours: bool = True,
    ) -> ProjectContext:
        """Build project context with optional file loading."""
        ctx = ProjectContext(project_root=self.project_root)

        if failure_files:
            for file_path in failure_files:
                ctx.add_affected_file(file_path)

                # Optionally load neighbouring files in same module
                if include_neighbours:
                    self._add_module_context(ctx, file_path)

        return ctx

    def _add_module_context(self, ctx: ProjectContext, file_path: str) -> None:
        """Add related files from the same module."""
        path = Path(file_path)
        if not path.suffix == ".rs":
            return

        parent = path.parent

        # Look for mod.rs or lib.rs in same directory
        for sibling in ["mod.rs", "lib.rs"]:
            sibling_path = parent / sibling
            if sibling_path.exists() and sibling_path != path:
                ctx.add_affected_file(sibling_path)
                break


def collect_context(
    project_root: Path,
    affected_paths: list[str],
) -> ProjectContext:
    """Convenience function to collect project context."""
    builder = ContextBuilder(project_root=project_root)
    return builder.build(failure_files=affected_paths)
