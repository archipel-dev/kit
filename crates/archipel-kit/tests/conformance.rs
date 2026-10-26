//! Every interactive component passes the conformance battery, and the audit catches the
//! mistakes it is meant to catch.

mod common;

use archipel_kit::testing::{self, Problem, Queryable};
use archipel_kit::{Glyph, Theme};
use archipel_kit::{
    action_system::{ActionButton, ActionMenuItem},
    editor::{CodeEditor, CodeView},
    openapi::ui::ComboBox,
    ui::tabs::{JBTabs, TabInfo},
    ui::{
        ActionLink, BreadcrumbsComponent, Button, Cell, ColoredListCellRenderer, ColumnInfo,
        JBCheckBox, JBIntSpinner, JBList, JBRadioButton, JBTable, JBTextField, OnOffButton,
        SegmentedButton, TitledSeparator, Tree, TreeNode, TreeTable,
    },
    wm::{SquareStripeButton, StatusBarWidget, ToolbarComboButton, ToolbarSplitButton},
};

#[test]
fn jb_tabs_buttons_conform() {
    // More tabs than fit: the strip shows "Show hidden tabs" next to "More actions".
    let tabs: Vec<TabInfo> = (0..8)
        .map(|i| TabInfo::new(format!("file{i}.rs")).closable(true))
        .collect();
    for button in ["Show hidden tabs", "More actions"] {
        testing::conformance(button, |ui, enabled| {
            ui.add_enabled_ui(enabled, |ui| {
                let out = JBTabs::new(&tabs, 0)
                    .more_menu(|ui| {
                        ui.add(ActionMenuItem::new("Close All Tabs"));
                    })
                    .show(ui);
                if button == "More actions" {
                    out.more_button
                } else {
                    out.hidden_tabs_button
                }
                .expect("the strip shows the button")
            })
            .inner
        })
        .run();
    }
}

#[test]
fn buttons_conform() {
    for variant in ["primary", "secondary", "ghost", "danger"] {
        testing::conformance("Run", move |ui, enabled| {
            let button = match variant {
                "primary" => Button::primary("Run"),
                "secondary" => Button::secondary("Run"),
                "ghost" => Button::ghost("Run"),
                _ => Button::danger("Run"),
            };
            ui.add(button.icon(Glyph::Run).enabled(enabled))
        })
        .run();
    }
    testing::conformance("Small", |ui, enabled| {
        ui.add(Button::secondary("Small").small(true).enabled(enabled))
    })
    .run();
}

#[test]
fn action_button_conforms() {
    testing::conformance("Reload", |ui, enabled| {
        ui.add(ActionButton::new(Glyph::Refresh, "Reload").enabled(enabled))
    })
    .run();
}

#[test]
fn toolbar_controls_conform() {
    testing::conformance("Search everywhere", |ui, enabled| {
        ui.add(
            ActionButton::new(Glyph::Search, "Search everywhere")
                .large(true)
                .enabled(enabled),
        )
    })
    .run();
    testing::conformance("Version Control", |ui, enabled| {
        ToolbarComboButton::new("Version Control")
            .icon(Glyph::Branch)
            .dropdown(true)
            .enabled(enabled)
            .show(ui)
    })
    .run();
}

#[test]
fn toolbar_split_button_conforms() {
    testing::conformance("Run app", |ui, enabled| {
        ToolbarSplitButton::new("Run app")
            .icon(Glyph::Run)
            .enabled(enabled)
            .show(ui)
            .main
    })
    .run();
    testing::conformance("Run app options", |ui, enabled| {
        ToolbarSplitButton::new("Run app")
            .icon(Glyph::Run)
            .enabled(enabled)
            .show(ui)
            .arrow
    })
    .run();
}

#[test]
fn action_button_variants_conform() {
    testing::conformance("Pin tab", |ui, enabled| {
        ui.add(
            ActionButton::new(Glyph::Pin, "Pin tab")
                .selected(true)
                .enabled(enabled),
        )
    })
    .run();
    testing::conformance("Branch", |ui, enabled| {
        ui.add(
            ActionButton::new(Glyph::Branch, "Branch")
                .dropdown(true)
                .enabled(enabled),
        )
    })
    .run();
    testing::conformance("Close window", |ui, enabled| {
        ui.add(
            ActionButton::new(Glyph::Close, "Close window")
                .title_pane(true)
                .enabled(enabled),
        )
    })
    .run();
}

