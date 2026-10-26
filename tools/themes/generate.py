#!/usr/bin/env python3
"""Generates the kit's IntelliJ theme and editor color scheme tables.

    python3 -I tools/themes/generate.py --community PATH --tag TAG [--commit SHA]

Reads only DATA files of an intellij-community checkout (`*.theme.json`, color scheme `*.xml`,
`DefaultColorSchemesManager.xml`, extension registration XML) and writes

    crates/archipel-kit/src/theme/generated/*.rs   the Rust tables
    docs/theme-mapping.md                          mapping, fallbacks, validation

The output is deterministic: running it twice yields identical bytes. See README.md.
"""

import argparse
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))  # `python -I` drops it

import emit_rust  # noqa: E402
import evaluate  # noqa: E402
import mapping  # noqa: E402
import report  # noqa: E402
import resolve_scheme  # noqa: E402
import resolve_ui  # noqa: E402
from inputs import SCHEMES, UI_THEMES  # noqa: E402

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
GENERATED = "crates/archipel-kit/src/theme/generated"
THEME_RS = "crates/archipel-kit/src/theme.rs"
DOC = "docs/theme-mapping.md"

# Validation pairs: (measured palette, generated theme) shown in docs/theme-mapping.md.
CHECKS = [
    (("Theme::dark()", "islands_dark"), "dark", "islands_dark"),
    (("Theme::light()", "islands_light"), "light", "islands_light"),
]


def luminance(rgba):
    r, g, b = ((rgba >> s) & 0xFF for s in (24, 16, 8))
    return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255


def commit_of(community, given):
    if given:
        return given
    try:
        out = subprocess.run(
            ["git", "-C", community, "rev-parse", "HEAD"], capture_output=True, text=True, check=True
        )
        return out.stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return "unknown"


def write(path, text, written):
    full = os.path.join(REPO, path)
    os.makedirs(os.path.dirname(full), exist_ok=True)
    with open(full, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    written.add(os.path.normpath(full))


def update_block(path, name, text):
    """Replaces the text between `<!-- generated:NAME:begin -->` and `...:end -->` markers."""
    full = os.path.join(REPO, path)
    with open(full, encoding="utf-8") as f:
        doc = f.read()
    begin, end = f"<!-- generated:{name}:begin -->", f"<!-- generated:{name}:end -->"
    if begin not in doc or end not in doc:
        sys.exit(f"{path}: missing the {begin} ... {end} markers")
    head, rest = doc.split(begin, 1)
    _, tail = rest.split(end, 1)
    with open(full, "w", encoding="utf-8", newline="\n") as f:
        f.write(f"{head}{begin}\n{text}\n{end}{tail}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--community", required=True, help="intellij-community checkout")
    parser.add_argument("--tag", required=True, help="tag of the checkout (recorded in the output)")
    parser.add_argument("--commit", help="commit of the checkout (default: git rev-parse HEAD)")
    args = parser.parse_args()
    meta = {"tag": args.tag, "commit": commit_of(args.community, args.commit)}

    with open(os.path.join(REPO, THEME_RS), encoding="utf-8") as f:
        theme_rs = f.read()
    measured = evaluate.measured_palettes(theme_rs)
    colors_fields = evaluate.struct_fields(theme_rs, "Colors")
    editor_fields = evaluate.struct_fields(theme_rs, "EditorColors")
    # `banner_*` are Tint structs: their sub-fields are mapped as `banner_info.background`.
    mapped_colors = {f.split(".")[0] for f, _ in mapping.COLORS}
    mapped_editor = {f for f, _ in mapping.EDITOR}
    for field in mapped_colors:
        if field not in colors_fields:
            sys.exit(f"mapping.py: Colors has no field {field!r}")
    for field in mapped_editor:
        if field not in editor_fields:
            sys.exit(f"mapping.py: EditorColors has no field {field!r}")
    unmapped = sorted(
        [f"Colors::{f}" for f in colors_fields if f not in mapped_colors and f not in mapping.UNMAPPED_COLORS]
        + [f"EditorColors::{f}" for f in editor_fields if f not in mapped_editor]
    )

    # Editor schemes.
    schemes = resolve_scheme.load_schemes(args.community, SCHEMES)
    id_by_key = {spec["key"]: spec["id"] for spec in SCHEMES}
    name_by_id = {spec["id"]: spec["name"] for spec in SCHEMES}
    scheme_results = []
    chains = {}
    for spec in SCHEMES:
        chain = resolve_scheme.Chain(schemes, SCHEMES, spec["id"])
        chains[spec["id"]] = chain
        mapped = evaluate.evaluate(mapping.EDITOR, None, chain, None)
        background = mapped["background"][0]
        dark = luminance(background) < 0.5 if background is not None else False
        parent = schemes[spec["id"]].parent
        parent_id = id_by_key[parent] if parent else None
        scheme_results.append(
            dict(
                spec=spec,
                scheme=schemes[spec["id"]],
                mapped=mapped,
                dark=dark,
                parent_id=parent_id,
                parent_name=name_by_id[parent_id] if parent_id else None,
            )
        )

    # UI themes.
    registry = resolve_ui.Registry(args.community)
    theme_results = []
    for spec in UI_THEMES:
        resolved = resolve_ui.resolve_theme(registry, spec["provider"])
        if "scheme" in spec:
            scheme_id = spec["scheme"]
        elif resolved.editor_scheme:
            scheme_id = id_by_key[resolved.editor_scheme]
        else:
            scheme_id = "darcula" if resolved.dark else "default"
        spec = dict(spec, scheme_id=scheme_id, dark=resolved.dark)
        mapped = evaluate.evaluate(mapping.COLORS, resolved, None, chains[scheme_id])
        theme_results.append(
            dict(
                spec=spec,
                resolved=resolved,
                mapped=mapped,
                scheme_id=scheme_id,
                scheme_name=name_by_id[scheme_id],
            )
        )

    # Write.
    written = set()
    for t in theme_results:
        text = emit_rust.emit_ui(t["spec"], t["resolved"], t["mapped"], meta)
        write(f"{GENERATED}/ui_{t['spec']['id']}.rs", text, written)
    for s in scheme_results:
        text = emit_rust.emit_scheme(s["spec"], s["scheme"], s["parent_id"], s["dark"], s["mapped"], meta)
        write(f"{GENERATED}/scheme_{s['spec']['id']}.rs", text, written)
    write(
        f"{GENERATED}/mod.rs",
        emit_rust.emit_index([t["spec"] for t in theme_results], SCHEMES, meta),
        written,
    )
    # Drop generated files of themes that are no longer shipped.
    gen_dir = os.path.join(REPO, GENERATED)
    for name in sorted(os.listdir(gen_dir)):
        path = os.path.normpath(os.path.join(gen_dir, name))
        if path not in written and name.endswith(".rs"):
            os.remove(path)
    write(
        DOC,
        report.build(meta, theme_results, scheme_results, measured, unmapped, CHECKS),
        written,
    )
    update_block(
        "docs/theming.md",
        "validation",
        report.validation_block(theme_results, scheme_results, measured, CHECKS),
    )
    print(f"wrote {len(written)} files ({len(theme_results)} themes, {len(scheme_results)} schemes)")


if __name__ == "__main__":
    main()
