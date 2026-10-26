"""Reads IntelliJ editor color scheme XML files.

A scheme file has `<colors>` (named colors such as `CARET_ROW_COLOR`; an empty value means
"no color" and stops inheritance) and `<attributes>` (text attributes keys). An attribute is
`<option name="KEY"><value>` with `FOREGROUND`, `BACKGROUND`, `FONT_TYPE`, `EFFECT_COLOR`,
`EFFECT_TYPE`, `ERROR_STRIPE_COLOR` children, or `baseAttributes="OTHER_KEY"` (start from
another key). `parent_scheme` names the scheme everything undefined here comes from.

Attribute semantics used here (README.md): an attribute defined in a scheme replaces the
parent's attribute of the same name as a whole (fields it omits are unset, they do not
inherit field by field); `baseAttributes` is resolved by name through the same chain; the
colorblind variants (`deuteranopia`, `protanopia` XML attributes) are ignored.
"""

import os
import xml.etree.ElementTree as ET

# FONT_TYPE: 0 plain, 1 bold, 2 italic, 3 bold italic.
# EFFECT_TYPE values as they appear in the XML (checked against the data, see README):
EFFECT_NAMES = {
    0: "Boxed",
    1: "LineUnderscore",
    2: "WaveUnderscore",
    3: "Strikeout",
    4: "BoldLineUnderscore",
    5: "BoldDottedLine",
    6: "SearchMatch",
    7: "RoundedBox",
}


def parse_color(text):
    """Scheme colors are hex without `#`: up to six digits are RGB (so `0` is black and
    `FF00` is 00FF00), eight digits are RRGGBBAA. Returns 0xRRGGBBAA, or None when empty."""
    text = (text or "").strip()
    if not text:
        return None
    value = int(text, 16)
    if len(text) == 8:
        return value
    return (value << 8) | 0xFF


class Scheme:
    def __init__(self, key, parent):
        self.key = key
        self.parent = parent  # IntelliJ scheme name or None
        self.colors = {}  # name -> 0xRRGGBBAA | None
        self.attributes = {}  # name -> dict(fg, bg, font, effect_color, effect_type, stripe, base)


def _attribute(option):
    value = option.find("value")
    fields = {}
    if value is not None:
        for child in value.findall("option"):
            fields[child.get("name")] = child.get("value")
    base = option.get("baseAttributes")
    if value is None and base is None:
        return None  # `<option name="KEY"/>`: no definition here
    def color(name):
        return parse_color(fields.get(name))
    def integer(name):
        text = fields.get(name)
        return int(text) if text not in (None, "") else None
    return {
        "fg": color("FOREGROUND"),
        "bg": color("BACKGROUND"),
        "font": integer("FONT_TYPE") or 0,
        "effect_color": color("EFFECT_COLOR"),
        "effect_type": integer("EFFECT_TYPE"),
        "stripe": color("ERROR_STRIPE_COLOR"),
        "base": base,
    }


def parse_scheme_element(el):
    scheme = Scheme(el.get("name"), el.get("parent_scheme"))
    colors = el.find("colors")
    if colors is not None:
        for option in colors.findall("option"):
            scheme.colors[option.get("name")] = parse_color(option.get("value"))
    attributes = el.find("attributes")
    if attributes is not None:
        for option in attributes.findall("option"):
            attribute = _attribute(option)
            if attribute is not None:
                scheme.attributes[option.get("name")] = attribute
    return scheme


def load_schemes(root, specs):
    """{kit id: Scheme} for every spec of inputs.SCHEMES."""
    default_file = os.path.join(root, "platform/platform-resources/src/DefaultColorSchemesManager.xml")
    bundled = {}
    for el in ET.parse(default_file).getroot().iter("scheme"):
        bundled[el.get("name")] = parse_scheme_element(el)
    out = {}
    for spec in specs:
        if spec["file"] is None:
            scheme = bundled[spec["key"]]
        else:
            scheme = parse_scheme_element(ET.parse(os.path.join(root, spec["file"])).getroot())
        out[spec["id"]] = scheme
    return out


class Chain:
    """Resolves colors and attributes of one scheme through its `parent_scheme` chain."""

    def __init__(self, schemes, specs, scheme_id):
        by_key = {spec["key"]: spec["id"] for spec in specs}
        self.layers = []  # leaf first
        current = scheme_id
        while current is not None:
            scheme = schemes[current]
            self.layers.append(scheme)
            parent = scheme.parent
            if parent is None:
                break
            if parent not in by_key:
                raise KeyError(f"scheme {scheme.key!r}: unknown parent_scheme {parent!r}")
            current = by_key[parent]

    def color(self, name):
        for layer in self.layers:
            if name in layer.colors:
                return layer.colors[name]
        return None

    def _own(self, name, start):
        for index in range(start, len(self.layers)):
            if name in self.layers[index].attributes:
                return index, self.layers[index].attributes[name]
        return None, None

    def attribute(self, name, depth=0):
        """Resolved attribute dict (without `base`) or None."""
        index, own = self._own(name, 0)
        if own is None:
            return None
        result = {k: own[k] for k in ("fg", "bg", "font", "effect_color", "effect_type", "stripe")}
        base = own["base"]
        if base and depth < 6:
            inherited = self.attribute(base, depth + 1)
            if inherited:
                for k, v in inherited.items():
                    if result[k] in (None, 0) and v not in (None,):
                        result[k] = v
        return result

    def names(self):
        out = set()
        for layer in self.layers:
            out |= set(layer.attributes)
        return sorted(out)

    def color_names(self):
        out = set()
        for layer in self.layers:
            out |= set(layer.colors)
        return sorted(out)
