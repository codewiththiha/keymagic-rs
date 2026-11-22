"""Check concise Rust and Python source comments and documentation."""
import argparse
import ast
import io
import re
import sys
import tokenize
from pathlib import Path


def rust_comments(source):
    lines = source.splitlines()
    entries = {}
    code = set()
    mode = ''
    raw_end = ''
    block_start = None
    for number, line in enumerate(lines, 1):
        i = 0
        while i < len(line):
            if mode == 'raw':
                end = line.find(raw_end, i)
                if end < 0:
                    code.add(number)
                    break
                code.add(number)
                i = end + len(raw_end)
                mode = ''
                continue
            if mode == 'string':
                code.add(number)
                if line[i] == '\\':
                    i += 2
                elif line[i] == '"':
                    mode = ''
                    i += 1
                else:
                    i += 1
                continue
            if mode == 'block':
                end = line.find('*/', i)
                if end < 0:
                    entries.setdefault(number, ('block-continuation', line[i:]))
                    break
                entries.setdefault(number, ('block-continuation', line[i:end + 2]))
                mode = ''
                i = end + 2
                continue
            raw = re.match(r'(?:br|r)(#{0,255})"', line[i:])
            if raw:
                code.add(number)
                mode = 'raw'
                raw_end = '"' + raw.group(1)
                i += len(raw.group(0))
            elif line.startswith('//', i):
                kind = 'module' if line.startswith('//!', i) else 'doc' if line.startswith('///', i) and not line.startswith('////', i) else 'ordinary'
                entries[number] = (kind, line[i:])
                break
            elif line.startswith('/*', i):
                end = line.find('*/', i + 2)
                if end < 0:
                    entries[number] = ('block-start', line[i:])
                    mode = 'block'
                    block_start = number
                    break
                entries[number] = ('block', line[i:end + 2])
                i = end + 2
            elif line[i] == '"':
                code.add(number)
                mode = 'string'
                i += 1
            elif line[i] == "'":
                char = re.match(r"'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^'\\])'", line[i:])
                code.add(number)
                i += len(char.group(0)) if char else 1
            else:
                if not line[i].isspace():
                    code.add(number)
                i += 1
    return lines, entries, code


def python_comments(source):
    lines = source.splitlines()
    entries = {}
    code = set()
    try:
        for tok in tokenize.generate_tokens(io.StringIO(source).readline):
            if tok.type == tokenize.COMMENT:
                if tok.start[0] == 1 and tok.string.startswith('#!'):
                    continue
                entries[tok.start[0]] = ('ordinary', tok.string)
            elif tok.type not in (tokenize.NL, tokenize.NEWLINE, tokenize.INDENT, tokenize.DEDENT, tokenize.ENDMARKER, tokenize.ENCODING):
                code.update(range(tok.start[0], tok.end[0] + 1))
    except (tokenize.TokenError, IndentationError) as exc:
        return lines, entries, code, [f'Python tokenization failed: {exc}']
    try:
        tree = ast.parse(source)
        nodes = [tree] + [n for n in ast.walk(tree) if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef))]
        for node in nodes:
            if node.body and isinstance(node.body[0], ast.Expr) and isinstance(node.body[0].value, ast.Constant) and isinstance(node.body[0].value.value, str):
                doc = node.body[0]
                limit = 5 if node is tree and doc.lineno in (1, 2) else 1
                entries[doc.lineno] = ('py-doc', lines[doc.lineno - 1])
                code.difference_update(range(doc.lineno, doc.end_lineno + 1))
                if doc.end_lineno - doc.lineno + 1 > limit:
                    entries[doc.lineno] = ('docstring-too-long', '')
    except SyntaxError as exc:
        return lines, entries, code, [f'Python parse failed: {exc}']
    return lines, entries, code, []


def analyze(source, suffix):
    if suffix == '.py':
        return python_comments(source)
    lines, entries, code = rust_comments(source)
    return lines, entries, code, []


def check_source(source, suffix):
    lines, entries, _, errors = analyze(source, suffix)
    module_end = 0
    while module_end + 1 in entries and entries[module_end + 1][0] == 'module':
        module_end += 1
    if module_end > 5:
        errors.append('line 1: initial module documentation exceeds five lines')
    previous = 0
    for number, (kind, text) in sorted(entries.items()):
        if kind == 'docstring-too-long':
            errors.append(f'line {number}: docstring exceeds its line limit')
            continue
        if kind in ('block-start', 'block-continuation'):
            errors.append(f'line {number}: multiline block comments are not allowed')
        if kind == 'module' and number > module_end:
            errors.append(f'line {number}: module docs must start on the first line')
        if previous and number == previous + 1 and not (number <= module_end and kind == 'module'):
            errors.append(f'line {number}: adjacent comment lines are not allowed')
        if len(lines[number - 1]) > 80:
            errors.append(f'line {number}: comment line exceeds 80 columns')
        if kind in ('ordinary', 'block', 'block-start', 'block-continuation'):
            if len(re.findall(r"\b[\w'-]+\b", re.sub(r'^\s*(?://|#|/\*)|\*/$', '', text))) > 15:
                errors.append(f'line {number}: ordinary comment exceeds 15 words')
            if 'comment-check: allow' in text:
                errors.append(f'line {number}: ordinary comments cannot use allow markers')
        if re.search(r'\b(?:TODO|FIXME)\b', text, re.I):
            errors.append(f'line {number}: unfinished-work markers are not allowed')
        previous = number
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('files', nargs='+', type=Path, help='explicit source file paths')
    args = parser.parse_args()
    failed = False
    for path in args.files:
        if path.suffix not in ('.rs', '.py'):
            parser.error(f'unsupported source type: {path}')
        if not path.is_file():
            print(f'{path}: file not found', file=sys.stderr)
            failed = True
            continue
        for error in check_source(path.read_text(encoding='utf8'), path.suffix):
            print(f'{path}:{error}', file=sys.stderr)
            failed = True
    if not failed:
        print(f'Comment rules passed for {len(args.files)} file(s).')
    return int(failed)


if __name__ == '__main__':
    raise SystemExit(main())