#[test]
fn status_bar_and_stripe_controls_conform() {
    testing::conformance("UTF-8", |ui, enabled| {
        ui.add(StatusBarWidget::new("UTF-8").enabled(enabled))
    })
    .run();
    testing::conformance("archipel", |ui, enabled| {
        ui.add_enabled_ui(enabled, |ui| {
            BreadcrumbsComponent::new()
                .crumb("archipel", Some(Glyph::Folder.into()))
                .show(ui)
                .crumbs
                .remove(0)
        })
        .inner
    })
    .run();
    testing::conformance("Project", |ui, enabled| {
        ui.add_enabled(enabled, SquareStripeButton::new(Glyph::Folder, "Project"))
    })
    .run();
}

#[test]
fn choices_conform() {
    let mut on = false;
    testing::conformance("Format on save", move |ui, enabled| {
        ui.add_enabled(enabled, JBCheckBox::new(&mut on, "Format on save"))
    })
    .run();
    let mut indent = "tabs";
    testing::conformance("Spaces", move |ui, enabled| {
        ui.add_enabled(enabled, JBRadioButton::new(&mut indent, "spaces", "Spaces"))
    })
    .run();
    let mut toggle = true;
    testing::conformance("Power save", move |ui, enabled| {
        ui.add_enabled(enabled, OnOffButton::new(&mut toggle, "Power save"))
    })
    .run();
}

#[test]
fn link_and_action_menu_item_conform() {
    testing::conformance("Configure…", |ui, enabled| {
        ui.add_enabled(enabled, ActionLink::new("Configure…"))
    })
    .run();
    testing::conformance("Copy", |ui, enabled| {
        ui.add(
            ActionMenuItem::new("Copy")
                .shortcut("Ctrl+C")
                .enabled(enabled),
        )
    })
    .run();
}

#[test]
fn collapsible_titled_separator_conforms() {
    let mut open = false;
    testing::conformance("Advanced", move |ui, enabled| {
        ui.add_enabled(
            enabled,
            TitledSeparator::new("Advanced").collapsible(&mut open),
        )
    })
    .run();
}

#[test]
fn collapsible_panel_conforms() {
    let mut collapsed = true;
    testing::conformance("Advanced settings", move |ui, enabled| {
        ui.add_enabled_ui(enabled, |ui| {
            archipel_kit::ui::CollapsiblePanel::new("Advanced settings", &mut collapsed)
                .show(ui, |ui| {
                    ui.label("Body");
                })
                .header
        })
        .inner
    })
    .run();
}

#[test]
fn colored_list_cell_renderer_conforms() {
    testing::conformance("main.rs", |ui, enabled| {
        ui.add_enabled_ui(enabled, |ui| {
            ColoredListCellRenderer::new("main.rs")
                .enabled(enabled)
                .show(ui)
        })
        .inner
        .response
    })
    .run();
}

#[test]
fn jb_text_field_conforms() {
    let mut text = String::from("archipel");
    testing::conformance("Project name", move |ui, enabled| {
        ui.add(
            JBTextField::new(&mut text)
                .label("Project name")
                .enabled(enabled),
        )
    })
    .no_click()
    .keys(&[])
    .run();
}

#[test]
fn jb_int_spinner_conforms() {
    let mut limit = 50_u32;
    testing::conformance("Limit", move |ui, enabled| {
        ui.add(
            JBIntSpinner::new(&mut limit)
                .name("Limit")
                .range(1..=500)
                .enabled(enabled),
        )
    })
    .no_click()
    .keys(&[])
    .run();
}

#[test]
fn code_editor_conforms() {
    let mut text = String::from("let a = 1;");
    testing::conformance("Script", move |ui, enabled| {
        ui.add(
            CodeEditor::new(&mut text)
                .label("Script")
                .enabled(enabled)
                .height(80.0),
        )
    })
    .no_click()
    .keys(&[])
    .run();
}

#[test]
fn code_view_and_editor_compose_audit_clean() {
    let mut text = String::from("{\n  \"a\": 1\n}");
    let mut harness = testing::harness(Theme::dark(), |ui| {
        CodeView::new("{\n  \"a\": 1\n}")
            .label("Response")
            .line_numbers(true)
            .folding(true)
            .height(80.0)
            .show(ui);
        ui.add(
            CodeEditor::new(&mut text)
                .label("Request")
                .line_numbers(true)
                .height(80.0),
        );
    });
    testing::audit(&mut harness).assert_clean();
}

#[test]
fn combo_box_conforms() {
    let mut method = "GET";
    testing::conformance("HTTP method", move |ui, enabled| {
        ui.add(
            ComboBox::new("method", &mut method, ["GET", "POST"])
                .label("HTTP method")
                .enabled(enabled),
        )
    })
    .run();
}

