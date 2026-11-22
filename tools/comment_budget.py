"""Enforce the added ordinary-comment budget across an explicit file diff."""
import argparse
import difflib
import subprocess
import sys
from pathlib import Path

from comment_check import analyze


def count_added(old, new, suffix):
    old_lines = old.splitlines()
    new_lines, comments, code, errors = analyze(new, suffix)
    if errors:
        raise ValueError('; '.join(errors))
    added_code = 0
    added_comments = 0
    for tag, _, _, start, end in difflib.SequenceMatcher(a=old_lines, b=new_lines, autojunk=False).get_opcodes():
        if tag not in ('insert', 'replace'):
            continue
        for index in range(start, end):
            number = index + 1
            if number in code:
                added_code += 1
            if number in comments and comments[number][0] in ('ordinary', 'block', 'block-start', 'block-continuation'):
                added_comments += 1
    return added_code, added_comments


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', required=True, help='base commit or tree')
    parser.add_argument('files', nargs='+', type=Path, help='explicit changed source paths')
    args = parser.parse_args()
    base = subprocess.run(['git', 'rev-parse', '--verify', f'{args.base}^{{tree}}'], capture_output=True, text=True)
    if base.returncode:
        print('Invalid diff base.', file=sys.stderr)
        return 2
    total_code = total_comments = 0
    for path in args.files:
        if path.suffix not in ('.rs', '.py') or not path.is_file():
            parser.error(f'unsupported or missing file: {path}')
        previous = subprocess.run(['git', 'show', f'{args.base}:{path.as_posix()}'], capture_output=True, text=True)
        old = previous.stdout if previous.returncode == 0 else ''
        try:
            code, comments = count_added(old, path.read_text(encoding='utf8'), path.suffix)
        except ValueError as exc:
            print(f'{path}: {exc}', file=sys.stderr)
            return 2
        total_code += code
        total_comments += comments
    limit = max(2, total_code // 25)
    print(f'Added code lines: {total_code}; ordinary comments: {total_comments}; limit: {limit}.')
    if total_comments > limit:
        print('Added comments exceed the diff budget.', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
