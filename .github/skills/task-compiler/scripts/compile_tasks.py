#!/usr/bin/env python3
from __future__ import annotations
import argparse, json, re, sys
from pathlib import Path
from fnmatch import fnmatch

TASK_RE = re.compile(r'^\s*- \[[ xX]\]\s+(T\d+)\s*(\[P\])?\s*(\[US\d+\])?\s*(.*)$')

# Native inline scheme used by specs/<id>/tasks.md:
#   "  - taskmeta: files=[...] deps=[...] exclusive=[...] est=30m"
INLINE_TASKMETA_RE = re.compile(r'^\s*-\s*taskmeta:\s*(.*)$')
# key=[...] or key=value; bracketed values may contain spaces.
NATIVE_TOKEN_RE = re.compile(r'([A-Za-z_][A-Za-z0-9_]*)=(\[[^\]]*\]|[^\s\]]+)')


def parse_list(body):
    """Split a bracketed list body. Comma-separated -> list; a comma-free body
    is kept as a single item (e.g. 'exclusive=[xtask rules]')."""
    body = body.strip()
    if not body:
        return []
    if ',' in body:
        return [x.strip().strip('"\'') for x in body.split(',') if x.strip()]
    return [body.strip().strip('"\'')]


def expand_id_range(tokens):
    """Expand range tokens like 'T024–T029' (en-dash/em-dash/hyphen) to the
    inclusive list of task ids."""
    out = []
    for tok in tokens:
        m = re.fullmatch(r'\s*(T\d+)\s*[–\-—]\s*(T\d+)\s*', tok)
        if m:
            a_str, b_str = m.group(1), m.group(2)
            a, b = int(a_str[1:]), int(b_str[1:])
            width = max(len(a_str) - 1, len(b_str) - 1)
            lo, hi = min(a, b), max(a, b)
            out.extend(f'T{i:0{width}d}' for i in range(lo, hi + 1))
        else:
            out.append(tok.strip())
    return out


def scalar(v):
    v = v.strip()
    if v.startswith('[') and v.endswith(']'):
        return parse_list(v[1:-1])
    if v.isdigit():
        return int(v)
    if v.lower() in ('true', 'false'):
        return v.lower() == 'true'
    return v.strip('"\'')


def parse_meta(lines):
    """Parse a fenced ```taskmeta block (key: value / '- item' lists)."""
    data = {}
    key = None
    for raw in lines:
        if not raw.strip():
            continue
        if re.match(r'^\s+-\s+', raw) and key:
            data.setdefault(key, []).append(re.sub(r'^\s+-\s+', '', raw).strip().strip('"\''))
            continue
        m = re.match(r'^\s*([A-Za-z_][A-Za-z0-9_-]*):\s*(.*)$', raw)
        if m:
            key = m.group(1)
            data[key] = scalar(m.group(2)) if m.group(2).strip() else []
    return data


def parse_native(rest):
    """Parse the inline 'taskmeta: files=[] deps=[] exclusive=[] est=NNm' body."""
    data = {}
    for m in NATIVE_TOKEN_RE.finditer(rest):
        key, val = m.group(1), m.group(2)
        if val.startswith('['):
            data[key] = parse_list(val[1:-1])
        elif key == 'est':
            data[key] = int(re.sub(r'[^0-9]', '', val) or '0')
        else:
            data[key] = val
    return data


def normalize(meta, fmt):
    """Map either scheme to a canonical meta dict consumed by the scheduler."""
    if fmt == 'native':
        return {
            'depends_on': expand_id_range(meta.get('deps', [])),
            'write_scope': meta.get('files', []),
            'read_scope': [],
            'exclusive_resources': meta.get('exclusive', []),
            'validation': [],
            'agent_profile': 'rust-builder',
            'estimated_minutes': meta.get('est', 30),
            'max_attempts': 2,
            'non_goals': [],
        }
    return {
        'depends_on': meta.get('depends_on', []),
        'write_scope': meta.get('write_scope', []),
        'read_scope': meta.get('read_scope', []),
        'exclusive_resources': meta.get('exclusive_resources', []),
        'validation': meta.get('validation', []),
        'agent_profile': meta.get('agent_profile', 'rust-builder'),
        'estimated_minutes': meta.get('estimated_minutes', 30),
        'max_attempts': meta.get('max_attempts', 2),
        'non_goals': meta.get('non_goals', []),
    }


