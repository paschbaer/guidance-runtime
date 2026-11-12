---
name: guidance-batch-record-repair
description: Repariert einen fehlerhaft breiten Spec-Kit-Batch-Release-Record der Guidance-MCP-Engine (entstanden durch release_batch mode "allReady"), der das Independent-Review-Gate (prepare_task_review, FR-2001) fail-closed blockiert. Chirurgischer State-Edit mit Operator-Stop/Start-Handshake, Backup, JSON/Diff-Validierung. Use when prepare_task_review mit "task <ID> has status pending" ablehnt, obwohl der Batch korrekt ist, oder wenn ein release_batch mehr Tasks erfasst hat als geplant.
---

# Guidance Batch-Record-Reparatur (Spec-Kit-Task-Maschinerie)

Repariere einen über-breiten `batches[<batchId>].taskIds`-Record im
persistierten Spec-Kit-State der Guidance-Engine, ohne Task-Status oder
andere Batches anzutasten. Der Operator macht AUSSCHLIESSLICH zwei Dinge auf
deine explizite Aufforderung: **Guidance-Server STOPPEN** und **STARTEN**.
An diesen Punkten hältst du an und wartest auf Bestätigung. Alle anderen
Schritte führst du selbst aus.

## Hintergrund (verifiziert)

- Das Independent-Review-Gate `prepare_task_review { sessionId, batchId }`
  enumeriert `batches[<batchId>].taskIds` aus dem **Haupt-Session-State**
  und lehnt fail-closed ab ("task <ID> has status pending"), solange
  irgendein Task darin `pending` ist (FR-2001).
- `release_batch { mode: "allReady" }` schreibt ALLE fälligen Tasks in EINEN
  Record — niemals verwenden; nur `single`/`batch` (Partitionierung folgt
  den taskmeta-Abhängigkeiten).
- Der Review-Gate-Sidecar (`review-gate.json`, `taskIdsAtRelease`) ist vom
  Fehler NICHT betroffen und wird nicht angefasst.
- Es existiert KEIN `task.batchId`-Feld — Task-Objekte werden nie editiert.
- Der Server hält den State nicht dauerhaft im RAM: jeder Tool-Call lädt
  von der Platte und schreibt Mutationen zurück. Ein Datei-Edit während der
  Server gestoppt ist, ist die sichere Reparatur (In-Memory-Kopie des
  laufenden Prozesses überschreibt sonst den Patch).

## Phase 0 — Preflight (Server läuft; nur LESEN)

