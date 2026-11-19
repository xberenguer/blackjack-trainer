#!/usr/bin/env python3
"""Check project-owned Markdown links, image targets and GitHub heading anchors."""
import argparse
from html import unescape
from pathlib import Path
import re
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]

def prose(raw):
    return re.sub(r'^\s*(```|~~~).*?^\s*\1\s*$', '', raw, flags=re.MULTILINE | re.DOTALL)

def anchors(path):
    raw = prose(path.read_text())
    result = set(re.findall(r'\b(?:id|name)=["\']([^"\']+)', raw))
    used = {}
    for line in raw.splitlines():
        found = re.match(r'^#{1,6}\s+(.+?)\s*#*$', line)
        if not found:
            continue
        heading = unescape(re.sub(r'<[^>]+>', '', found[1])).lower()
        heading = re.sub(r'[^\w\s-]', '', heading)
        base = re.sub(r'\s', '-', heading)
        n = used.get(base, 0)
        result.add(base if n == 0 else base + '-' + str(n))
        used[base] = n + 1
    return result

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=ROOT)
    args = parser.parse_args()
    root = args.root.resolve()
    docs = [root/'README.md', *sorted((root/'docs').glob('*.md')), root/'browser/README.md', root/'firmware/README.md']
    issues = []
    checked = 0
    cache = {}
    for path in docs:
        if not path.is_file():
            issues.append(str(path.relative_to(root)) + ': missing document')
            continue
        raw = prose(path.read_text())
        targets = re.findall(r'\]\(([^)\n]+)\)', raw)
        targets += re.findall(r'<(?:img|a)\b[^>]*\b(?:src|href)=["\']([^"\']+)', raw)
        for target in targets:
            target = target.strip()
            target = target[1:target.index('>')] if target.startswith('<') and '>' in target else target.split()[0]
            parts = urlsplit(unescape(target))
            if parts.scheme or parts.netloc:
                continue
            checked += 1
            local = (path.parent / unquote(parts.path)).resolve() if parts.path else path
            try:
                local.relative_to(root)
            except ValueError:
                issues.append(f'{path.relative_to(root)}: link escapes repository: {target}')
                continue
            if not local.exists():
                issues.append(f'{path.relative_to(root)}: missing target: {target}')
            elif parts.fragment and local.suffix == '.md':
                if local not in cache:
                    cache[local] = anchors(local)
                if unquote(parts.fragment) not in cache[local]:
                    issues.append(f'{path.relative_to(root)}: missing anchor: {target}')
    if issues:
        print('\n'.join(issues))
        raise SystemExit(1)
    print(f'PASS {len(docs)} project documents and {checked} local links/images/anchors.')

if __name__ == '__main__':
    main()
