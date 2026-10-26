"""Resolves IntelliJ UI theme JSON files into flat key tables.

Semantics (documented in README.md, validated against the measured Islands Dark/Light):

1. `parentTheme` chains are merged at the data level, root first: `colors` and `ui` are
   flattened to dotted keys and a child key replaces the parent's key. A child that
   redefines a palette name therefore also changes every parent entry that uses the name.
2. Palette (`colors`) values are `#RGB`, `#RRGGBB`, `#RRGGBBAA` (alpha last) or the name
   of another palette entry.
3. A `ui` value is a color literal, a palette name, the name of another `ui` key (alias),
   a number, a boolean, an insets/size string (`"4,12,4,12"`, `"72,28"`), a Java class
   name (ignored) or an object keyed by OS (`os.linux` is taken, then `os.default`).
4. `"*": {"prop": value}` sets `Component.prop` for every component the theme does not set
   `Component.prop` for itself. IntelliJ applies it to the UIManager keys the base look and
   feel defined, which live in Java code; the component set here is Swing's standard
   component names plus every top-level component name a shipped theme mentions.
"""

import json
import os
import re
import xml.etree.ElementTree as ET

from inputs import EXTENSIONS, EXTRA_THEME_FILES, PLUGIN_THEME_ROOTS

# Swing's standard component names (the public `UIDefaults` prefixes).
SWING_COMPONENTS = [
    "Button", "CheckBox", "CheckBoxMenuItem", "ColorChooser", "ComboBox", "DesktopIcon",
    "DesktopPane", "EditorPane", "FileChooser", "FormattedTextField", "InternalFrame",
    "Label", "List", "Menu", "MenuBar", "MenuItem", "OptionPane", "Panel", "PasswordField",
    "PopupMenu", "ProgressBar", "RadioButton", "RadioButtonMenuItem", "RootPane",
    "ScrollBar", "ScrollPane", "Separator", "Slider", "Spinner", "SplitPane", "TabbedPane",
    "Table", "TableHeader", "TextArea", "TextField", "TextPane", "TitledBorder",
    "ToggleButton", "ToolBar", "ToolTip", "Tree", "Viewport",
]

HEX = re.compile(r"^#([0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$")
JAVA_CLASS = re.compile(r"^(com|org|java|javax)\.[\w.$]+$")
NUMERIC_STRING = re.compile(r"^-?[\d.]+(,\s*-?[\d.]+)*$")
ICON_REF = re.compile(r"^(AllIcons\.[\w.]+|/[\w/.\-]+\.(svg|png))$")  # icon references
BORDER_SPEC = re.compile(r"^\d+(,\d+){3},#[0-9a-fA-F]{6,8}$")  # "1,1,1,1,#4C4F56": insets + color


def parse_hex(text):
    """`#RGB`, `#RRGGBB` or `#RRGGBBAA` to a packed 0xRRGGBBAA int, else None."""
    m = HEX.match(text)
    if not m:
        return None
    h = m.group(1)
    if len(h) in (3, 4):
        h = "".join(c * 2 for c in h)
    if len(h) == 6:
        h += "ff"
    return int(h, 16)


def pick_os(value):
    """Linux first (the kit is cross-platform; Linux/default is the neutral look)."""
    for key in ("os.linux", "os.default", "os.windows", "os.mac"):
        if key in value:
            return value[key]
    return None


def flatten(node, prefix, out):
    """Nested theme objects to dotted keys. `*` stays a segment (`*.background`)."""
    for key, value in node.items():
        if key.startswith("_"):  # `_comment`
            continue
        path = f"{prefix}.{key}" if prefix else key
        if isinstance(value, dict):
            if any(k.startswith("os.") for k in value):
                picked = pick_os(value)
                if picked is not None:
                    out[path] = picked
            else:
                flatten(value, path, out)
        else:
            out[path] = value


class Registry:
    """Finds theme files by their IntelliJ theme id (`parentTheme` refers to these)."""

    def __init__(self, root):
        self.root = root
        self.files = dict(EXTRA_THEME_FILES)
        for rel in EXTENSIONS:
            tree = ET.parse(os.path.join(root, rel))
            for el in tree.iter("themeProvider"):
                path = el.get("path").lstrip("/")
                base = PLUGIN_THEME_ROOTS.get(el.get("id"))
                if base:
                    self.files[el.get("id")] = f"{base}/{path}"
                else:
                    self.files[el.get("id")] = f"platform/platform-resources/src/{path}"

    def load(self, provider):
        rel = self.files[provider]
        with open(os.path.join(self.root, rel), encoding="utf-8") as f:
            return rel, json.load(f)

    def chain(self, provider):
        """[(provider, file, json)] from the root parent down to `provider`."""
        out = []
        while provider:
            rel, data = self.load(provider)
            out.append((provider, rel, data))
            provider = data.get("parentTheme")
        return out[::-1]


