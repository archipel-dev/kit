# Icons for agent sessions and transcripts

The IDE's Agents mode (an orchestrator for AI coding agents, with a native transcript built
from kit components) needs a small vocabulary of glyphs. This page maps every concept to a
glyph. A new glyph was drawn only where no existing one fit; everything else reuses the
set, because one concept has one glyph (see [design rules](design.md#metaphors)).

New glyphs are in the **Agents** category (11), plus `shield-check` (Data & network),
`deny` (Status), `mention` and `hash` (General). See the [catalog](catalog.md#agents).

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="img/agents.dark.png">
  <img alt="The Agents glyphs" src="img/agents.light.png" width="640">
</picture>

Working is not a glyph: the kit's progress component (spinner) shows it.

## Agents mode chrome

| Concept | Glyph | |
|---|---|---|
| Agent (an AI worker, not a person) | `agent` | new |
| Agents, the mode switch | `agents` | new |
| Task | `task` | new |
| Worktree (a branch with its own folder) | `worktree` | new |
| Fan-out (one prompt to several agents) | `fan-out` | new |
| Sub-agent spawned | `spawn` | new |
| Unread dot, badge | `unread` | new (filled, `Tone::Info`) |

## Session states

| Concept | Glyph | |
|---|---|---|
| Working | progress component | no glyph |
| Needs input, question | `help` | existing |
| Done | `success` | existing |
| Failed | `error` | existing |
| Stopped | `stop` | existing |
| Paused, interrupted | `pause` | existing |

## Transcript rows

| Concept | Glyph | |
|---|---|---|
| User prompt | `user` | existing |
| Assistant message | `sparkle` | existing |
| Thinking (collapsed reasoning) | `thinking` | new |
| Tool call, generic | `tool` | new |
| Read file | `file-text` | existing |
| Edit file | `edit` | existing |
| Write, new file | `new-file` | existing |
| Run command | `terminal` | existing |
| Search | `search` | existing |
| Web fetch | `globe` | existing |
| Spawn agent | `spawn` | new (same as the chrome glyph) |
| Permission request | `shield` | existing |
| Allow | `check` | existing |
| Deny | `deny` | new |
| Always allow | `shield-check` | new |
| Turn result, summary | `summary` | new |
| Token, cost | `tokens` | new |
| Attachment (`@file`) | `mention` | new |
| Issue (`#issue`) | `hash` | new |
| Send | `send` | existing |
| Stop generation | `stop` | existing |
| Retry | `refresh` | existing |
| Copy | `copy` | existing |
| Expand, collapse | `chevron-down`, `chevron-right`, `expand-all`, `collapse-all` | existing |
| Open diff, compare | `diff` | existing |
| Open in editor | `external-link` | existing |
| Merge | `merge` | existing |
| Discard | `delete` | existing |

## The new designs

- `agent`: a rounded head with two eyes and an antenna. The antenna is what separates it
  from a chat bubble or a face, and it carries no person shape, so it never reads as `user`.
- `agents`: a second head behind the first, the way "several of" is drawn for `copy`.
- `task`: a checklist with one done and one open item, so it reads as a plan rather than a
  single checkbox.
- `worktree`: the branch's two nodes joined to a folder, so it says "a branch that lives in
  its own directory".
- `fan-out`: a filled source and three targets on elbow lines. The source is filled so the
  direction reads at 16 px.
- `spawn`: a parent node with an elbow line to a smaller box, a child below it.
- `unread`: a filled dot in the info tone, the one status colour that is not red or amber.
- `thinking`: a pill with an ellipsis and a trailing ring, a thought rather than speech
  (compare `comment`, which has a pointed tail).
- `tool`: a wrench, outline only. `build` stays the hammer.
- `summary`: a bulleted list; the dots keep it apart from `menu`.
- `tokens`: a coin with a T.
- `shield-check`: `shield` with the `check` mark; `deny`: a circle with a slash, distinct
  from `close` (dismiss) and `error` (a failure).
- `mention` (`@`) and `hash` (`#`): the composer's triggers for attaching files and
  referencing issues.

The Nerd Font mapping of the terminal kit reuses the nearest existing characters for these
(the mapping only uses codepoints that are already verified, so robot, `@` and `#`
share the closest existing ones); see `crates/archipel-kit-tui/src/glyphs.rs`.