def parse(path):
    lines = path.read_text(encoding='utf-8').splitlines()
    tasks = []
    i = 0
    n = len(lines)
    while i < n:
        m = TASK_RE.match(lines[i])
        if not m:
            i += 1
            continue
        tid, pm, story, desc = m.groups()
        meta, fmt = {}, None
        j = i + 1
        while j < n and not TASK_RE.match(lines[j]):
            if lines[j].strip() == '```taskmeta':
                k = j + 1
                while k < n and lines[k].strip() != '```':
                    k += 1
                meta, fmt = parse_meta(lines[j + 1:k]), 'fenced'
                j = k + 1
                break
            im = INLINE_TASKMETA_RE.match(lines[j])
            if im:
                meta, fmt = parse_native(im.group(1)), 'native'
                j += 1
                break
            j += 1
        tasks.append({
            'id': tid,
            'parallel_marked': bool(pm),
            'story': story[1:-1] if story else None,
            'description': desc.strip(),
            'meta': normalize(meta, fmt) if fmt else {},
            'has_meta': fmt is not None,
        })
        i = j
    return tasks


def overlap(a, b):
    for x in a:
        for y in b:
            if x == y or fnmatch(x, y) or fnmatch(y, x):
                return True
            xp = x.rstrip('*').rstrip('/')
            yp = y.rstrip('*').rstrip('/')
            if xp and yp and (xp.startswith(yp + '/') or yp.startswith(xp + '/')):
                return True
    return False


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('tasks_md', type=Path)
    ap.add_argument('--out-dir', type=Path, required=True)
    args = ap.parse_args()
    tasks = parse(args.tasks_md)
    errors = []
    warnings = []
    ids = {t['id'] for t in tasks}

    for t in tasks:
        tid = t['id']
        m = t['meta']
        if not t['has_meta']:
            errors.append(f"{tid}: missing taskmeta")
            continue
        for d in m['depends_on']:
            if d not in ids:
                errors.append(f"{tid}: unknown dependency {d}")
            if d == tid:
                errors.append(f"{tid}: self dependency")
        if t['parallel_marked'] and not m['write_scope']:
            errors.append(f"{tid}: [P] requires write_scope")
        if isinstance(m['estimated_minutes'], int) and m['estimated_minutes'] > 60:
            warnings.append(f"{tid}: estimated duration exceeds 60 minutes")
        if isinstance(m['max_attempts'], int) and m['max_attempts'] > 2:
            warnings.append(f"{tid}: max_attempts exceeds 2")
        if any(x in ('**', '*', '.') for x in m['write_scope']):
            warnings.append(f"{tid}: repository-wide write scope")

    deps = {t['id']: set(t['meta'].get('depends_on', [])) for t in tasks}
    remaining = set(ids)
    waves = []
    done = set()
    while remaining:
        ready = sorted(x for x in remaining if deps[x] <= done)
        if not ready:
            errors.append('dependency cycle detected among: ' + ', '.join(sorted(remaining)))
            break
        pending = ready
        while pending:
            wave = []
            next_pending = []
            for tid in pending:
                t = next(x for x in tasks if x['id'] == tid)
                conflict = False
                for wid in wave:
                    w = next(x for x in tasks if x['id'] == wid)
                    if overlap(t['meta'].get('write_scope', []), w['meta'].get('write_scope', [])) or \
                       set(t['meta'].get('exclusive_resources', [])) & set(w['meta'].get('exclusive_resources', [])):
                        conflict = True
                        break
                (next_pending if conflict else wave).append(tid)
            waves.append(wave)
            pending = next_pending
        done.update(ready)
        remaining -= set(ready)

    out = args.out_dir
    (out / 'manifests').mkdir(parents=True, exist_ok=True)
    wave_map = {tid: i + 1 for i, w in enumerate(waves) for tid in w}
    graph = {
        'source': str(args.tasks_md),
        'valid': not errors,
        'tasks': [],
        'waves': waves,
        'errors': errors,
        'warnings': warnings,
    }
    for t in tasks:
        rec = {**t, 'wave': wave_map.get(t['id'])}
        graph['tasks'].append(rec)
        (out / 'manifests' / f"{t['id']}.json").write_text(json.dumps(rec, indent=2), encoding='utf-8')
    (out / 'task-graph.json').write_text(json.dumps(graph, indent=2), encoding='utf-8')
    (out / 'execution-waves.md').write_text(
        '# Execution waves\n\n' + ''.join(
            f"## Wave {i}\n\n" + '\n'.join(f'- {x}' for x in w) + '\n\n'
            for i, w in enumerate(waves, 1)
        ),
        encoding='utf-8',
    )
    report = ['# Validation report', '', f"Status: {'PASS' if not errors else 'FAIL'}", '',
              '## Errors'] + [f'- {x}' for x in errors or ['None']] + ['',
              '## Warnings'] + [f'- {x}' for x in warnings or ['None']]
    (out / 'validation-report.md').write_text('\n'.join(report) + '\n', encoding='utf-8')
    print(json.dumps({'valid': not errors, 'tasks': len(tasks), 'waves': len(waves), 'out_dir': str(out)}))
    return 0 if not errors else 2


if __name__ == '__main__':
    raise SystemExit(main())
