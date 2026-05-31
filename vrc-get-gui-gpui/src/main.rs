use gpui::{
    App, Application, Context, IntoElement, ParentElement, Render, Styled, Window, WindowOptions,
    div,
};
use gpui::prelude::*;
use gpui_component::{
    Root, WindowExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    table::{Column, Table, TableDelegate, TableState},
    v_flex,
};
use vrc_get_gui_runtime::TokioBridge;

#[derive(Clone)]
struct PackageRow {
    name: String,
    version: String,
    source: String,
}

struct PackageTableDelegate {
    columns: Vec<Column>,
    rows: Vec<PackageRow>,
}

impl PackageTableDelegate {
    fn new() -> Self {
        Self {
            columns: vec![
                Column::new("name", "Package"),
                Column::new("version", "Version"),
                Column::new("source", "Source"),
            ],
            rows: vec![
                PackageRow {
                    name: "com.vrchat.base".to_owned(),
                    version: "3.7.5".to_owned(),
                    source: "Official".to_owned(),
                },
                PackageRow {
                    name: "com.vrchat.worlds".to_owned(),
                    version: "3.7.5".to_owned(),
                    source: "Official".to_owned(),
                },
                PackageRow {
                    name: "com.anatawa12.package-installer".to_owned(),
                    version: "2.0.0".to_owned(),
                    source: "Community".to_owned(),
                },
            ],
        }
    }
}

impl TableDelegate for PackageTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> &Column {
        &self.columns[col_ix]
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let value = match col_ix {
            0 => self.rows[row_ix].name.clone(),
            1 => self.rows[row_ix].version.clone(),
            _ => self.rows[row_ix].source.clone(),
        };
        div().child(value)
    }
}

struct PackageManagementPoc {
    _bridge: TokioBridge,
    search_input: gpui::Entity<InputState>,
    table_state: gpui::Entity<TableState<PackageTableDelegate>>,
}

impl PackageManagementPoc {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search package"));
        let table_state = cx.new(|cx| TableState::new(PackageTableDelegate::new(), window, cx));

        Self {
            _bridge: TokioBridge::new("vrc-get-gpui-runtime"),
            search_input,
            table_state,
        }
    }
}

impl Render for PackageManagementPoc {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(Input::new(&self.search_input).cleanable())
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("open-native-dialog")
                                    .label("Open with rfd")
                                    .on_click(|_, _, _| {
                                        let _ = rfd::FileDialog::new().pick_folder();
                                    }),
                            )
                            .child(
                                Button::new("show-dialog")
                                    .primary()
                                    .label("Show dialog")
                                    .on_click(|_, window, cx| {
                                        window.open_dialog(cx, |dialog, _, _| {
                                            dialog
                                                .title("Validation dialog")
                                                .confirm()
                                                .child("Dialog wiring check for GPUI migration")
                                        });
                                    }),
                            ),
                    ),
            )
            .child(Table::new(&self.table_state).stripe(true))
            .child(
                div().opacity(0.7).child(
                    "POC target: package table + dialog + text input before full migration",
                ),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);

        cx.open_window(WindowOptions::default(), |window, cx| {
            let view = cx.new(|cx| PackageManagementPoc::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("opening gpui window");

        cx.activate(true);
    });
}
