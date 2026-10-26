//! Nerd Font characters for the icon kit: glyphs, language logos, symbol icons and control
//! shapes. Codepoints from Nerd Fonts `glyphnames.json` (v3.5.1, MIT); only codepoints are
//! referenced, the drawings come from the font installed in the terminal.

use archipel_icons::{Glyph, Language, SymbolKind};

/// The Nerd Font character for `glyph` (codicons, one Material Design icon).
pub const fn nerd_font(glyph: Glyph) -> char {
    match glyph {
        Glyph::ChevronDown => '\u{eab4}',  // cod-chevron_down
        Glyph::ChevronUp => '\u{eab7}',    // cod-chevron_up
        Glyph::ChevronLeft => '\u{eab5}',  // cod-chevron_left
        Glyph::ChevronRight => '\u{eab6}', // cod-chevron_right
        Glyph::ArrowLeft => '\u{ea9b}',    // cod-arrow_left
        Glyph::ArrowRight => '\u{ea9c}',   // cod-arrow_right
        Glyph::ArrowUp => '\u{eaa1}',      // cod-arrow_up
        Glyph::ArrowDown => '\u{ea9a}',    // cod-arrow_down
        Glyph::ExternalLink => '\u{eb14}', // cod-link_external
        Glyph::Menu => '\u{eb94}',         // cod-menu
        Glyph::More => '\u{ea7c}',         // cod-ellipsis
        Glyph::MoreVertical => '\u{eb10}', // cod-kebab_vertical
        Glyph::Home => '\u{eb06}',         // cod-home
        Glyph::ExpandAll => '\u{eb95}',    // cod-expand_all
        Glyph::CollapseAll => '\u{eac5}',  // cod-collapse_all
        Glyph::Fullscreen => '\u{eb4c}',   // cod-screen_full
        Glyph::Minimize => '\u{eaba}',     // cod-chrome_minimize
        Glyph::Close => '\u{ea76}',        // cod-close
        Glyph::SidebarLeft => '\u{ebf3}',  // cod-layout_sidebar_left
        Glyph::SidebarRight => '\u{ebf4}', // cod-layout_sidebar_right
        Glyph::PanelBottom => '\u{ebf2}',  // cod-layout_panel
        Glyph::SplitColumns => '\u{eb56}', // cod-split_horizontal
        Glyph::SplitRows => '\u{eb57}',    // cod-split_vertical
        Glyph::Plus => '\u{ea60}',         // cod-add
        Glyph::Minus => '\u{eb3b}',        // cod-remove
        Glyph::AddBox => '\u{eadc}',       // cod-diff_added
        Glyph::Edit => '\u{ea73}',         // cod-edit
        Glyph::Delete => '\u{ea81}',       // cod-trash
        Glyph::Copy => '\u{ebcc}',         // cod-copy
        Glyph::Paste => '\u{eac0}',        // cod-clippy
        Glyph::Save => '\u{eb4b}',         // cod-save
        Glyph::Undo => '\u{eae2}',         // cod-discard
        Glyph::Redo => '\u{ebb0}',         // cod-redo
        Glyph::Refresh => '\u{eb37}',      // cod-refresh
        Glyph::Search => '\u{ea6d}',       // cod-search
        Glyph::FindInFiles => '\u{ec0d}',  // cod-search_fuzzy
        Glyph::Filter => '\u{eaf1}',       // cod-filter
        Glyph::Sort => '\u{eb16}',         // cod-list_ordered
        Glyph::Settings => '\u{eaf8}',     // cod-gear
        Glyph::Pin => '\u{eb2b}',          // cod-pin
        Glyph::Star => '\u{ea6a}',         // cod-star_empty
        Glyph::Lock => '\u{ea75}',         // cod-lock
        Glyph::Unlock => '\u{eb74}',       // cod-unlock
        Glyph::Eye => '\u{ea70}',          // cod-eye
        Glyph::EyeOff => '\u{eae7}',       // cod-eye_closed
        Glyph::Download => '\u{eac2}',     // cod-cloud_download
        Glyph::Upload => '\u{eac3}',       // cod-cloud_upload
        Glyph::Link => '\u{eb15}',         // cod-link
        Glyph::Send => '\u{ec0f}',         // cod-send
        Glyph::File => '\u{ea7b}',         // cod-file
        Glyph::FileText => '\u{ec5e}',     // cod-file_text
        Glyph::FileCode => '\u{eae9}',     // cod-file_code
        Glyph::NewFile => '\u{ea7f}',      // cod-new_file
        Glyph::Folder => '\u{ea83}',       // cod-folder
        Glyph::FolderOpen => '\u{eaf7}',   // cod-folder_opened
        Glyph::NewFolder => '\u{ea80}',    // cod-new_folder
        Glyph::Project => '\u{eb46}',      // cod-root_folder
        Glyph::Image => '\u{eaea}',        // cod-file_media
        Glyph::Archive => '\u{ea98}',      // cod-archive
        Glyph::Run => '\u{eb2c}',          // cod-play
        Glyph::Debug => '\u{eb91}',        // cod-debug_alt
        Glyph::Stop => '\u{ead7}',         // cod-debug_stop
        Glyph::Pause => '\u{ead1}',        // cod-debug_pause
        Glyph::Rerun => '\u{ead2}',        // cod-debug_restart
        Glyph::StepOver => '\u{ead6}',     // cod-debug_step_over
        Glyph::StepInto => '\u{ead4}',     // cod-debug_step_into
        Glyph::StepOut => '\u{ead5}',      // cod-debug_step_out
        Glyph::Breakpoint => '\u{ea71}',   // cod-circle_filled
        Glyph::Build => '\u{eb6d}',        // cod-tools
        Glyph::Terminal => '\u{ea85}',     // cod-terminal
        Glyph::Code => '\u{eac4}',         // cod-code
        Glyph::Test => '\u{ea79}',         // cod-beaker
        Glyph::Package => '\u{eb29}',      // cod-package
        Glyph::Structure => '\u{ea91}',    // cod-symbol_structure
        Glyph::Problems => '\u{ea87}',     // cod-error
        Glyph::Layers => '\u{ebd2}',       // cod-layers
        Glyph::Log => '\u{eb9d}',          // cod-output
        Glyph::ScrollEnd => '\u{eaf3}',    // cod-fold_down
        Glyph::SoftWrap => '\u{eb80}',     // cod-word_wrap
        Glyph::Device => '\u{eadb}',       // cod-device_mobile
        Glyph::Sparkle => '\u{ec10}',      // cod-sparkle
        Glyph::Plugin => '\u{eae6}',       // cod-extensions
        Glyph::Branch => '\u{ec6f}',       // cod-git_branch
        Glyph::Commit => '\u{eafc}',       // cod-git_commit
        Glyph::Merge => '\u{eafe}',        // cod-git_merge
        Glyph::PullRequest => '\u{ea64}',  // cod-git_pull_request
        Glyph::Diff => '\u{eae1}',         // cod-diff
        Glyph::History => '\u{ea82}',      // cod-history
        Glyph::Tag => '\u{ea66}',          // cod-tag
        Glyph::Agent => '\u{ec10}',        // cod-sparkle (no robot in the font; same as Sparkle)
        Glyph::Agents => '\u{ebd2}',       // cod-layers
        Glyph::Task => '\u{eab3}',         // cod-checklist
        Glyph::Worktree => '\u{ec6f}',     // cod-git_branch
        Glyph::FanOut => '\u{eafe}',       // cod-git_merge
        Glyph::Spawn => '\u{ec6f}',        // cod-git_branch
        Glyph::Unread => '\u{ea71}',       // cod-circle_filled
        Glyph::Thinking => '\u{ea61}',     // cod-lightbulb
        Glyph::Tool => '\u{eb6d}',         // cod-tools
        Glyph::Summary => '\u{eb9d}',      // cod-output
        Glyph::Tokens => '\u{eace}',       // cod-database
        Glyph::Error => '\u{ea87}',        // cod-error
        Glyph::Warning => '\u{ea6c}',      // cod-warning
        Glyph::WeakWarning => '\u{ea6c}',  // cod-warning
        Glyph::Info => '\u{ea74}',         // cod-info
        Glyph::Success => '\u{eab2}',      // cod-check
        Glyph::Check => '\u{eab2}',        // cod-check
        Glyph::Help => '\u{eb32}',         // cod-question
        Glyph::Bell => '\u{eaa2}',         // cod-bell
        Glyph::Clock => '\u{eb7c}',        // cod-watch
        Glyph::Deny => '\u{ea87}',         // cod-error
        Glyph::Database => '\u{eace}',     // cod-database
        Glyph::Table => '\u{ebb7}',        // cod-table
        Glyph::Json => '\u{eb0f}',         // cod-json
        Glyph::Key => '\u{eb11}',          // cod-key
        Glyph::Server => '\u{eb50}',       // cod-server
        Glyph::Cloud => '\u{ebaa}',        // cod-cloud
        Glyph::Globe => '\u{eb01}',        // cod-globe
        Glyph::Plug => '\u{eb2d}',         // cod-plug
        Glyph::Shield => '\u{eb53}',       // cod-shield
        Glyph::ShieldCheck => '\u{eb53}',  // cod-shield
        Glyph::User => '\u{eb99}',         // cod-account
        Glyph::Lightning => '\u{f140c}',   // md-lightning_bolt_outline
        Glyph::Bookmark => '\u{eaa5}',     // cod-bookmark
        Glyph::Comment => '\u{ea6b}',      // cod-comment
        Glyph::Mention => '\u{ea6b}',      // cod-comment
        Glyph::Hash => '\u{ea6b}',         // cod-comment
        Glyph::Sun => '\u{eac6}',          // cod-color_mode
        Glyph::Moon => '\u{eac6}',         // cod-color_mode
    }
}

