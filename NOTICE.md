# Notices

## Fonts

`crates/archipel-kit/assets/fonts/` bundles:

- **Inter** (Regular, SemiBold), Copyright the Inter Project Authors, licensed under the SIL
  Open Font License 1.1 (`Inter-OFL.txt`).
- **JetBrains Mono** (Regular, Bold), Copyright the JetBrains Mono Project Authors, licensed
  under the SIL Open Font License 1.1 (`JetBrainsMono-OFL.txt`).

## Theme colors

The color values of `Theme::dark()` and `Theme::light()` are facts extracted from the theme
and color scheme data files of **IntelliJ Community**
(https://github.com/JetBrains/intellij-community, tag `idea/262.10968.63`), licensed under
the Apache License 2.0. Upstream notice: "This software includes code from IntelliJ IDEA.
Copyright (C) JetBrains s.r.o."

Only color values were taken. No source code, icons, images or other artwork were copied;
the icons in `icons.rs` are original drawings. IntelliJ and JetBrains are trademarks of
JetBrains s.r.o.; Archipel is not affiliated with or endorsed by JetBrains.

## Language colors and file associations

`crates/archipel-icons/src/languages.rs` is generated from the `languages.yml` data file of
**GitHub Linguist** (https://github.com/github-linguist/linguist), Copyright (c) 2017 GitHub,
Inc., licensed under the MIT License: language names, colors, extensions and file names.
The language icons are original designs (tiles with letters); no logos are used.

## Nerd Font codepoints

`crates/archipel-kit-tui/src/glyphs.rs` maps kit glyphs, languages and symbols to codepoints
listed in `glyphnames.json` of **Nerd Fonts** (https://github.com/ryanoasis/nerd-fonts,
v3.5.1), Copyright (c) 2014 Ryan L McIntyre, licensed under the MIT License. Only codepoints
and names were taken. No font or glyph drawings are included: the terminal's installed Nerd
Font draws them, and the docs images draw Archipel's own icons in their place.
