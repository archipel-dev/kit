# Changelog

## [0.3.2](https://github.com/archipel-dev/kit/compare/v0.3.1...v0.3.2) (2026-10-12)


### Bug Fixes

* preserve typed drops and clip table header targets ([#43](https://github.com/archipel-dev/kit/issues/43)) ([d48b2a0](https://github.com/archipel-dev/kit/commit/d48b2a02f9ec0b318a1545333af2d0376e56ff03))

## [0.3.1](https://github.com/archipel-dev/kit/compare/v0.3.0...v0.3.1) (2026-10-12)


### Bug Fixes

* **window:** the macOS green button zooms instead of entering full screen ([#41](https://github.com/archipel-dev/kit/issues/41)) ([3a45b61](https://github.com/archipel-dev/kit/commit/3a45b610c48bfd092eda285a18e0613e2bc13d84))

## [0.3.0](https://github.com/archipel-dev/kit/compare/v0.2.2...v0.3.0) (2026-10-12)


### ⚠ BREAKING CHANGES

* **tab-bar:** `Colors` gains `tab_scroll_thumb` and `tab_scroll_thumb_hover`; `TabBarResponse` is `#[non_exhaustive]` with two button responses and no longer `Copy`/`Eq`; the bar adds the 6 pt inset itself and fills the available width, so callers drop their own leading spacer.

### Features

* **tab-bar:** Android Studio's strip inset, overflow scrolling and strip buttons ([#39](https://github.com/archipel-dev/kit/issues/39)) ([9be68de](https://github.com/archipel-dev/kit/commit/9be68de72cefe14d32d0dd5fead54d52a08917bf))

## [0.2.2](https://github.com/archipel-dev/kit/compare/v0.2.1...v0.2.2) (2026-10-12)


### Features

* **workbench:** let an embedded workbench keep the host's frame ([#37](https://github.com/archipel-dev/kit/issues/37)) ([e110403](https://github.com/archipel-dev/kit/commit/e110403c2013568ffc128185823467f22f3902c7))

## [0.2.1](https://github.com/archipel-dev/kit/compare/v0.2.0...v0.2.1) (2026-10-12)


### Features

* hairline under every island header ([#35](https://github.com/archipel-dev/kit/issues/35)) ([7969287](https://github.com/archipel-dev/kit/commit/7969287b067112345cf6cea1a66308db40f03d12))

## [0.2.0](https://github.com/archipel-dev/kit/compare/v0.1.0...v0.2.0) (2026-10-12)


### ⚠ BREAKING CHANGES

* Colors gains tab_text, selection_inactive, control, control_selected, border_disabled, group_separator and the clear-button colors; focus_ring, the dark control_border, ICON_TEXT_GAP, HEADER_HEIGHT and TREE_INDENT change; rows lose their hover fill and buttons their hover and press shading; Breadcrumbs::show returns BreadcrumbsResponse; SectionHeader has a lifetime parameter.
* `DialogAction` is `#[non_exhaustive]` and gained `Tertiary`; exhaustive matches need a wildcard arm.
* `code::TokenKind` is `#[non_exhaustive]` and gained `Resolved`, `Unresolved` and `Secret`; exhaustive matches need a wildcard arm.
* **list-row:** `RowContent` gained fields; struct-literal callers must use `..RowContent::new(...)`.
* `Orientation` and the new `Side` are `#[non_exhaustive]`; exhaustive matches on `Orientation` need a wildcard arm.
* `MenuItem` has a lifetime parameter (`MenuItem<'a>`), and `ContextMenu` closes on a click outside it only; widgets other than `MenuItem` must call `ui.close()` to close the menu.
* parts building blocks, UX audit and conformance tests

### Features

* add collapse to SplitPane ([#23](https://github.com/archipel-dev/kit/issues/23)) ([b934b02](https://github.com/archipel-dev/kit/commit/b934b022ead6c01940229062a850d94d705a5796)), closes [#17](https://github.com/archipel-dev/kit/issues/17)
* add KeyValueTable component ([#10](https://github.com/archipel-dev/kit/issues/10)) ([47c5582](https://github.com/archipel-dev/kit/commit/47c5582209d2008889c3382529d276dbc584fd44))
* add SplitPane component ([#6](https://github.com/archipel-dev/kit/issues/6)) ([a3ff615](https://github.com/archipel-dev/kit/commit/a3ff615136575febe5e0f384911be8a3519d8b69))
* Android Studio Islands look and the components the apps need ([#32](https://github.com/archipel-dev/kit/issues/32)) ([e692ccd](https://github.com/archipel-dev/kit/commit/e692ccdc64c69b916a5b6a7973d87c62bd675884))
* archipel-kit v0.1 components, icon kit, testing helpers, docs and gallery ([de5707e](https://github.com/archipel-dev/kit/commit/de5707e4728d82709ef5ed0817117aee6ebd6eb8))
* archipel-kit v0.1 components, testing helpers, docs and gallery ([08401c7](https://github.com/archipel-dev/kit/commit/08401c79099b10613b1dda3a94ac948f91826e17))
* **chart:** add Histogram and Sparkline ([11db25b](https://github.com/archipel-dev/kit/commit/11db25ba25baa17f70a9c3d2896762c493a92d0c))
* **chart:** add Histogram and Sparkline ([df24ad5](https://github.com/archipel-dev/kit/commit/df24ad573543d7df14a70277fb20757aa1183e0e))
* CodeView and CodeEditor with pluggable highlighting ([#8](https://github.com/archipel-dev/kit/issues/8)) ([49472af](https://github.com/archipel-dev/kit/commit/49472af3b8443c8b9090ed9716c5edbb9102f114))
* **collections:** add TreeTable with inline cell edit ([#4](https://github.com/archipel-dev/kit/issues/4)) ([dc27d56](https://github.com/archipel-dev/kit/commit/dc27d56b79bc9b26f1a54fe9947d7a2743c81ff6))
* **collections:** color table cells by value kind ([#7](https://github.com/archipel-dev/kit/issues/7)) ([7b46940](https://github.com/archipel-dev/kit/commit/7b469404db185d3a3b759b1232fc566f2e4b9488))
* **collections:** reorder Table columns by dragging headers and edit cells inline ([#5](https://github.com/archipel-dev/kit/issues/5)) ([a8b6a7a](https://github.com/archipel-dev/kit/commit/a8b6a7ae9b9a2acdb315d929329d47dfa5e14183))
* **gallery:** demonstrate picker slots and new field and tree states ([eea9d17](https://github.com/archipel-dev/kit/commit/eea9d176fc05b410f728ea3b4886f16e957d2991))
* highlighted single-line TextField with {{variable}} tokens and caret API ([#18](https://github.com/archipel-dev/kit/issues/18)) ([bebbe8f](https://github.com/archipel-dev/kit/commit/bebbe8f509b3cd60457fa86d4e729b88ccfb18db))
* **icons:** archipel-icons kit with 113 designed glyphs, design lints and review ([7b6a00f](https://github.com/archipel-dev/kit/commit/7b6a00f62aa4141ddbff271816fd47b235aa0d29))
* **icons:** center every glyph and add 78 language icons ([4e7f01c](https://github.com/archipel-dev/kit/commit/4e7f01cfa40f5063c4ac4f2bbd86e43dd42e4f47))
* **icons:** code symbol icons for types, members and modifiers ([6b2b3fe](https://github.com/archipel-dev/kit/commit/6b2b3feecbcc9e82b3036c1adb1d25dfdf96693d))
* **icons:** redesign language icons as file outline with color band ([6e050d5](https://github.com/archipel-dev/kit/commit/6e050d5225c0300f5f3b68fcb8719e20df3c5b84))
* **icons:** selectable language icon styles; gallery without panel lines ([6578911](https://github.com/archipel-dev/kit/commit/65789113b19e380fb18d0c3cfe50a746dcaedf24))
* **kit:** align components with the Islands design values ([91b307e](https://github.com/archipel-dev/kit/commit/91b307ec3c7b2f6f3636c729b9651e8f4fe0f563))
* **list-row:** row options, draggable-row Tab order fix, Tree per-row behaviours ([#9](https://github.com/archipel-dev/kit/issues/9)) ([3e6469f](https://github.com/archipel-dev/kit/commit/3e6469f91cb7e9920ff9b1ceeed565bac0f31c95))
* MenuItem submenus, TextField::id, Surface::Chrome, Panel::on_hide, StatusBarItem ([#11](https://github.com/archipel-dev/kit/issues/11)) ([31bd3c2](https://github.com/archipel-dev/kit/commit/31bd3c2f8cf27bb701b78d17619f046cf66fc567))
* parts building blocks, UX audit and conformance tests ([536c93e](https://github.com/archipel-dev/kit/commit/536c93e47b62f8987d947efc77368f4252110ac7))
* **segmented:** add named icons and arrow navigation ([cca780a](https://github.com/archipel-dev/kit/commit/cca780aa771919fb1ef789fb9afc593dc42b7b80))
* slots, behaviours with .with(), action cursor and a building-components guide ([164832d](https://github.com/archipel-dev/kit/commit/164832d321c285f431c180127d922f4e6e0a3bef))
* **text-field:** add named multiline inputs with bounded scrolling ([cd8079d](https://github.com/archipel-dev/kit/commit/cd8079d3b42347e664b32ec5d32b24f3a792257e))
* tooltips, dialogs, menu separators and nested cursor rules ([5578f12](https://github.com/archipel-dev/kit/commit/5578f12dfbda58bb07b96b74fbb2d917ec373d04))
* **tree:** add loading indicators and themed icon tones ([491dfd1](https://github.com/archipel-dev/kit/commit/491dfd141bc17bd5467c3e484cd15422be1c2d8b))
* **tui:** add archipel-kit-tui, the kit for ratatui ([65ecbe2](https://github.com/archipel-dev/kit/commit/65ecbe2fcdec7af8326ea085bfd61f0be3a01a82))
* **tui:** flat NvChad-style components ([0f338d1](https://github.com/archipel-dev/kit/commit/0f338d1f7479580058444c3dc14cee691beaa96e))
* value tones, chart palette and HTTP method/status tones ([#3](https://github.com/archipel-dev/kit/issues/3)) ([3c5dbf5](https://github.com/archipel-dev/kit/commit/3c5dbf5643a00071f77e9c9b43e8442d6d0f6838))
* virtualized VirtualList, Tree and Table ([2b36d4a](https://github.com/archipel-dev/kit/commit/2b36d4a41ed63733c9ac18a94d9c740b9384f319))


### Bug Fixes

* align desktop input, picker and theme behavior ([6461237](https://github.com/archipel-dev/kit/commit/646123765ad3f47bcfe1280480b5f5d8d0457ffb))
* audit reachability of draggable rows, dialog Enter, dropdown Escape, dialog height, disabled primary contrast ([#19](https://github.com/archipel-dev/kit/issues/19)) ([976b946](https://github.com/archipel-dev/kit/commit/976b946573e52aab251a833fdf9d23172e565ea6)), closes [#21](https://github.com/archipel-dev/kit/issues/21)
* **chart:** address review: selection pill, tokens, tooltip rule, docs ([699bdba](https://github.com/archipel-dev/kit/commit/699bdba0845182b17546d178c8704eafd6681812))
* **chart:** prevent count overflow and keep hover tooltips stable ([49ac3c1](https://github.com/archipel-dev/kit/commit/49ac3c171737c572a6d87f707708aefdc81bdd89))
* **chart:** prevent count overflow and keep hover tooltips stable ([ee1b5cb](https://github.com/archipel-dev/kit/commit/ee1b5cbf0e75e014f6bd726a3aeb74f83ed4ac26))
* **dropdown:** support keyboard picking and shared leading slots ([4456152](https://github.com/archipel-dev/kit/commit/4456152b35f4c8ac4a915a828385e6ad88647073))
* **gallery:** enable native graphics backends outside test builds ([8f9d751](https://github.com/archipel-dev/kit/commit/8f9d751518d63073a981bfed15a5502a4c38e186))
* **theme:** keep native appearance and kit visuals in sync ([aabede4](https://github.com/archipel-dev/kit/commit/aabede4958573ae9e9015d8f7005421ea99210de))
