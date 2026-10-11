# Workbench

`Workbench` assembles a center island and optional left, right and bottom tool windows.
All gutters and splitters use the kit tokens. Its optional toolbar uses `WindowToolbar`;
the optional status area stays on the themed frame. Applications supply the content.

```rust
use archipel_kit::components::{Workbench, WorkbenchState, WindowChrome, StatusBarItem};
let mut state = WorkbenchState::default();
# egui::__run_test_ctx(|ctx| {
Workbench::new("workspace", &mut state)
    .toolbar(WindowChrome::Integrated, |ui| { ui.label("My application"); })
    .left("Browser", |ui| { ui.label("Collections"); })
    .right("Details", |ui| { ui.label("Properties"); })
    .bottom("Messages", |ui| { ui.label("Ready"); })
    .status(|ui| { ui.add(StatusBarItem::new("UTF-8")); })
    .show(ctx, |ui| { ui.label("Document"); });
# });
```

`WorkbenchRegion` identifies the left, right or bottom maximized tool window.

The caller owns `WorkbenchState`, which supports serde serialization independently of
egui. Drag or use the splitter arrow keys to resize; Enter collapses or expands its tool
window. Headers expose named Hide and Maximize/Restore buttons. To reopen a hidden pane,
set its `*_hidden` flag to false from the application's navigation or shortcut.

Small viewports temporarily hide right, then left tool windows to preserve the central
work area. A short viewport hides the bottom pane. Preferred ratios and explicit hidden
flags are retained, so panes return when the window grows. Missing or hidden maximized
panes return to the center. Hidden content callbacks are not invoked.

`show_inside` embeds only the docked islands into a bounded UI area. `show` additionally
supplies the outer gutter, toolbar and status area. Floating windows, tool-window
stripes, multiple tools per region, and SDK/app migration remain separate follow-ups;
this component does not silently store unsupported layout modes.

![Three islands dark](img/workbench_3.dark.png)
![Three islands light](img/workbench_3.light.png)
![Four islands dark](img/workbench_4.dark.png)
![Four islands light](img/workbench_4.light.png)
