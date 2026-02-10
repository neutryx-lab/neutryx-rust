#!/usr/bin/env python3
"""
AI-generated documentation compression script for Neutryx specs.
Applies compression rules to reduce bloat while preserving technical decisions.
"""

import re
from pathlib import Path

def compress_design(content: str) -> str:
    """Apply design.md compression rules"""
    lines = content.split('\n')
    result = []
    in_code_block = False
    code_block_lines = []
    code_block_lang = ""
    skip_section = False
    skip_until_header = False
    mermaid_count = 0

    i = 0
    while i < len(lines):
        line = lines[i]

        # Track code blocks
        if line.startswith('```'):
            if not in_code_block:
                in_code_block = True
                code_block_lang = line[3:].strip()
                code_block_lines = [line]
            else:
                in_code_block = False
                code_block_lines.append(line)

                # D1: Remove Rust code blocks longer than ~15 lines (keep signatures only)
                if code_block_lang == 'rust' and len(code_block_lines) > 17:
                    # Keep only pub trait/struct/enum signatures
                    compressed = []
                    for cb_line in code_block_lines:
                        if any(keyword in cb_line for keyword in ['```rust', 'pub trait', 'pub struct', 'pub enum', 'pub type']):
                            compressed.append(cb_line)
                        elif cb_line.strip().startswith('#[') or cb_line.strip().startswith('///'):
                            compressed.append(cb_line)
                    if len(compressed) > 2:  # More than just fences
                        result.extend(compressed[:10] + ['    // ... implementation omitted ...', '```'])
                    else:
                        result.append('```rust\n// [Large code block omitted]\n```')
                else:
                    # D2: Keep at most 2 Mermaid diagrams
                    if code_block_lang == 'mermaid':
                        mermaid_count += 1
                        if mermaid_count <= 2:
                            result.extend(code_block_lines)
                        else:
                            result.append('*[Mermaid diagram omitted]*')
                    else:
                        result.extend(code_block_lines)

                code_block_lines = []
                code_block_lang = ""
            i += 1
            continue

        if in_code_block:
            code_block_lines.append(line)
            i += 1
            continue

        # D3-D7: Delete specific sections
        if line.startswith('##'):
            header_lower = line.lower()
            if any(keyword in header_lower for keyword in [
                'non-goals',  # D3
                'security considerations',  # D4
                'migration strategy',  # D5
                'technology stack',  # D6
                'supporting references',  # D11
                'performance & scalability',  # D12
            ]):
                skip_until_header = True
                i += 1
                continue
            else:
                skip_until_header = False

        # D9-D11: Remove contract sections
        if line.startswith('###') or line.startswith('####'):
            header_lower = line.lower()
            if any(keyword in header_lower for keyword in [
                'api contract',
                'event contract',
                'batch contract',
                'job contract',
                'physical data model',
            ]):
                skip_until_header = True
                i += 1
                continue
            else:
                skip_until_header = False

        if not skip_until_header:
            result.append(line)

        i += 1

    return '\n'.join(result)

def compress_requirements(content: str) -> str:
    """Apply requirements.md compression rules"""
    lines = content.split('\n')
    result = []
    skip_until_header = False
    in_criteria = False
    criteria_count = 0
    criteria_buffer = []

    for line in lines:
        # R1: Delete Glossary sections
        if line.startswith('##') and 'glossary' in line.lower():
            skip_until_header = True
            continue

        # R2: Delete Dependencies tables
        if 'dependencies' in line.lower() and ('##' in line or '###' in line):
            skip_until_header = True
            continue

        if line.startswith('##') or line.startswith('###'):
            skip_until_header = False
            # Flush criteria buffer if needed
            if in_criteria and criteria_buffer:
                # R4: Keep only 3 most substantive criteria
                result.extend(criteria_buffer[:3])
                if len(criteria_buffer) > 3:
                    result.append(f'*[{len(criteria_buffer) - 3} additional criteria omitted]*')
                criteria_buffer = []
                in_criteria = False

        # Track acceptance criteria
        if 'acceptance criteria' in line.lower():
            in_criteria = True
            criteria_count = 0

        if in_criteria and line.strip() and line[0].isdigit() and '.' in line[:5]:
            criteria_buffer.append(line)
            continue

        # R3: Delete project-wide constraints
        if 'constraint' in line.lower() and any(keyword in line.lower() for keyword in [
            'british english', 'static dispatch', 'layer isolation'
        ]):
            continue

        if not skip_until_header:
            result.append(line)

    return '\n'.join(result)

def compress_tasks(content: str) -> str:
    """Apply tasks.md compression rules"""
    lines = content.split('\n')
    result = []
    in_test_block = False
    skip_test = False

    for line in lines:
        # T1: Delete Rust test code blocks
        if line.startswith('```rust'):
            if '#[test]' in '\n'.join(lines[lines.index(line):lines.index(line)+10]) or '#[cfg(test)]' in '\n'.join(lines[lines.index(line):lines.index(line)+10]):
                skip_test = True
                continue

        if skip_test:
            if line.startswith('```') and not line.startswith('```rust'):
                skip_test = False
            continue

        # T2: Remove Status/Priority metadata (but keep overview table)
        if line.strip().startswith('**Status:**') or line.strip().startswith('**Priority:**'):
            if '##' not in '\n'.join(result[-10:]):  # Not in overview section
                continue

        # T3: Delete Mermaid dependency graphs
        if '```mermaid' in line and 'graph' in '\n'.join(lines[lines.index(line):lines.index(line)+5]).lower():
            skip_test = True
            result.append('*[Task dependency graph omitted]*')
            continue

        result.append(line)

    return '\n'.join(result)

