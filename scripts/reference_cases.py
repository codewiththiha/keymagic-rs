"""Compare corpus expectations with a pinned upstream engine executable."""
import argparse
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CORPUS = ROOT / 'corpus'


def encode_events(events):
    encoded = []
    for event in events:
        key, vk = event.get('key'), event.get('vk')
        valid_key = isinstance(key, str) and len(key) == 1
        valid_vk = type(vk) is int and 0 <= vk <= 65535
        if 'modifiers' in event:
            modifiers = event['modifiers']
            flags = {'shift': 1, 'ctrl': 2, 'alt': 4, 'caps_lock': 8}
            if (not valid_vk or ('key' in event and not valid_key)
                    or not isinstance(modifiers, list)
                    or any(not isinstance(item, str) or item not in flags for item in modifiers)
                    or len(set(modifiers)) != len(modifiers)):
                raise ValueError(f'unsupported event: {event!r}')
            mask = sum(flags[item] for item in modifiers)
            encoded.append(f'm:{mask}:{vk}:{key if valid_key else ""}')
        elif valid_key and valid_vk:
            encoded.append(f'k:{vk}:{key}')
        elif valid_key and 'vk' not in event:
            encoded.append('c:' + key)
        elif valid_vk and 'key' not in event:
            encoded.append('v:' + str(vk))
        else:
            raise ValueError(f'unsupported event: {event!r}')
    return encoded


def decode_output(output):
    events = []
    for line in output.splitlines():
        if not line.startswith('CORPUS\t'):
            continue
        fields = line.split('\t')
        if (len(fields) != 6 or fields[1] not in ('none', 'insert', 'delete', 'replace')
                or fields[5] not in ('true', 'false')):
            raise ValueError('Invalid reference event')
        action, count, text, composing, processed = fields[1:]
        events.append({'action': action, 'delete': int(count),
                       'text': bytes.fromhex(text).decode('utf8'),
                       'composing': bytes.fromhex(composing).decode('utf8'),
                       'processed': processed == 'true'})
    return events


def compare_events(expected, actual):
    if len(expected) != len(actual):
        return [f'event count: expected {len(expected)}, got {len(actual)}']
    errors = []
    for index, (want, got) in enumerate(zip(expected, actual), start=1):
        for key in ('action', 'delete', 'text', 'composing', 'processed'):
            if key in want and want[key] != got[key]:
                errors.append(f'event {index}: {key} expected {want[key]!r}, got {got[key]!r}')
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('driver', type=Path)
    args = parser.parse_args()
    driver = args.driver.resolve(strict=True)
    count = 0
    errors = []
    for path in sorted((CORPUS / 'cases').glob('*.toml')):
        with path.open('rb') as file:
            cases = tomllib.load(file)['case']
        for case in cases:
            count += 1
            try:
                keys = encode_events(case['events'])
            except ValueError as error:
                errors.append(f"{case['name']}: {error}")
                continue
            keyboard = (CORPUS / case['keyboard']).resolve(strict=True)
            if not keyboard.is_relative_to(CORPUS.resolve()):
                errors.append(f"{case['name']}: keyboard outside corpus")
                continue
            result = subprocess.run([str(driver), str(keyboard), *keys],
                                    capture_output=True, text=True, timeout=60)
            if result.returncode:
                errors.append(f"{case['name']}: reference failed: {result.stderr.strip()}")
                continue
            try:
                differences = compare_events(case['events'], decode_output(result.stdout))
            except (ValueError, UnicodeError) as error:
                errors.append(f"{case['name']}: invalid reference output: {error}")
                continue
            errors.extend(f"{case['name']}: {error}" for error in differences)
    for error in errors:
        print(error, file=sys.stderr)
    print(f'Checked {count} cases; {len(errors)} issue(s).')
    return 1 if errors else 0


if __name__ == '__main__':
    raise SystemExit(main())