class Resolved:
    """A fully merged and resolved theme."""

    def __init__(self):
        self.name = ""
        self.dark = False
        self.editor_scheme = None
        self.chain = []  # [(provider, file)]
        self.colors = {}  # key -> 0xRRGGBBAA
        self.numbers = {}  # key -> int | float
        self.flags = {}  # key -> bool
        self.strings = {}  # key -> "a,b,c,d" insets / "w,h" sizes
        self.unresolved = {}  # key -> raw value that is neither color, key, number nor class
        self.stars = {}  # property -> (kind, value)
        self.star_keys = set()  # keys that came from a star entry
        self.palette = {}


def _flat_ui(data):
    out = {}
    flatten(data.get("ui", {}), "", out)
    return out


# Properties Swing's own defaults define for every component; a star entry for one of these
# also reaches components the theme files never name.
SWING_PROPS = {
    "background", "foreground", "selectionBackground", "selectionForeground", "disabledText",
    "inactiveForeground", "inactiveBackground", "caretForeground",
}


def resolve_theme(registry, provider):
    res = Resolved()
    palette_raw = {}
    state = {}  # key -> raw value, layer by layer
    star_set = set()  # keys whose current value came from a star entry
    chain = registry.chain(provider)
    # The root theme's star entries exist to reach every component: its properties apply to
    # Swing's components as well as to keys some theme names.
    root_stars = {k[2:] for k in _flat_ui(chain[0][2]) if k.startswith("*.")}
    for pid, rel, data in chain:
        res.chain.append((pid, rel))
        res.name = data.get("name", res.name)
        res.dark = data.get("dark", res.dark)
        if "editorScheme" in data:
            res.editor_scheme = data["editorScheme"]
        flatten(data.get("colors", {}), "", palette_raw)
        ui = {}
        flatten(data.get("ui", {}), "", ui)
        stars = {k[2:]: v for k, v in ui.items() if k.startswith("*.")}
        # A layer's stars first: they reach every existing key ending in `.prop`, including
        # keys an earlier layer set explicitly. Then its explicit keys.
        for prop, raw in sorted(stars.items()):
            suffix = "." + prop
            targets = {k for k in state if k.endswith(suffix)}
            if prop in SWING_PROPS or prop in root_stars:
                targets |= {f"{c}.{prop}" for c in SWING_COMPONENTS}
            for key in targets:
                state[key] = raw
                star_set.add(key)
            res.stars[prop] = raw
        for key, raw in ui.items():
            if not key.startswith("*."):
                state[key] = raw
                star_set.discard(key)
    res.star_keys = star_set

    palette = res.palette

    def palette_color(name, seen=()):
        if name in palette:
            return palette[name]
        raw = palette_raw.get(name)
        if raw is None or name in seen:
            return None
        color = None
        if isinstance(raw, str):
            color = parse_hex(raw) or palette_color(raw, seen + (name,))
        palette[name] = color
        return color

    for name in palette_raw:
        palette_color(name)

    def value_of(raw, key, depth=0):
        if isinstance(raw, bool):
            return "flag", raw
        if isinstance(raw, (int, float)):
            return "number", raw
        if isinstance(raw, str):
            color = parse_hex(raw)
            if color is not None:
                return "color", color
            if palette.get(raw) is not None:
                return "color", palette[raw]
            if depth < 4 and raw in state and raw != key:
                return value_of(state[raw], raw, depth + 1)
            if NUMERIC_STRING.match(raw.strip()):
                return "string", raw.replace(" ", "")
        return None

    targets = {"color": res.colors, "number": res.numbers, "flag": res.flags, "string": res.strings}
    for key, raw in state.items():
        got = value_of(raw, key)
        if got is None:
            if isinstance(raw, str) and not (
                JAVA_CLASS.match(raw) or BORDER_SPEC.match(raw) or ICON_REF.match(raw)
            ):
                res.unresolved[key] = raw
            continue
        targets[got[0]][key] = got[1]
    return res