fn tree_table_rows() -> Vec<TreeNode> {
    vec![
        TreeNode::new("name", 0).editable(true),
        TreeNode::new("address", 0).icon(Glyph::Folder).parent(true),
        TreeNode::new("city", 1),
    ]
}

/// The collection holds the focus as a whole (Tab reaches it once); its rows are pointer
/// targets and its keys are the arrows, not Enter/Space.
#[test]
fn tree_table_conforms() {
    let rows = tree_table_rows();
    let mut columns = vec![
        ColumnInfo::new("Key", 120.0),
        ColumnInfo::new("Value", 120.0),
    ];
    let mut selected = None;
    testing::conformance("Document", move |ui, enabled| {
        ui.add_enabled_ui(enabled, |ui| {
            TreeTable::new("doc", &mut columns, &rows, &mut selected)
                .name("Document")
                .height(96.0)
                .show(ui, |_, _| Cell::new("value").editable(true))
                .response
        })
        .inner
    })
    .no_click()
    .keys(&[])
    .run();
}

#[test]
fn reorderable_editable_table_conforms_and_passes_the_audit() {
    let mut columns = vec![
        ColumnInfo::new("Name", 120.0),
        ColumnInfo::new("Size", 80.0).right(),
    ];
    let mut selected = None;
    testing::conformance("Files", move |ui, enabled| {
        ui.add_enabled_ui(enabled, |ui| {
            JBTable::new("files", &mut columns, 5, &mut selected)
                .name("Files")
                .reorderable(true)
                .height(96.0)
                .show(ui, |r, c| Cell::new(format!("{r}:{c}")).editable(c == 0))
                .response
        })
        .inner
    })
    .no_click()
    .keys(&[])
    .run();

    let mut columns = vec![
        ColumnInfo::new("Name", 120.0),
        ColumnInfo::new("Size", 80.0).right(),
    ];
    let mut selected = Some(0);
    let mut harness = testing::harness(Theme::dark(), |ui| {
        let _ = JBTable::new("files", &mut columns, 5, &mut selected)
            .name("Files")
            .reorderable(true)
            .height(96.0)
            .show(ui, |r, c| Cell::new(format!("{r}:{c}")).editable(c == 0));
    });
    harness.run();
    testing::audit(&mut harness).assert_clean();
    harness.get_by_label("Files").focus();
    harness.key_press(egui::Key::F2);
    harness.run();
    assert!(harness.query_by_label("Edit Name").is_some());
    let audit = testing::audit(&mut harness);
    for problem in [Problem::MissingName, Problem::SmallTarget, Problem::Overlap] {
        assert!(audit.of(problem).is_empty(), "{problem:?}: {audit:?}");
    }
}

/// The audit passes at rest, and with the editor open the field is named, big enough and
/// clear of other targets.
#[test]
fn tree_table_passes_the_audit_while_editing() {
    let rows = tree_table_rows();
    let mut columns = vec![
        ColumnInfo::new("Key", 120.0),
        ColumnInfo::new("Value", 120.0),
    ];
    let mut selected = Some(0);
    let mut harness = testing::harness(Theme::dark(), |ui| {
        let _ = TreeTable::new("doc", &mut columns, &rows, &mut selected)
            .name("Document")
            .height(96.0)
            .show(ui, |_, _| Cell::new("value").editable(true));
    });
    harness.run();
    testing::audit(&mut harness).assert_clean();
    harness.get_by_label("Document").focus();
    harness.key_press(egui::Key::F2);
    harness.run();
    assert!(harness.query_by_label("Edit Key").is_some());
    // Tab leaves (and so commits) the field, so reachability is not meaningful while it is
    // open; the field must still be named, big enough and clear of the other targets.
    let audit = testing::audit(&mut harness);
    for problem in [Problem::MissingName, Problem::SmallTarget, Problem::Overlap] {
        assert!(audit.of(problem).is_empty(), "{problem:?}: {audit:?}");
    }
}

