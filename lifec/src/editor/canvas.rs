use atlier::system::App;
use imgui::{ChildWindow, DragDropFlags, StyleVar, Window};
use tracing::{event, Level};

use crate::engine::WorkspaceCommand;

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Canvas {
    workspace: Vec<WorkspaceCommand>,
    opened: bool,
}

impl App for Canvas {
    fn name() -> &'static str {
        "canvas"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        let window_padding = ui.push_style_var(StyleVar::WindowPadding([16.0, 16.0]));
        let frame_padding = ui.push_style_var(StyleVar::FramePadding([8.0, 5.0]));
        Window::new("Canvas").build(ui, || {
            ChildWindow::new("Plugins").build(ui, || {
                imgui::TreeNode::new("Plugins")
                    .opened(self.opened, imgui::Condition::Always)
                    .build(ui, || {
                        let snapshot = self.workspace.clone();
                        for (idx, w) in snapshot.iter().enumerate() {
                            imgui::TreeNode::new(format!("{} - {idx}", w))
                                .label::<String, _>(format!("{}", w))
                                .leaf(true)
                                .build(ui, || {
                                    if let Some(tooltip) =
                                        imgui::drag_drop::DragDropSource::new("REORDER")
                                            .flags(DragDropFlags::SOURCE_NO_PREVIEW_TOOLTIP)
                                            .begin_payload(ui, idx)
                                    {
                                        tooltip.end();
                                    }

                                    if let Some(target) = imgui::drag_drop::DragDropTarget::new(ui)
                                    {
                                        match target.accept_payload::<WorkspaceCommand, _>(
                                            "ADD_PLUGIN",
                                            DragDropFlags::empty(),
                                        ) {
                                            Some(result) => match result {
                                                Ok(command) => {
                                                    self.workspace.insert(idx, command.data);
                                                    self.workspace.remove(idx + 1);
                                                }
                                                Err(err) => {
                                                    event!(
                                                        Level::ERROR,
                                                        "Error accepting workspace command, {err}"
                                                    );
                                                }
                                            },
                                            None => {}
                                        }

                                        match target.accept_payload::<usize, _>(
                                            "REORDER",
                                            DragDropFlags::empty(),
                                        ) {
                                            Some(result) => match result {
                                                Ok(data) => {
                                                    let from = data.data;
                                                    self.workspace.swap(idx, from);
                                                }
                                                Err(err) => {
                                                    event!(
                                                        Level::ERROR,
                                                        "Error accepting workspace command, {err}"
                                                    );
                                                }
                                            },
                                            None => {}
                                        }
                                    }
                                });
                        }
                    });
            });

            if let Some(target) = imgui::drag_drop::DragDropTarget::new(ui) {
                match target
                    .accept_payload::<WorkspaceCommand, _>("ADD_PLUGIN", DragDropFlags::empty())
                {
                    Some(result) => match result {
                        Ok(command) => {
                            self.workspace.push(command.data);
                            self.opened = true;
                        }
                        Err(err) => {
                            event!(Level::ERROR, "Error accepting workspace command, {err}");
                        }
                    },
                    None => {}
                }
            }
        });
        window_padding.end();
        frame_padding.end();
    }

    fn display_ui(&self, _: &imgui::Ui) {
        // No-op
    }
}
