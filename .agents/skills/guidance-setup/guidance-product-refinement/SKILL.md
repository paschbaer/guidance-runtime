---
name: guidance-product-refinement
version: 2.0.0
language: en
summary: Interactive Product-to-Feature refinement and controlled Spec-Kit handoff without manual stage-prompt copying.
description: Interactive Product-to-Feature refinement and controlled Spec-Kit handoff — drives all refinement stages (Product → Epic → Capability → Feature Candidate) and the Spec-Kit feature handoff automatically, with iterative generate/audit/revise cycles, persistent findings, and automatic approval only after zero open blocking findings.
---

# Guidance Product Refinement

## Default Entry Point

Use `references/initial-workflow-prompt.md`. It discovers available inputs, asks one grouped questionnaire for missing blocking values, and then drives all refinement or handoff stages automatically.

## Supported Workflows

- Product → Epic Set
- Epic → Capability Set
- Capability → Feature Candidate Set
- approved Feature Candidate → Spec-Kit Feature Specification **or** handoff to `/guidance-speckit-development` (operator choice)

## Mandatory Behavior

- Complete Child Sets, not isolated children
- Iterative generate/audit/revise cycles
- Automatic approval only with zero open blocking findings and completed deterministic checks
- Persistent findings and revision history
- English repository artifacts
- Managed writes only inside the specification subrepository
- No manual copying of internal stage prompts
- At handoff, ask the operator which target to use (direct Spec-Kit feature specification or `/guidance-speckit-development`) unless the request already specifies one; WAIT for the answer — do not pick a target on the operator's behalf

## Resources

- `references/initial-workflow-prompt.md`: operator entry point
- `references/stage-contracts.md`: normative internal stage behavior
- `docs/user-guide-en.md`: English user guide
- `templates/run-manifest.yaml`, `findings.yaml`, `approval.yaml`: persistence templates
