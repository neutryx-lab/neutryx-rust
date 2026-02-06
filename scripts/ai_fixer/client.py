"""
Gemini API client for code fix generation.

Uses Google's Generative AI SDK to generate fix patches.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .parser import FailureInfo
    from .context import ProjectContext


@dataclass
class FixResponse:
    """Response from the AI fix generation."""

    patch: str
    explanation: str
    confidence: float  # 0.0 to 1.0
    files_modified: list[str]
    requires_review: bool = True


class GeminiClient:
    """Client for Google Gemini API."""

    MODEL_ID = "gemini-1.5-pro"
    FLASH_MODEL_ID = "gemini-1.5-flash"

    def __init__(
        self,
        api_key: str | None = None,
        use_flash: bool = False,
    ):
        """
        Initialise Gemini client.

        Args:
            api_key: Google AI API key. If None, reads from GOOGLE_API_KEY env var.
            use_flash: Use faster Flash model instead of Pro.
        """
        self.api_key = api_key or os.environ.get("GOOGLE_API_KEY")
        if not self.api_key:
            raise ValueError(
                "Gemini API key required. Set GOOGLE_API_KEY environment variable."
            )

        self.model_id = self.FLASH_MODEL_ID if use_flash else self.MODEL_ID
        self._client = None

    def _get_client(self):
        """Lazy initialisation of Gemini client."""
        if self._client is None:
            try:
                import google.generativeai as genai

                genai.configure(api_key=self.api_key)
                self._client = genai.GenerativeModel(
                    model_name=self.model_id,
                    generation_config={
                        "temperature": 0.2,  # Low temperature for deterministic fixes
                        "top_p": 0.95,
                        "top_k": 40,
                        "max_output_tokens": 8192,
                    },
                )
            except ImportError:
                raise ImportError(
                    "google-generativeai package required. "
                    "Install with: pip install google-generativeai"
                )
        return self._client

    def generate_fix(
        self,
        failures: list["FailureInfo"],
        context: "ProjectContext",
    ) -> FixResponse:
        """
        Generate a fix patch for the given failures.

        Args:
            failures: List of parsed CI failures.
            context: Project context including affected files.

        Returns:
            FixResponse with generated patch and metadata.
        """
        prompt = self._build_prompt(failures, context)
        client = self._get_client()

        response = client.generate_content(prompt)

        return self._parse_response(response.text)

    def _build_prompt(
        self,
        failures: list["FailureInfo"],
        context: "ProjectContext",
    ) -> str:
        """Build the complete prompt for fix generation."""
        parts = [
            context.to_system_prompt(),
            "",
            "# CI Failure Analysis",
            "",
            "The following CI failures need to be fixed:",
            "",
        ]

        # Add failure details
        for i, failure in enumerate(failures, 1):
            parts.append(f"## Failure {i}")
            parts.append(failure.to_prompt_context())
            parts.append("")

        # Add affected file contents
        file_context = context.to_file_context()
        if file_context:
            parts.append(file_context)

        # Add instructions
        parts.extend([
            "# Instructions",
            "",
            "Generate a fix for the CI failures above.",
            "",
            "## Output Format",
            "",
            "Respond with a JSON object containing:",
            "```json",
            "{",
            '  "explanation": "Brief explanation of the fix",',
            '  "confidence": 0.95,  // 0.0-1.0 confidence score',
            '  "files_modified": ["path/to/file.rs"],',
            '  "requires_review": true,  // Always true for safety',
            '  "patch": "unified diff format patch"',
            "}",
            "```",
            "",
            "## Patch Format",
            "",
            "The patch should be in standard unified diff format:",
            "```diff",
            "--- a/path/to/file.rs",
            "+++ b/path/to/file.rs",
            "@@ -line,count +line,count @@",
            " context line",
            "-removed line",
            "+added line",
            "```",
            "",
            "## Important Rules",
            "",
            "1. Use British English spelling (optimiser, behaviour, serialisation)",
            "2. Follow existing code style and conventions",
            "3. Do not introduce new dependencies",
            "4. Preserve existing functionality",
            "5. Handle edge cases (NaN, overflow, etc.)",
            "6. For Clippy fixes, apply the minimal change needed",
            "7. For test failures, fix the root cause, not just the test",
            "",
        ])

        return "\n".join(parts)

    def _parse_response(self, response_text: str) -> FixResponse:
        """Parse the LLM response into a FixResponse."""
        # Try to extract JSON from response
        try:
            # Look for JSON block in response
            json_match = self._extract_json(response_text)
            if json_match:
                data = json.loads(json_match)
                return FixResponse(
                    patch=data.get("patch", ""),
                    explanation=data.get("explanation", ""),
                    confidence=float(data.get("confidence", 0.5)),
                    files_modified=data.get("files_modified", []),
                    requires_review=data.get("requires_review", True),
                )
        except (json.JSONDecodeError, KeyError, TypeError):
            pass

        # Fallback: try to extract patch directly
        patch = self._extract_diff(response_text)

        return FixResponse(
            patch=patch,
            explanation="AI-generated fix (parsed from raw response)",
            confidence=0.3,  # Lower confidence for fallback parsing
            files_modified=[],
            requires_review=True,
        )

    def _extract_json(self, text: str) -> str | None:
        """Extract JSON block from text."""
        import re

        # Try code block first
        json_block = re.search(r"```json\s*\n(.*?)\n```", text, re.DOTALL)
        if json_block:
            return json_block.group(1)

        # Try raw JSON
        json_match = re.search(r"\{[^{}]*\"patch\"[^{}]*\}", text, re.DOTALL)
        if json_match:
            return json_match.group(0)

        return None

    def _extract_diff(self, text: str) -> str:
        """Extract diff/patch from text."""
        import re

        # Look for diff block
        diff_block = re.search(r"```diff\s*\n(.*?)\n```", text, re.DOTALL)
        if diff_block:
            return diff_block.group(1)

        # Look for unified diff pattern
        diff_lines = []
        in_diff = False
        for line in text.split("\n"):
            if line.startswith("---") or line.startswith("+++"):
                in_diff = True
            if in_diff:
                diff_lines.append(line)
                if line.startswith("@@") and not line.strip().endswith("@@"):
                    pass  # Continue collecting

        return "\n".join(diff_lines)


class MockGeminiClient(GeminiClient):
    """Mock client for testing without API calls."""

    def __init__(self, mock_response: FixResponse | None = None):
        self.mock_response = mock_response or FixResponse(
            patch="",
            explanation="Mock response",
            confidence=1.0,
            files_modified=[],
            requires_review=True,
        )

    def _get_client(self):
        return None

    def generate_fix(
        self,
        failures: list["FailureInfo"],
        context: "ProjectContext",
    ) -> FixResponse:
        return self.mock_response