#[test]
fn screens_with_navigation_and_collections_pass_the_audit() {
    let mut selected = 1;
    let tabs = vec![
        TabInfo::new("main.rs").closable(true),
        TabInfo::new("lib.rs"),
    ];
    let active = 0;
    let mut row = Some(0);
    let mut tree_row = None;
    let rows = vec![
        TreeNode::new("src", 0).icon(Glyph::Folder).parent(true),
        TreeNode::new("main.rs", 1),
    ];
    let mut columns = vec![
        ColumnInfo::new("Name", 120.0),
        ColumnInfo::new("Size", 80.0).right(),
    ];
    let mut table_row = None;
    let mut tree_columns = columns.clone();
    let mut tree_table_row = None;
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add(SegmentedButton::new(
            &mut selected,
            [(0, "Tree"), (1, "List")],
        ));
        let _ = JBTabs::new(&tabs, active).show(ui);
        let _ = JBList::new("list", 50, &mut row)
            .height(72.0)
            .show(ui, |i| ColoredListCellRenderer::new(format!("Item {i}")));
        let _ = Tree::new("tree", &rows, &mut tree_row)
            .height(48.0)
            .show(ui);
        let _ = JBTable::new("table", &mut columns, 20, &mut table_row)
            .height(72.0)
            .show(ui, |r, c| format!("{r}:{c}").into());
        let _ = TreeTable::new("tree-table", &mut tree_columns, &rows, &mut tree_table_row)
            .height(48.0)
            .show(ui, |r, c| format!("{r}:{c}").into());
    });
    testing::audit(&mut harness).assert_clean();
}

// The audit must catch what it promises to catch.

#[test]
fn audit_catches_unnamed_controls() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        // A raw click area with no name.
        let (rect, response) = ui.allocate_exact_size(egui::vec2(40.0, 24.0), egui::Sense::click());
        ui.painter().rect_filled(rect, 4.0, egui::Color32::GRAY);
        let _ = response;
    });
    let audit = testing::audit(&mut harness);
    assert_eq!(audit.of(Problem::MissingName).len(), 1, "{audit:?}");
}

#[test]
fn audit_catches_small_targets() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        let (_, response) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Tiny"));
    });
    let audit = testing::audit(&mut harness);
    assert_eq!(audit.of(Problem::SmallTarget).len(), 1, "{audit:?}");
}

#[test]
fn audit_catches_overlapping_controls() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        let rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(60.0, 24.0));
        ui.put(rect, Button::secondary("One"));
        ui.put(
            rect.translate(egui::vec2(20.0, 0.0)),
            Button::secondary("Two"),
        );
    });
    let audit = testing::audit(&mut harness);
    assert_eq!(audit.of(Problem::Overlap).len(), 1, "{audit:?}");
}

#[test]
fn audit_passes_a_clean_form() {
    let mut name = String::new();
    let mut agree = false;
    let mut harness = testing::harness(Theme::dark(), |ui| {
        ui.add(JBTextField::new(&mut name).label("Name"));
        ui.add(JBCheckBox::new(&mut agree, "I agree"));
        ui.add(Button::primary("Create"));
    });
    testing::audit(&mut harness).assert_clean();
}

#[test]
#[should_panic(expected = "a click on \"Broken\" did not activate it")]
fn conformance_catches_a_button_that_ignores_clicks() {
    testing::conformance("Broken", |ui, enabled| {
        // Looks like a button, but senses only hover.
        let (_, response) = ui.allocate_exact_size(egui::vec2(80.0, 28.0), egui::Sense::hover());
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, "Broken"));
        response
    })
    .run();
}

#[test]
#[should_panic(expected = "doesn't report disabled")]
fn conformance_catches_a_fake_disabled_state() {
    testing::conformance("Run", |ui, _enabled| ui.add(Button::primary("Run"))).run();
}

#[test]
fn key_value_text_cells_conform() {
    use archipel_kit::ui::{KeyValueEntry, KeyValueTable};
    for label in ["Key, row 1", "Value, row 1"] {
        let mut rows = vec![KeyValueEntry::new("page", "1")];
        testing::conformance(label, move |ui, enabled| {
            KeyValueTable::new("params", &mut rows)
                .read_only(!enabled)
                .show(ui)
                .response
        })
        .no_click()
        .keys(&[])
        .run();
    }
}

/// The table reports its cells through `changes`, not through one response: turns "this
/// change was reported" into the `clicked()` that `conformance` counts as an activation.
fn activated_if(mut response: egui::Response, activated: bool) -> egui::Response {
    if activated {
        response
            .flags
            .insert(egui::response::Flags::FAKE_PRIMARY_CLICKED);
    }
    response
}

#[test]
fn key_value_enable_check_box_conforms() {
    use archipel_kit::ui::{Change, KeyValueEntry, KeyValueTable};
    let mut rows = vec![KeyValueEntry::new("page", "1")];
    testing::conformance("Enabled, row 1", move |ui, enabled| {
        let out = KeyValueTable::new("params", &mut rows)
            .read_only(!enabled)
            .show(ui);
        activated_if(out.response, out.changes.contains(&Change::Toggled(0)))
    })
    .run();
}

