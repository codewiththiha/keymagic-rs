#!/usr/bin/env python3
"""Enforce one-line commit messages of no more than 50 Unicode characters."""
import argparse
import subprocess
import sys


def problems(message: str) -> list[str]:
    message = message.rstrip('\n')
    errors = []
    if not message.strip():
        errors.append('message is empty')
    if '\n' in message or '\r' in message:
        errors.append('message must be a single line')
    if len(message) > 50:
        errors.append(f'message is {len(message)} characters (maximum 50)')
    return errors


def git(*args: str) -> str:
    return subprocess.run(['git', *args], check=True, text=True, stdout=subprocess.PIPE).stdout


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument('--message', help='check a proposed message')
    group.add_argument('--range', dest='revision_range', help='git revision or range')
    args = parser.parse_args()
    try:
        if args.message is not None:
            messages = [('proposed message', args.message)]
        else:
            shas = git('rev-list', '--reverse', args.revision_range).splitlines()
            messages = [(sha[:12], git('show', '-s', '--format=%B', sha)) for sha in shas]
        failed = False
        for label, message in messages:
            for problem in problems(message):
                print(f'{label}: {problem}', file=sys.stderr)
                failed = True
        if not failed:
            print(f'Checked {len(messages)} commit message(s).')
        return int(failed)
    except subprocess.CalledProcessError:
        print('Unable to read commit range; fetch the base revision first.', file=sys.stderr)
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