def compress_gap_analysis(content: str) -> str:
    """Apply gap-analysis.md compression rules"""
    lines = content.split('\n')
    result = []
    in_code_block = False
    code_line_count = 0

    for i, line in enumerate(lines):
        if line.startswith('```'):
            if not in_code_block:
                in_code_block = True
                code_line_count = 0
            else:
                in_code_block = False

        if in_code_block:
            code_line_count += 1

        # G1: Replace quoted Rust code blocks (>10 lines) with file path references
        if in_code_block and code_line_count > 12 and '```rust' in '\n'.join(lines[max(0,i-15):i]):
            result.append('// [Code excerpt - see implementation file]')
            continue

        # G3: Replace emoji markers with plain text
        line = line.replace('✅', 'Complete')
        line = line.replace('❌', 'Missing')
        line = line.replace('🔶', 'Partial')
        line = line.replace('⏳', 'Deferred')
        line = line.replace('🎯', 'Future')

        # G2: Condense rejected implementation options
        if '**option' in line.lower() and ('rejected' in '\n'.join(lines[i:i+10]).lower() or 'not recommended' in '\n'.join(lines[i:i+10]).lower()):
            # Simplify to one-liner
            next_lines = []
            j = i + 1
            while j < len(lines) and not lines[j].startswith('**Option') and not lines[j].startswith('##'):
                next_lines.append(lines[j])
                j += 1
            summary = ' '.join([l.strip() for l in next_lines[:2] if l.strip()])
            result.append(f"{line} - {summary[:80]}...")
            # Skip the detailed lines
            continue

        result.append(line)

    return '\n'.join(result)

def compress_research(content: str) -> str:
    """Apply research.md compression rules"""
    lines = content.split('\n')
    result = []
    in_decision_table = False

    for i, line in enumerate(lines):
        # RS1: Simplify over-structured Research Log entries
        if '### topic' in line.lower() or '### research' in line.lower():
            # Simplify format
            result.append(line)
            # Skip verbose Context/Sources/Findings structure
            j = i + 1
            while j < len(lines) and not lines[j].startswith('###'):
                if lines[j].startswith('- **Findings**:') or lines[j].startswith('**Findings**:'):
                    result.append(lines[j])
                    # Include next few lines only
                    for k in range(j+1, min(j+5, len(lines))):
                        if lines[k].strip():
                            result.append(lines[k])
                    break
                j += 1
            continue

        # RS2: Keep only selected approach in decision tables
        if '| option' in line.lower() or '| alternative' in line.lower():
            if '**selected' in '\n'.join(lines[max(0,i-2):i+3]).lower() or '**adopted' in '\n'.join(lines[max(0,i-2):i+3]).lower():
                result.append(line)
            else:
                # Compress rejected alternative to minimal form
                parts = line.split('|')
                if len(parts) > 2:
                    result.append(f"| {parts[1].strip()} | [Rejected] | - | - |")
            continue

        result.append(line)

    return '\n'.join(result)

def process_spec_directory(spec_dir: Path):
    """Process all markdown files in a spec directory"""
    print(f"\n=== Processing {spec_dir.name} ===")

    files_processed = {}

    for md_file in spec_dir.glob('*.md'):
        print(f"  {md_file.name}...", end=' ')

        content = md_file.read_text(encoding='utf-8')
        before_lines = len(content.split('\n'))

        if md_file.name == 'design.md':
            compressed = compress_design(content)
        elif md_file.name == 'requirements.md':
            compressed = compress_requirements(content)
        elif md_file.name == 'tasks.md':
            compressed = compress_tasks(content)
        elif md_file.name == 'gap-analysis.md':
            compressed = compress_gap_analysis(content)
        elif md_file.name == 'research.md':
            compressed = compress_research(content)
        else:
            compressed = content

        after_lines = len(compressed.split('\n'))
        reduction = before_lines - after_lines
        pct = (reduction / before_lines * 100) if before_lines > 0 else 0

        md_file.write_text(compressed, encoding='utf-8')
        files_processed[md_file.name] = (before_lines, after_lines, reduction, pct)

        print(f"{before_lines} -> {after_lines} lines (-{reduction}, -{pct:.1f}%)")

    return files_processed

def main():
    base_path = Path(r"c:\Users\khosh\Codes\neutryx-rust\.kiro\specs")

    spec_dirs = [
        "fx-vol-surface-calibration",
        "generic-pricer-engine",
        "infra-primitives-migration",
        "instrument-definitions"
    ]

    total_before = 0
    total_after = 0

    for spec_dir_name in spec_dirs:
        spec_dir = base_path / spec_dir_name
        if spec_dir.exists():
            results = process_spec_directory(spec_dir)
            for filename, (before, after, reduction, pct) in results.items():
                total_before += before
                total_after += after

    total_reduction = total_before - total_after
    total_pct = (total_reduction / total_before * 100) if total_before > 0 else 0

    print(f"\n=== TOTAL ===")
    print(f"  {total_before} -> {total_after} lines")
    print(f"  Reduction: {total_reduction} lines ({total_pct:.1f}%)")

if __name__ == '__main__':
    main()
