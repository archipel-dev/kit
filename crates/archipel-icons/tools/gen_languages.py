#!/usr/bin/env python3
"""Generates `src/languages.rs` from GitHub Linguist's `languages.yml` (MIT).

    curl -sSfO https://raw.githubusercontent.com/github-linguist/linguist/main/lib/linguist/languages.yml
    python3 tools/gen_languages.py languages.yml > src/languages.rs

The list below picks the languages that get an icon, their enum name and their label (one
to three characters). Order matters: when two languages claim the same
extension, the earlier one wins.
"""
import hashlib
import sys

import yaml

# (Linguist name, Rust variant, label, pixel label for the stroke-font tile style)
LANGUAGES = [
    ("Rust", "Rust", "Rs", "RS"),
    ("Java", "Java", "J", "JA"),
    ("Kotlin", "Kotlin", "Kt", "KT"),
    ("Scala", "Scala", "Sc", "SC"),
    ("Groovy", "Groovy", "Gr", "GR"),
    ("Gradle", "Gradle", "Gdl", "GD"),
    ("Gradle Kotlin DSL", "GradleKotlin", "Gk", "GK"),
    ("Clojure", "Clojure", "Clj", "CJ"),
    ("Python", "Python", "Py", "PY"),
    ("Ruby", "Ruby", "Rb", "RB"),
    ("PHP", "Php", "Php", "PH"),
    ("Perl", "Perl", "Pl", "PL"),
    ("Lua", "Lua", "Lua", "LU"),
    ("JavaScript", "JavaScript", "JS", "JS"),
    ("TypeScript", "TypeScript", "TS", "TS"),
    ("TSX", "Tsx", "TSX", "TX"),
    ("Go", "Go", "Go", "GO"),
    ("C", "C", "C", "C"),
    ("C++", "Cpp", "C++", "C+"),
    ("C#", "CSharp", "C#", "C#"),
    ("F#", "FSharp", "F#", "F#"),
    ("Objective-C", "ObjectiveC", "ObC", "OC"),
    ("Swift", "Swift", "Sw", "SW"),
    ("Dart", "Dart", "Dt", "DT"),
    ("Zig", "Zig", "Zig", "ZG"),
    ("Nim", "Nim", "Nim", "NM"),
    ("Crystal", "Crystal", "Cr", "CR"),
    ("D", "D", "D", "D"),
    ("Elixir", "Elixir", "Ex", "EX"),
    ("Erlang", "Erlang", "Erl", "ER"),
    ("Haskell", "Haskell", "Hs", "HS"),
    ("OCaml", "OCaml", "ML", "ML"),
    ("Elm", "Elm", "Elm", "EL"),
    ("Julia", "Julia", "Jl", "JL"),
    ("R", "R", "R", "R"),
    ("MATLAB", "Matlab", "M", "MT"),
    ("Fortran", "Fortran", "F", "FT"),
    ("Pascal", "Pascal", "Pas", "PA"),
    ("Ada", "Ada", "Ada", "AD"),
    ("Prolog", "Prolog", "Pro", "PG"),
    ("Racket", "Racket", "Rkt", "RK"),
    ("Scheme", "Scheme", "Scm", "SM"),
    ("Common Lisp", "CommonLisp", "Lsp", "LI"),
    ("Assembly", "Assembly", "Asm", "AS"),
    ("WebAssembly", "WebAssembly", "Wa", "WA"),
    ("Solidity", "Solidity", "Sol", "SO"),
    ("Vim script", "VimScript", "Vim", "VI"),
    ("Shell", "Shell", ">_", "SH"),
    ("PowerShell", "PowerShell", "PS", "PS"),
    ("Batchfile", "Batchfile", "Bat", "BT"),
    ("SQL", "Sql", "SQL", "SQ"),
    ("GraphQL", "GraphQl", "GQL", "GQ"),
    ("Protocol Buffer", "Protobuf", "Pb", "PB"),
    ("HCL", "Hcl", "TF", "TF"),
    ("Nix", "Nix", "Nix", "NX"),
    ("Dockerfile", "Dockerfile", "Dk", "DK"),
    ("Makefile", "Makefile", "Mk", "MK"),
    ("CMake", "CMake", "CM", "CM"),
    ("Starlark", "Starlark", "Bzl", "BZ"),
    ("HTML", "Html", "<>", "HT"),
    ("CSS", "Css", "#", "CS"),
    ("SCSS", "Scss", "Ss", "SS"),
    ("Less", "Less", "Ls", "LS"),
    ("Vue", "Vue", "Vue", "VU"),
    ("Svelte", "Svelte", "Sv", "SV"),
    ("XML", "Xml", "</>", "<>"),
    ("Markdown", "Markdown", "Md", "MD"),
    ("reStructuredText", "ReStructuredText", "Rst", "RT"),
    ("TeX", "Tex", "TeX", "TE"),
    ("AsciiDoc", "AsciiDoc", "Ad", "AC"),
    ("JSON", "Json", "{}", "{}"),
    ("YAML", "Yaml", "Yml", "YM"),
    ("TOML", "Toml", "Tml", "TO"),
    ("INI", "Ini", "Ini", "IN"),
    ("CSV", "Csv", "Csv", "CV"),
    ("Java Properties", "Properties", "Prp", "PR"),
    ("Ignore List", "IgnoreList", "Ig", "GI"),
    ("EditorConfig", "EditorConfig", "Ec", "EC"),
]

