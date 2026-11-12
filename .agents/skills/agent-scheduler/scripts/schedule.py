#!/usr/bin/env python3
from __future__ import annotations
import argparse, json, subprocess, re
from pathlib import Path

def safe_id(x):
    if not re.fullmatch(r'T\d+',x): raise ValueError(f'unsafe task id: {x}')
    return x

def render_prompt(t):
    m=t['meta']; lines=[f"# Task {t['id']}", '', t['description'], '', '## Allowed write scope']
    lines += [f'- `{x}`' for x in m.get('write_scope',[])]
    lines += ['', '## Read scope']+[f'- `{x}`' for x in m.get('read_scope',[])]
    lines += ['', '## Non-goals']+[f'- {x}' for x in m.get('non_goals',[])]
    lines += ['', '## Validation']+[f'- `{x}`' for x in m.get('validation',[])]
    lines += ['', '## Execution mode', '', 'Execute this task as a guidance workflow per the `guidance-plain-workflow` skill:', '`start_workflow { workspace: {{GUIDANCE_WORKSPACE}}, request: <this task prompt> }`,', 'with plan-level submissions through the phase gates.', '', 'In your final message you MUST report the guidance workflow session id (or the exact start_workflow error if it failed) and the phase you reached. A result without a workflow session id is treated as a failed dispatch even if the code changes are green.', '', 'Implement only this task. Do not redesign public contracts. Stop after the declared validation succeeds or after the configured attempt limit.']
    return '\n'.join(lines)+'\n'

def run(argv,cwd,timeout=None,check=True):
    return subprocess.run(argv,cwd=cwd,text=True,capture_output=True,timeout=timeout,check=check)

def main():
    ap=argparse.ArgumentParser(); ap.add_argument('graph',type=Path); ap.add_argument('--repo',type=Path,required=True); ap.add_argument('--config',type=Path,required=True)
    mode=ap.add_mutually_exclusive_group(required=True); mode.add_argument('--dry-run',action='store_true'); mode.add_argument('--execute',action='store_true')
    ap.add_argument('--wave',type=int); ap.add_argument('--out-dir',type=Path,default=Path('scheduler-output')); args=ap.parse_args()
    graph=json.loads(args.graph.read_text()); cfg=json.loads(args.config.read_text());
    if not graph.get('valid'): raise SystemExit('Refusing invalid task graph')
    repo=args.repo.resolve(); out=args.out_dir.resolve(); (out/'prompts').mkdir(parents=True,exist_ok=True)
    tasks={t['id']:t for t in graph['tasks']}; waves=graph['waves']; selected=[]
    for i,w in enumerate(waves,1):
        if args.wave is None or args.wave==i: selected += [tasks[x] for x in w]
    plan=[]
    for t in selected:
        tid=safe_id(t['id'])
        prompt=(out/'prompts'/f'{tid}.md'); prompt.write_text(render_prompt(t),encoding='utf-8')
        wt=Path(cfg.get('worktree_root','../agent-worktrees')).resolve()/tid.lower(); branch=cfg.get('branch_prefix','agent/')+tid.lower()
        plan.append({'task_id':tid,'wave':t.get('wave'),'branch':branch,'worktree':str(wt),'prompt':str(prompt),'status':'pending'})
    (out/'schedule-plan.json').write_text(json.dumps(plan,indent=2),encoding='utf-8')
    if args.dry_run:
        print(json.dumps({'mode':'dry-run','tasks':len(plan),'plan':str(out/'schedule-plan.json')})); return 0
    maxp=int(cfg.get('max_parallel',3))
    if maxp>3: raise SystemExit('max_parallel must not exceed 3 orchestrator-spawned subagents')
    state=[]
    # Execution is orchestrator-driven: prepare worktrees, then the current agent
    # spawns at most `max_parallel` subagents (spawn_agent), each running its task
    # as a guidance workflow. No agent subprocesses are launched here.
    for item in plan:
        wt=Path(item['worktree']); wt.parent.mkdir(parents=True,exist_ok=True)
        if not wt.exists(): run(['git','worktree','add',str(wt),'-b',item['branch']],repo)
        state.append({**item,'status':'ready_for_dispatch'})
    (out/'run-state.json').write_text(json.dumps(state,indent=2),encoding='utf-8')
    print(json.dumps({'mode':'execute','tasks':len(state),'max_parallel':maxp,'ready_for_dispatch':len(state)}))
    return 0
if __name__=='__main__': raise SystemExit(main())
