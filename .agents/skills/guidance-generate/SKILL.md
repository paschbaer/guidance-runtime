---
name: guidance-generate
description: Generates (or refreshes) a project's .guidance/ configuration via setup_guidance_generate from wizard answers, including the chunked write protocol for large outputs. Use when the user asks to "generate the guidance config", "refresh the guidance config", "apply new guidance schemas", after a guidance-setup session collected answers, or when schema-drift warnings demand re-running setup_guidance_generate.
---

# Guidance Generate — Config-Dateien aus Wizard-Antworten erzeugen

## When to Use

- Ein `setup_guidance_start`/`setup_guidance_answer`-Wizard ist durchgelaufen und `nextTool: "setup_guidance_generate"` wird gemeldet.
- Eine Schema-Drift-Warnung (in der `start_workflow`-Response unter `warnings[]` oder als `schemaDrift` in `setup_guidance_verify`) nennt als Remedy „re-run setup_guidance_generate".
- Bestehende `.guidance/`-Dateien sollen auf den aktuellen Template-Stand gebracht werden (z. B. spec-kit Schema-Fixes).

## Never

- Do NOT answer wizard questions on the operator's behalf — sammle fehlende Antworten über `setup_guidance_start`/`setup_guidance_answer`, frage den Operator.
- Do NOT write files before collecting ALL answers — `setup_guidance_generate` lehnt unvollständige Antworten fail-closed ab (`setup answers incomplete`).
- Do NOT skip chunks — Teil-Overwrites erzeugen einen inkonsistenten Config-Baum.

## Procedure

1. **Answers sammeln**: Wenn keine vollständigen `answers` vorliegen, den Wizard fahren (`setup_guidance_start` → Fragen einzeln an den Operator → `setup_guidance_answer`). Die Response signalisiert `done: true` + `nextTool: "setup_guidance_generate"`.
2. **Generate aufrufen**:
   - Unchunked: `setup_guidance_generate { answers }` — Response enthält `files[]` (`path` + `content`), `notes[]` und einen Manifest-Hash.
   - Chunked (bei großen Outputs): `setup_guidance_generate { answers, cursor: "0" }` und jeden Chunk mit `nextCursor` nachziehen, bis `hasMore: false`. Jeder Chunk trägt einen Hash — der Folge-Chunk fail-closed bei Hash-Mismatch (Reihenfolge einhalten, nichts überspringen).
3. **WRITE CONTRACT (zwingend)**:
   - JEDE `files[]`-Entry unter `.guidance/` des Ziel-Repos schreiben (Pfade sind `.guidance/`-relativ).
   - Parent-Verzeichnisse VOR dem Schreiben anlegen (`workflows/`, `schemas/`, `schemas/spec-kit/`, `.guidance/scripts/`) — ein File-Tool ohne auto-mkdir hinterlässt sonst stille Lücken.
   - Im Chunked-Mode das Per-Chunk-Manifest (Pfad + UTF-8-Bytegröße) gegen die geschriebenen Dateien reconcilen, BEVOR „done" gemeldet wird.
4. **Verifizieren**: `setup_guidance_verify` (read-only) über den gesamten Baum fahren — siehe Skill `guidance-verify`. Erwartung: `verified: true`, `missing: []`, `sizeMismatch: []`, `schemaDrift: []`.
5. **Aktivierung**: Neue Sessions komponieren die neue Konfiguration (Configuration-Snapshot per Session); laufende Sessions rebinden beim nächsten Zugriff. Bei Bedarf Server neu starten.

## Notes

- Der Variant-Output enthält `workflows/spec-kit-development.json` + `schemas/spec-kit/*.schema.json` — Sessions für den artifact-bound Modus starten mit `workflowId: "spec-kit-development"`.
- Voraussetzung für den Spec-Kit-Modus: die speckit Commands (`/speckit-specify` …) müssen im Ziel-Repo verfügbar sein.
- Template-Inhalte werden zur Generate-Zeit von disk gelesen (`GUIDANCE_TEMPLATES_DIR`-Override möglich); die eingebettete pkg/-Baum ist nur Build-Fallback.
- Schema-Dateien der Templates werden fail-closed auf JSON-Parse-Validität geprüft — ein Generate-Fehler mit „template schema unparseable" ist ein Template-Defekt, kein Operator-Fehler.