FALLBACK = "#6F737A"


def luminance(hex_color):
    def channel(v):
        v = v / 255
        return v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4

    r, g, b = (int(hex_color[i : i + 2], 16) for i in (1, 3, 5))
    return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)


def contrast(a, b):
    la, lb = luminance(a), luminance(b)
    return (max(la, lb) + 0.05) / (min(la, lb) + 0.05)


def rust_str(value):
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def main(path):
    raw = open(path, "rb").read()
    data = yaml.safe_load(raw)
    claimed_ext, claimed_name = {}, {}
    rows = []
    for name, variant, label, pixel in LANGUAGES:
        info = data[name]
        color = info.get("color", FALLBACK)
        # Label color: white or near-black, whichever reads better on the tile.
        dark_label = contrast(color, "#000000") > contrast(color, "#FFFFFF")
        exts = []
        for ext in info.get("extensions", []):
            ext = ext.lower().lstrip(".")
            if ext not in claimed_ext:
                claimed_ext[ext] = variant
                exts.append(ext)
        names = []
        for file_name in info.get("filenames", []):
            if file_name not in claimed_name:
                claimed_name[file_name] = variant
                names.append(file_name)
        rows.append((name, variant, label, color.upper(), dark_label, exts, names, info["type"], pixel))

    sha = hashlib.sha256(raw).hexdigest()[:16]
    out = [
        "//! Programming languages and file formats with an icon.",
        "//!",
        "//! Generated by `tools/gen_languages.py` from GitHub Linguist's `languages.yml`",
        f"//! (MIT, sha256 {sha}…); do not edit by hand.",
        "",
        "/// A programming language or file format with a file icon: a rounded tile in the",
        "/// language's color with a one- or two-letter label.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]",
        "pub enum Language {",
    ]
    for name, variant, *_ in rows:
        out.append(f"    /// {name}.")
        out.append(f"    {variant},")
    out += ["}", "", "impl Language {", "    /// Every language, in catalog order.", "    pub const ALL: &'static [Language] = &["]
    out += [f"        Language::{v}," for _, v, *_ in rows]
    out += ["    ];", ""]

    def table(fn, doc, ty, value):
        lines = [f"    /// {doc}", f"    pub const fn {fn}(self) -> {ty} {{", "        match self {"]
        for row in rows:
            lines.append(f"            Language::{row[1]} => {value(row)},")
        lines += ["        }", "    }", ""]
        return lines

    out += table("name", "Display name (as GitHub Linguist names it).", "&'static str", lambda r: rust_str(r[0]))
    out += table("label", "The label on the icon band.", "&'static str", lambda r: rust_str(r[2]))
    out += table(
        "pixel_label",
        "Capital label for the pixel-tile style (one or two characters of the stroke font).",
        "&'static str",
        lambda r: rust_str(r[8]),
    )
    out += table("color", "Tile color as `0xRRGGBB` (GitHub Linguist's language color).", "u32", lambda r: "0x" + r[3][1:])
    out += table("dark_label", "Whether the label is dark (on light bands) rather than white.", "bool", lambda r: "true" if r[4] else "false")
    out += table(
        "kind",
        "Programming language, markup, data or prose.",
        "LanguageKind",
        lambda r: "LanguageKind::" + r[7].capitalize(),
    )
    out += table(
        "extensions",
        "File extensions, lowercase, without the dot (only those no earlier language claims).",
        "&'static [&'static str]",
        lambda r: "&[" + ", ".join(rust_str(e) for e in r[5]) + "]",
    )
    out += table(
        "file_names",
        "Exact file names (Makefile, Dockerfile, ...).",
        "&'static [&'static str]",
        lambda r: "&[" + ", ".join(rust_str(n) for n in r[6]) + "]",
    )
    out[-1:] = ["}", ""]
    out += [
        "/// Linguist's language type.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]",
        "pub enum LanguageKind {",
        "    /// A programming language.",
        "    Programming,",
        "    /// A markup language (HTML, Markdown).",
        "    Markup,",
        "    /// A data format (JSON, YAML).",
        "    Data,",
        "    /// Prose (plain text formats).",
        "    Prose,",
        "}",
    ]
    print("\n".join(out))


if __name__ == "__main__":
    main(sys.argv[1])