/// The Nerd Font logo for `language` (seti, devicons, custom or Material Design icons),
/// or a plain file for languages without one.
pub const fn language(language: Language) -> char {
    match language {
        Language::Rust => '\u{e68b}',             // seti-rust
        Language::Java => '\u{e66d}',             // seti-java
        Language::Kotlin => '\u{e634}',           // seti-kotlin
        Language::Scala => '\u{e68e}',            // seti-scala
        Language::Groovy => '\u{e775}',           // dev-groovy
        Language::Gradle => '\u{e660}',           // seti-gradle
        Language::GradleKotlin => '\u{e660}',     // seti-gradle
        Language::Clojure => '\u{e642}',          // seti-clojure
        Language::Python => '\u{e606}',           // seti-python
        Language::Ruby => '\u{e605}',             // seti-ruby
        Language::Php => '\u{e608}',              // seti-php
        Language::Perl => '\u{e67e}',             // seti-perl
        Language::Lua => '\u{e620}',              // seti-lua
        Language::JavaScript => '\u{e60c}',       // seti-javascript
        Language::TypeScript => '\u{e628}',       // seti-typescript
        Language::Tsx => '\u{e625}',              // seti-react
        Language::Go => '\u{e627}',               // seti-go
        Language::C => '\u{e649}',                // seti-c
        Language::Cpp => '\u{e7a3}',              // dev-cplusplus
        Language::CSharp => '\u{e648}',           // seti-c_sharp
        Language::FSharp => '\u{e65a}',           // seti-f_sharp
        Language::ObjectiveC => '\u{e84d}',       // dev-objectivec
        Language::Swift => '\u{e699}',            // seti-swift
        Language::Dart => '\u{e64c}',             // seti-dart
        Language::Zig => '\u{e6a9}',              // seti-zig
        Language::Nim => '\u{e677}',              // seti-nim
        Language::Crystal => '\u{e62f}',          // seti-crystal
        Language::D => '\u{e651}',                // seti-d
        Language::Elixir => '\u{e62d}',           // seti-elixir
        Language::Erlang => '\u{e7b1}',           // dev-erlang
        Language::Haskell => '\u{e61f}',          // seti-haskell
        Language::OCaml => '\u{e67a}',            // seti-ocaml
        Language::Elm => '\u{e62c}',              // seti-elm
        Language::Julia => '\u{e624}',            // seti-julia
        Language::R => '\u{e68a}',                // seti-r
        Language::Matlab => '\u{e82a}',           // dev-matlab
        Language::Fortran => '\u{e7de}',          // dev-fortran
        Language::Pascal => '\u{ea7b}',           // cod-file
        Language::Ada => '\u{e6b5}',              // custom-ada
        Language::Prolog => '\u{e685}',           // seti-prolog
        Language::Racket => '\u{e93a}',           // dev-racket
        Language::Scheme => '\u{e6b1}',           // custom-scheme
        Language::CommonLisp => '\u{e6b0}',       // custom-common_lisp
        Language::Assembly => '\u{e637}',         // seti-asm
        Language::WebAssembly => '\u{e6a1}',      // seti-wasm
        Language::Solidity => '\u{e8a6}',         // dev-solidity
        Language::VimScript => '\u{e7c5}',        // dev-vim
        Language::Shell => '\u{e691}',            // seti-shell
        Language::PowerShell => '\u{e683}',       // seti-powershell
        Language::Batchfile => '\u{e70f}',        // dev-windows
        Language::Sql => '\u{e64d}',              // seti-db
        Language::GraphQl => '\u{e662}',          // seti-graphql
        Language::Protobuf => '\u{ea7b}',         // cod-file
        Language::Hcl => '\u{e69a}',              // seti-terraform
        Language::Nix => '\u{f1105}',             // md-nix
        Language::Dockerfile => '\u{e650}',       // seti-docker
        Language::Makefile => '\u{e673}',         // seti-makefile
        Language::CMake => '\u{e794}',            // dev-cmake
        Language::Starlark => '\u{e63a}',         // seti-bazel
        Language::Html => '\u{e60e}',             // seti-html
        Language::Css => '\u{e614}',              // seti-css
        Language::Scss => '\u{e603}',             // seti-sass
        Language::Less => '\u{e60b}',             // seti-less
        Language::Vue => '\u{e6a0}',              // seti-vue
        Language::Svelte => '\u{e697}',           // seti-svelte
        Language::Xml => '\u{e619}',              // seti-xml
        Language::Markdown => '\u{e609}',         // seti-markdown
        Language::ReStructuredText => '\u{ea7b}', // cod-file
        Language::Tex => '\u{e69b}',              // seti-tex
        Language::AsciiDoc => '\u{ea7b}',         // cod-file
        Language::Json => '\u{e60b}',             // seti-json
        Language::Yaml => '\u{e8eb}',             // dev-yaml
        Language::Toml => '\u{e6b2}',             // custom-toml
        Language::Ini => '\u{e615}',              // seti-config
        Language::Csv => '\u{e64a}',              // seti-csv
        Language::Properties => '\u{e615}',       // seti-config
        Language::IgnoreList => '\u{e65d}',       // seti-git
        Language::EditorConfig => '\u{e652}',     // seti-editorconfig
    }
}

