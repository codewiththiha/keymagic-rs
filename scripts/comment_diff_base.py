"""Choose the comment-check base for pushes and history rewrites."""
import subprocess
import sys


def git(*args):
    return subprocess.run(['git', *args], capture_output=True, text=True)


def main():
    if len(sys.argv) != 4:
        print('Expected event, previous SHA, and head SHA.', file=sys.stderr)
        return 2
    event, previous, head = sys.argv[1:]
    if git('cat-file', '-e', f'{head}^{{commit}}').returncode:
        print('Head commit is unavailable.', file=sys.stderr)
        return 1
    has_previous = previous and not git('cat-file', '-e', f'{previous}^{{commit}}').returncode
    if event != 'workflow_dispatch' and has_previous:
        result = git('merge-base', previous, head)
        if result.returncode:
            print('No common ancestor for comment checks.', file=sys.stderr)
            return 1
    else:
        if event != 'workflow_dispatch':
            print('Previous commit unavailable; checking the last commit.', file=sys.stderr)
        result = git('rev-parse', '--verify', f'{head}^')
        if result.returncode:
            result = git('hash-object', '-t', 'tree', '/dev/null')
    if result.returncode:
        print('Could not resolve comment-check base.', file=sys.stderr)
        return 1
    print(result.stdout.strip())
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