1. State-Datei lokalisieren: der Server läuft mit seinem eigenen CWD als
   State-Basis — typisch `D:\tools\guidance\.guidance\state\` (NICHT das
   Repo-`.guidance/`, dort liegen i. d. R. nur alte Sessions). Suche die
   Datei, die den `batchId` enthält:
   `spec-kit-states/session-<sessionId>.json`.
2. Per Python (json.load) den Ist-Zustand prüfen und notieren:
   - `batches[<batchId>].taskIds` (Ist, meist T<N>–T<M> überbreit),
   - `batches[<batchId>].status`,
   - `batches[<erfolgreiche-teilbatches>].taskIds` (referenz-ok),
   - Task-Status-Baseline via `Counter(t['status'] for t in d['tasks'].values())`,
   - `activeBatchId`.
3. Sicherstellen, dass es GENAU EINE Kandidatendatei ist.
4. Noch KEIN Backup, KEINE Änderung.
5. Meldung: „Bitte den Guidance-Server STOPPEN und mir bestätigen."

## ⛔ STOPP 1 — warte auf „Server gestoppt"
Keinen Guidance-Tool-Call ausführen (auch keine Statusabfragen).

## Phase 1 — Reparatur-Edit (Server aus)

1. Backup: `<datei>.pre-repair-<kuerzel>-bak` auf gleicher Ebene (cp).
2. Surgischer **Text-Replace** (NICHT json.load/dump — das reformatiert die
   ganze Datei): das exakte `taskIds`-Array des Ziel-Batches als
   whitespace-treuen Block ersetzen; `assert s.count(old) == 1` vor dem
   Replace; Newline-Handling via `newline=''` bewahren.
3. Validierung:
   a) JSON-Parstest der GESAMTEN Datei,
   b) Ziel-`taskIds` == Ziel-Liste, `status` unverändert,
   c) alle anderen Batches gegen Soll-Listen geassertet,
   d) Task-Anzahl (38 o. ä.) + Status-Baseline unverändert,
   e) `activeBatchId` unverändert,
   f) unified Diff gegen das Backup: genau EINE logische Änderung, nur im
      Ziel-Array (Komma-Behandlung beachten).
4. Meldung: „Reparatur-Edit abgeschlossen und validiert. Bitte den
   Guidance-Server STARTEN und mir bestätigen."

## ⛔ STOPP 2 — warte auf „Server gestartet"

## Phase 2 — Verifikation

1. `get_spec_kit_status` (detail summary): Task-Baseline unverändert
   (completed/implemented/pending), `integrity.ok: true`, Session aktiv.
   Hinweis: das Sidecar `reviewGate.batches[<id>].taskIdsAtRelease` kann
   STALE bleiben — der Gate validiert den Haupt-State; staler Sidecar ist
   dokumentiert und unschädlich.
2. Backup-Datei erst löschen, NACHDEM die Baseline bestätigt ist (oder als
   Beweis liegen lassen, wenn Residuen dokumentiert werden).

## Phase 3 — Weiterarbeit (unverändert)

- Batch normal abschließen: implement → `submit_task_implementation` →
  `prepare_task_review` (GENAU EINMAL pro Token-Reason) → Independent
  Review via frischen `spawn_agent`-Subagent →
  `submit_independent_review { reviewToken }` → `verify_task` × N →
  `complete_task` × N.
- Zukünftige `release_batch`-Calls: nur `single`/`batch` — NIEMALS `allReady`.
- Bei fixRequired-Findings: Fix → `submit_task_implementation`
  (Re-Submission setzt den Review-Zustand zurück) → `prepare_task_review`
  (reason "re-review") → Resolution über neuen Token — NIEMALS den
  verbrauchten Token wiederverwenden (`review_token_invalid`, single-claim).

## Verifikations-Werkzeuge (Windows-Host)

- Python one-liners statt jq (jq nicht garantiert installiert).
- `rg`/`grep -c` für Trefferzählung; `git --no-pager` für Diffs im Repo.
- Beispieldatei-Pfade: siehe Phase 0; Session-Dateien zusätzlich unter
  `.guidance/state/sessions/` (Workflow-Phasen) und
  `.guidance/state/spec-kit-states/` (Batch/Task-State) — NUR letztere
  enthält `batches`.

## Bekannte Fallstricke

- `release_batch { mode: "allReady" }` ist die Ursache — nie verwenden.
- Weitere verifizierte Ursache für einen blockierten Batch: ein ZWEITER
  `release_batch`-Call vor Abschluss des ersten Batches. Jeder Release setzt
  `activeBatchId` neu (lifecycle.rs), nichts löscht ihn je — der erste Batch
  wird unerreichbar (`start_task`: "another batch is active"), und seine
  bereits releaseden Task-IDs können nicht re-released werden. Reparatur:
  surgischer Edit des Feldes `"activeBatchId"` auf die Ziel-`batchId` — das
  restliche Record-Set (batches, tasks, review-gate Sidecar) bleibt unangetastet.
- Nach dem Server-Neustart lädt die Session den gepatchten State; die
  In-Memory-Kopie des LAUFENDEN Prozesses überschreibt Disk-Edits bei
  jedem Tool-Call — deshalb Stop-before-Edit.
- `prepare_task_review` minted Token single-claim; ein zweiter Call
  invalidiert den ersten unclaimed Token (FR-2016) — nie doppelt rufen.
- Offene `fix_required`-Findings blockieren `complete_task` bis zur
  formellen Resolution über einen FRISCHEN Reviewer-Token (Re-Review).