/// The codicon for symbols of `kind`.
pub const fn symbol(kind: SymbolKind) -> char {
    match kind {
        SymbolKind::Class => '\u{eb5b}',         // cod-symbol_class
        SymbolKind::Interface => '\u{eb61}',     // cod-symbol_interface
        SymbolKind::Enum => '\u{ea95}',          // cod-symbol_enum
        SymbolKind::Record => '\u{ea91}',        // cod-symbol_structure
        SymbolKind::Annotation => '\u{eb63}',    // cod-symbol_misc
        SymbolKind::Struct => '\u{ea91}',        // cod-symbol_structure
        SymbolKind::Trait => '\u{eb61}',         // cod-symbol_interface
        SymbolKind::Object => '\u{ea8b}',        // cod-symbol_namespace
        SymbolKind::Method => '\u{ea8c}',        // cod-symbol_method
        SymbolKind::Function => '\u{ea8c}',      // cod-symbol_method
        SymbolKind::Constructor => '\u{ea8c}',   // cod-symbol_method
        SymbolKind::Field => '\u{eb5f}',         // cod-symbol_field
        SymbolKind::Property => '\u{eb65}',      // cod-symbol_property
        SymbolKind::Variable => '\u{ea88}',      // cod-symbol_variable
        SymbolKind::Constant => '\u{eb5d}',      // cod-symbol_constant
        SymbolKind::Parameter => '\u{ea92}',     // cod-symbol_parameter
        SymbolKind::EnumMember => '\u{eb5e}',    // cod-symbol_enum_member
        SymbolKind::TypeParameter => '\u{ea92}', // cod-symbol_parameter
        SymbolKind::Lambda => '\u{ec41}',        // cod-symbol_method_arrow
        SymbolKind::Package => '\u{ea8b}',       // cod-symbol_namespace
        SymbolKind::Module => '\u{ea8b}',        // cod-symbol_namespace
    }
}

/// Powerline half circles: the rounded ends of pills.
pub const CAP_LEFT: char = '\u{e0b6}'; // ple-left_half_circle_thick
/// See [`CAP_LEFT`].
pub const CAP_RIGHT: char = '\u{e0b4}'; // ple-right_half_circle_thick
/// Checkbox states.
pub const CHECKBOX_ON: char = '\u{f0132}'; // md-checkbox_marked
/// See [`CHECKBOX_ON`].
pub const CHECKBOX_OFF: char = '\u{f0131}'; // md-checkbox_blank_outline
/// See [`CHECKBOX_ON`].
pub const CHECKBOX_MIXED: char = '\u{f0375}'; // md-minus_box
/// Radio button states.
pub const RADIO_ON: char = '\u{f043e}'; // md-radiobox_marked
/// See [`RADIO_ON`].
pub const RADIO_OFF: char = '\u{f043d}'; // md-radiobox_blank
/// Switch states.
pub const TOGGLE_ON: char = '\u{f0521}'; // md-toggle_switch
/// See [`TOGGLE_ON`].
pub const TOGGLE_OFF: char = '\u{f0a19}'; // md-toggle_switch_off_outline
