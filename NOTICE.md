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

The bundled themes and editor color schemes (`Theme::islands_dark()` and the other
constructors, `EditorScheme::all()`, `crates/archipel-kit/src/theme/generated/`) are generated
by `tools/themes` from these data files of the same checkout (tag `idea/262.10968.63`, commit
`8062c061ff33e22041d9f1844db5b8b8a6b5e4d7`), also licensed under the Apache License 2.0:

- UI themes, in `platform/platform-resources/src/themes/`: `darcula.theme.json`,
  `intellijlaf.theme.json`, `Light.theme.json`, `HighContrast.theme.json`,
  `expUI/expUI_dark.theme.json`, `expUI/expUI_light.theme.json`,
  `expUI/expUI_light_with_light_header.theme.json`, `islands/ManyIslandsDark.theme.json`,
  `islands/ManyIslandsLight.theme.json`, `islands/ManyIslandsDarcula.theme.json` and
  `islands/HighContrast.theme.json`; and in `plugins/laf/`:
  `macos/resources/macintellijlaf.theme.json` and `win10/resources/win10intellijlaf.theme.json`.
- Editor color schemes: `platform/platform-resources/src/DefaultColorSchemesManager.xml`
  (Default, Darcula), and in `platform/platform-resources/src/themes/`: `Light.xml`,
  `highContrastScheme.xml`, `expUI/expUI_darkScheme.xml`, `expUI/expUI_lightScheme.xml`,
  `expUI/expUI_darculaContrastScheme.xml`, `islands/IslandSchemeDark.xml` and
  `islands/highContrastScheme.xml`; and the bundled scheme plugins in
  `plugins/color-schemes/` (`monokai`, `twilight`, `blackboard`, `cobalt`,
  `all-hallows-eve`, `warm-neon`, `github`, `vibrant-ink` and `rails-casts`, each
  `resources/colorSchemes/*.xml`). These plugins are part of the same repository, name
  JetBrains as their vendor and carry no license file of their own; the scheme names refer to
  well-known editor themes, and only the color values of JetBrains' files are used.
- Registration files read for theme ids and paths: `platform/platform-resources/src/META-INF/PlatformExtensions.xml`
  and the `META-INF/plugin.xml` of the macOS and Windows 10 laf plugins.

Only color values (and, for the UI themes, numeric metrics such as arc radii and row heights)
were taken. No source code, icons, images or other artwork were copied; the icons in `icons.rs`
are original drawings. IntelliJ and JetBrains are trademarks of JetBrains s.r.o.; Archipel is
not affiliated with or endorsed by JetBrains.

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