#[test]
fn key_value_remove_button_conforms() {
    use archipel_kit::ui::{Change, KeyValueEntry, KeyValueTable};
    // Read-only tables have no delete button: the disabled state is a disabled `Ui`.
    testing::conformance("Remove, row 1", |ui, enabled| {
        // Rebuilt every frame: a removed row would take the button with it.
        let mut rows = vec![KeyValueEntry::new("page", "1")];
        ui.add_enabled_ui(enabled, |ui| {
            let out = KeyValueTable::new("params", &mut rows).show(ui);
            activated_if(out.response, out.changes.contains(&Change::Removed(0)))
        })
        .inner
    })
    .run();
}

#[test]
fn splitter_divider_conforms() {
    use archipel_kit::openapi::ui::Splitter;
    let mut ratio = 0.5;
    testing::conformance("Resize sidebar", move |ui, enabled| {
        ui.allocate_ui(egui::vec2(320.0, 120.0), |ui| {
            Splitter::horizontal("conformance", &mut ratio)
                .label("Resize sidebar")
                .enabled(enabled)
                .show(ui, |ui| ui.label("Sidebar"), |ui| ui.label("Request"))
                .divider
        })
        .inner
    })
    .run();
}

#[test]
fn collapsible_splitter_divider_conforms_open_and_collapsed() {
    use archipel_kit::openapi::ui::{Side, Splitter};
    for collapsed in [false, true] {
        let mut ratio = 0.5;
        testing::conformance("Resize sidebar", move |ui, enabled| {
            let mut collapsed = collapsed;
            ui.allocate_ui(egui::vec2(320.0, 120.0), |ui| {
                Splitter::horizontal("conformance", &mut ratio)
                    .label("Resize sidebar")
                    .enabled(enabled)
                    .collapsible(Side::First, &mut collapsed)
                    .show(ui, |ui| ui.label("Sidebar"), |ui| ui.label("Request"))
                    .divider
            })
            .inner
        })
        .run();
    }
}

/// A row the arrow keys reach but Tab never focuses: a bare accessibility node.
fn roving_row(ui: &mut egui::Ui, id: &str, label: &str) {
    use egui::accesskit::{Action, Role};
    let (rect, _) = ui.allocate_exact_size(egui::vec2(120.0, 20.0), egui::Sense::hover());
    ui.ctx().accesskit_node_builder(egui::Id::new(id), |node| {
        node.set_role(Role::ListItem);
        node.set_label(label);
        node.add_action(Action::Focus);
        node.set_bounds(egui::accesskit::Rect {
            x0: rect.min.x.into(),
            y0: rect.min.y.into(),
            x1: rect.max.x.into(),
            y1: rect.max.y.into(),
        });
    });
}

#[test]
fn audit_still_catches_a_focusable_row_outside_any_collection() {
    let mut harness = testing::harness(Theme::dark(), |ui| {
        roving_row(ui, "lonely", "Lonely row");
    });
    let audit = testing::audit(&mut harness);
    assert_eq!(audit.of(Problem::Unreachable).len(), 1, "{audit:?}");
}

#[test]
fn one_pixel_splitter_divider_conforms() {
    use archipel_kit::openapi::ui::OnePixelSplitter;
    let mut ratio = 0.5;
    testing::conformance("Resize sidebar", move |ui, enabled| {
        ui.allocate_ui(egui::vec2(320.0, 120.0), |ui| {
            OnePixelSplitter::horizontal("conformance", &mut ratio)
                .label("Resize sidebar")
                .enabled(enabled)
                .show(ui, |ui| ui.label("Sidebar"), |ui| ui.label("Request"))
                .divider
        })
        .inner
    })
    .run();
}

#[test]
fn jb_splitter_divider_conforms() {
    use archipel_kit::host::InMemoryProperties;
    use archipel_kit::openapi::ui::JBSplitter;
    let mut ratio = 0.5;
    let mut properties = InMemoryProperties::new();
    testing::conformance("Resize sidebar", move |ui, enabled| {
        ui.allocate_ui(egui::vec2(320.0, 120.0), |ui| {
            JBSplitter::vertical(
                "conformance",
                &mut ratio,
                "conformance.key",
                &mut properties,
            )
            .label("Resize sidebar")
            .enabled(enabled)
            .show(ui, |ui| ui.label("Top"), |ui| ui.label("Bottom"))
            .divider
        })
        .inner
    })
    .run();
}

#[test]
fn icon_only_status_bar_widget_conforms() {
    testing::conformance("Notifications", |ui, enabled| {
        ui.add(
            StatusBarWidget::new("")
                .icon(Glyph::Bell)
                .name("Notifications")
                .enabled(enabled),
        )
    })
    .run();
}
