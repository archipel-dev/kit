"""Inputs: which data files of intellij-community make up the shipped themes and schemes."""

THEMES_DIR = "platform/platform-resources/src/themes"
EXTENSIONS = [
    "platform/platform-resources/src/META-INF/PlatformExtensions.xml",
    "plugins/laf/macos/resources/META-INF/plugin.xml",
    "plugins/laf/win10/resources/META-INF/plugin.xml",
]

# Theme JSON files that are not registered by a `themeProvider` in the extension files at
# the pinned tag (the classic High Contrast file is data only).
EXTRA_THEME_FILES = {"HighContrastClassic": f"{THEMES_DIR}/HighContrast.theme.json"}
# `themeProvider` ids whose `path` is relative to the plugin's `resources/` directory.
PLUGIN_THEME_ROOTS = {
    "macOSLight": "plugins/laf/macos/resources",
    "win10Light": "plugins/laf/win10/resources",
}

# Shipped UI themes. `id` is the kit id (kebab case, also the Rust module name); `provider`
# the IntelliJ theme id (what `parentTheme` refers to); `ctor` the `Theme` constructor.
# `scheme` is the kit id of the editor scheme the theme selects (`editorScheme` in its JSON,
# or the platform default scheme of its brightness when it names none). `look` is the
# `UiLook` the theme belongs to (`targetUi`: classic themes, the New UI `expUI` themes, the
# Islands themes); it is a suggestion for pickers, the application decides the look.
UI_THEMES = [
    dict(id="islands_dark", provider="Islands Dark", ctor="islands_dark", name="Islands Dark", look="Islands"),
    dict(id="islands_light", provider="Islands Light", ctor="islands_light", name="Islands Light", look="Islands"),
    dict(id="islands_darcula", provider="Islands Darcula", ctor="islands_darcula", name="Islands Darcula", look="Islands"),
    dict(id="high_contrast", provider="JetBrainsHighContrastTheme", ctor="high_contrast", name="High Contrast", look="Islands"),
    dict(id="new_ui_dark", provider="ExperimentalDark", ctor="new_ui_dark", name="New UI Dark", look="NewUi"),
    dict(id="new_ui_light", provider="ExperimentalLight", ctor="new_ui_light", name="New UI Light", look="NewUi"),
    dict(id="light_with_light_header", provider="ExperimentalLightWithLightHeader", ctor="light_with_light_header", name="Light with Light Header", look="NewUi"),
    dict(id="darcula", provider="Darcula", ctor="darcula", name="Darcula", look="Classic"),
    dict(id="intellij_light", provider="JetBrainsLightTheme", ctor="intellij_light", name="IntelliJ Light", look="Classic"),
    dict(id="classic_high_contrast", provider="HighContrastClassic", ctor="classic_high_contrast", name="High Contrast (Classic)", scheme="classic_high_contrast", look="Classic"),
    dict(id="macos_light", provider="macOSLight", ctor="macos_light", name="macOS Light", look="Classic"),
    dict(id="windows10_light", provider="win10Light", ctor="windows10_light", name="Windows 10 Light", look="Classic"),
]

# Editor color schemes: kit id, IntelliJ scheme name (what `parent_scheme` and
# `editorScheme` refer to), display name, data file (None: inside DefaultColorSchemesManager).
PLUGIN_SCHEMES = "plugins/color-schemes"
SCHEMES = [
    dict(id="default", key="Default", name="Default", file=None),
    dict(id="darcula", key="Darcula", name="Darcula", file=None),
    dict(id="islands_dark", key="Islands Dark", name="Islands Dark", file=f"{THEMES_DIR}/islands/IslandSchemeDark.xml"),
    dict(id="new_ui_dark", key="Dark", name="New UI Dark", file=f"{THEMES_DIR}/expUI/expUI_darkScheme.xml"),
    dict(id="new_ui_light", key="Light", name="New UI Light", file=f"{THEMES_DIR}/expUI/expUI_lightScheme.xml"),
    dict(id="darcula_contrast", key="Darcula Contrast", name="Darcula Contrast", file=f"{THEMES_DIR}/expUI/expUI_darculaContrastScheme.xml"),
    dict(id="intellij_light", key="IntelliJ Light", name="IntelliJ Light", file=f"{THEMES_DIR}/Light.xml"),
    dict(id="high_contrast", key="High contrast", name="High Contrast", file=f"{THEMES_DIR}/islands/highContrastScheme.xml"),
    dict(id="classic_high_contrast", key="High contrast (classic)", name="High Contrast (Classic)", file=f"{THEMES_DIR}/highContrastScheme.xml"),
    dict(id="monokai", key="Monokai", name="Monokai", file=f"{PLUGIN_SCHEMES}/monokai-color-scheme/resources/colorSchemes/monokai.xml"),
    dict(id="twilight", key="Twilight", name="Twilight", file=f"{PLUGIN_SCHEMES}/twilight-color-scheme/resources/colorSchemes/twilight.xml"),
    dict(id="blackboard", key="Blackboard", name="Blackboard", file=f"{PLUGIN_SCHEMES}/blackboard-color-scheme/resources/colorSchemes/blackboard.xml"),
    dict(id="cobalt", key="Cobalt", name="Cobalt", file=f"{PLUGIN_SCHEMES}/cobalt-color-scheme/resources/colorSchemes/cobalt.xml"),
    dict(id="all_hallows_eve", key="All hallow's eve", name="All Hallows Eve", file=f"{PLUGIN_SCHEMES}/all-hallows-eve-color-scheme/resources/colorSchemes/all_hallows_eve.xml"),
    dict(id="warm_neon", key="WarmNeon", name="Warm Neon", file=f"{PLUGIN_SCHEMES}/warm-neon-color-scheme/resources/colorSchemes/WarmNeon.xml"),
    dict(id="github", key="Github", name="GitHub", file=f"{PLUGIN_SCHEMES}/github-color-scheme/resources/colorSchemes/github.xml"),
    dict(id="vibrant_ink", key="VibrantInk", name="Vibrant Ink", file=f"{PLUGIN_SCHEMES}/vibrant-ink-color-scheme/resources/colorSchemes/vibrant_ink.xml"),
    dict(id="rails_casts", key="Railscasts", name="Rails Casts", file=f"{PLUGIN_SCHEMES}/rails-casts-color-scheme/resources/colorSchemes/rails_casts.xml"),
]
