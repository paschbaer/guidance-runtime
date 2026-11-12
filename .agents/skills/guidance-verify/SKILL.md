---
name: guidance-verify
description: Read-only verification of a project's .guidance/ tree via setup_guidance_verify — checks manifest completeness, file sizes, path traversal, and schema drift against the server's tool catalog. Use when the user asks to "verify the guidance config", after setup_guidance_generate writes files, when troubleshooting submission_invalid or schema-drift warnings, or before starting guidance sessions in a fresh repo.
---

# Guidance Verify — Config-Baum read-only verifizieren

## When to Use

- Nach `setup_guidance_generate`: geschriebenen Baum reconcilen, BEVOR „done" gemeldet wird (Write-Contract).
- troubleshooting: `submission_invalid`-Fehler, Schema-Drift-Warnungen in der `start_workflow`-Response, oder Verdacht auf fehlende/beschädigte `.guidance/`-Dateien.
- Ein frisch onboardiertes Repo soll vor der ersten Session geprüft werden.

## Never

- Do NOT modify files during verify — das Tool ist strikt read-only; Reparaturen (falls nötig) separat über `setup_guidance_generate` fahren.
- Do NOT interpret ein leeres `schemaDrift: []` als Beweis, dass JEDE Variant-Datei gescannt wurde — unlesbare Workflow-Dateien werden still übersprungen (Existenz/Parse prüft verify separat über das Manifest).

## Procedure

1. **Aufruf**: `setup_guidance_verify { manifest?, guidanceDir?, fileSetHash? }`
   - `guidanceDir`: absoluter oder projekt-relativer Pfad zum `.guidance/`-Verzeichnis; Default: `<workspace>/.guidance`.
   - `manifest`: optionale Dateiliste aus dem Generate-Output (Chunks liefern sie mit); ohne Manifest wird nur der Drift-Check gefahren.
   - `fileSetHash`: der Manifest-Hash aus dem Generate-Output (Integritätsbindung).
2. **Response lesen**:
   - `verified: true` + `missing: []` + `sizeMismatch: []` + `traversalRejected: []` = Baum vollständig.
   - `schemaDrift: []` = Phasen-Schemas aligniert mit dem Tool-Katalog (keine Legacy-Felder in `required`, keine uncovered required-Felder).
   - Bei Findings: jedes enthält Schema-Pfad, betroffenes Feld und Remedy (`setup_guidance_generate`).
3. **Bei Befunden**:
   - `missing`/`sizeMismatch`: fehlende Dateien über `setup_guidance_generate` neu erzeugen (Skill `guidance-generate`), dann verify wiederholen.
   - `schemaDrift`: betroffene Schema-Datei gemäß Remedy aktualisieren (Generate neu fahren oder Datei gezielt ersetzen), verify wiederholen.
4. **Remedy-Konvention**: Die Response trägt den kanonischen `remedy`-Text — diesen bei Weitergabe an den Operator wortgetreu verwenden.

## Notes

- Ein Session-Start mit `workflowId: "spec-kit-development"` fail-closed, bis jede referenzierte `submissionSchema`-Datei existiert — verify VOR dem ersten Spec-Kit-Session-Start fahren.
- Der Drift-Check vergleicht NUR `required`-Listen gegen den Katalog (Regel A: legacy-gestrippte Felder; Regel B: Katalog-Properties exklusive Dispatch-Keys `sessionId`/`requestId`/`includeGuidance`/`workspace`) — er ist advisory und ersetzt keine vollständige Config-Review.
- Runtime-Äquivalent: dieselben Findings erscheinen als `warnings[]` in der `start_workflow`-Response (nur wenn nicht-leer).
