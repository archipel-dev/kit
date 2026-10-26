"""Evaluates the mapping table (mapping.py) against resolved themes and schemes, and reads
the kit's measured palettes out of `theme.rs` for fallback and validation."""

import re


def mix(a, b, t):
    """Blend packed 0xRRGGBBAA colors, `t` toward `b`; keeps `a`'s alpha."""
    out = 0
    for shift in (24, 16, 8):
        x = (a >> shift) & 0xFF
        y = (b >> shift) & 0xFF
        v = int(x * (1 - t) + y * t + 0.5)
        out |= v << shift
    return out | (a & 0xFF)


def split_top(text, sep="|"):
    return text.split(sep)


def resolve_source(src, ui, scheme, default_scheme, values):
    """Returns (packed color or None, label)."""
    kind, _, arg = src.partition(":")
    if kind == "ui":
        value = ui.colors.get(arg) if ui else None
        return value, arg
    if kind == "scheme":
        value = default_scheme.color(arg) if default_scheme else None
        return value, f"scheme {arg}"
    if kind == "color":
        value = scheme.color(arg) if scheme else None
        return value, arg
    if kind == "attr":
        name, _, part = arg.rpartition(".")
        attribute = scheme.attribute(name) if scheme else None
        if attribute is None:
            return None, arg
        key = {"fg": "fg", "bg": "bg", "effect": "effect_color", "stripe": "stripe"}[part]
        return attribute[key], arg
    if kind == "self":
        return values.get(arg), f"= {arg}"
    if kind == "mix":
        a, b, t = split_top(arg)
        va, _ = resolve_source(a, ui, scheme, default_scheme, values)
        vb, _ = resolve_source(b, ui, scheme, default_scheme, values)
        if t.startswith("num:"):
            number = ui.numbers.get(t[4:]) if ui else None
            if number is None:
                return None, src
            t_value = 1.0 - float(number)  # `unselectedAlpha` is the opacity of the tab text
        else:
            t_value = float(t)
        if va is None or vb is None:
            return None, src
        return mix(va, vb, t_value), f"mix({a}, {b}, {t})"
    raise ValueError(f"unknown mapping source {src!r}")


def evaluate(entries, ui, scheme, default_scheme):
    """{field: (packed color | None, label | None)} in table order."""
    values, labels = {}, {}
    for field, sources in entries:
        values[field], labels[field] = None, None
        for src in sources:
            value, label = resolve_source(src, ui, scheme, default_scheme, values)
            if value is not None:
                values[field], labels[field] = value, label
                break
    return {f: (values[f], labels[f]) for f, _ in entries}


# ---------------------------------------------------------------------------------------
# The kit's measured palettes.
# ---------------------------------------------------------------------------------------

LITERAL = re.compile(r"^\s*(\w+):\s*(rgba?)\(0x([0-9A-Fa-f]+)\),?\s*$")
TINT = re.compile(r"^\s*(\w+):\s*Tint\s*\{\s*$")


def packed(fn, digits):
    value = int(digits, 16)
    return (value << 8) | 0xFF if fn == "rgb" else value


def measured_palettes(theme_rs):
    """Parses `Theme::dark()` / `Theme::light()` in theme.rs:
    {"dark": {"colors": {field: packed}, "editor": {...}}, "light": ...}. Nested tints are
    `banner_info.background`."""
    out = {}
    for name, key in (("dark", "dark"), ("light", "light")):
        start = theme_rs.index(f"pub fn {name}() -> Self")
        end = theme_rs.index("\n    }\n", start)
        body = theme_rs[start:end].splitlines()
        group, tint = None, None
        palette = {"colors": {}, "editor": {}}
        for line in body:
            stripped = line.strip()
            if stripped.startswith("colors: Colors {"):
                group = "colors"
            elif stripped.startswith("editor: EditorColors {"):
                group = "editor"
            elif m := TINT.match(line):
                tint = m.group(1)
            elif stripped.startswith("},") and tint is not None:
                tint = None
            elif m := LITERAL.match(line):
                if group is None:
                    continue
                field = f"{tint}.{m.group(1)}" if tint else m.group(1)
                palette[group][field] = packed(m.group(2), m.group(3))
        out[key] = palette
    return out


def struct_fields(theme_rs, struct):
    """Field names of `pub struct <struct>` in theme.rs."""
    start = theme_rs.index(f"pub struct {struct} {{")
    end = theme_rs.index("\n}\n", start)
    return re.findall(r"^\s+pub (\w+):", theme_rs[start:end], re.M)
