#!/usr/bin/env python3
"""Check that every commit in a revision range has the owner's author email."""
import argparse
import subprocess
import sys

from check_commit_messages import git

OWNER_EMAIL = 'kt007441@gmail.com'


def author_problem(email: str) -> str | None:
    if email.strip().casefold() != OWNER_EMAIL.casefold():
        return f'author email must be {OWNER_EMAIL}'
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--range', required=True, dest='revision_range')
    args = parser.parse_args()
    try:
        shas = git('rev-list', '--reverse', args.revision_range).splitlines()
        failed = False
        for sha in shas:
            problem = author_problem(git('show', '-s', '--format=%ae', sha))
            if problem:
                print(f'{sha[:12]}: {problem}', file=sys.stderr)
                failed = True
        if not failed:
            print(f'Checked {len(shas)} commit author(s).')
        return int(failed)
    except subprocess.CalledProcessError:
        print('Unable to read commit range; fetch the base revision first.', file=sys.stderr)
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
