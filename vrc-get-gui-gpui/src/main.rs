mod backend;

use backend::{ProjectRow, load_projects};

use gpui::prelude::*;
use gpui::{
    App, Application, Context, IntoElement, ParentElement, Render, SharedString, Styled, Window,
    WindowOptions, div,
};
use gpui_component::{
    Root, StyledExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    spinner::Spinner,
    table::{Column, Table, TableDelegate, TableState},
    v_flex,
};
use vrc_get_gui_runtime::TokioBridge;

// ---------------------------------------------------------------------------
// Projects table delegate
// ---------------------------------------------------------------------------

struct ProjectsDelegate {
    columns: Vec<Column>,
    rows: Vec<ProjectRow>,
}

impl ProjectsDelegate {
    fn new() -> Self {
        Self {
            columns: vec![
                Column::new("name", "Name"),
                Column::new("type", "Type"),
                Column::new("unity", "Unity"),
                Column::new("path", "Path"),
            ],
            rows: vec![],
        }
    }

    fn set_rows(&mut self, rows: Vec<ProjectRow>) {
        self.rows = rows;
    }
}

impl TableDelegate for ProjectsDelegate {
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
        let row = &self.rows[row_ix];
        let value: SharedString = match col_ix {
            0 => row.name.clone().into(),
            1 => row.project_type.clone().into(),
            2 => row.unity.clone().into(),
            _ => row.path.clone().into(),
        };
        div().child(value)
    }
}

// ---------------------------------------------------------------------------
// Loading state
// ---------------------------------------------------------------------------

enum ProjectsData {
    Loading,
    Loaded(Vec<ProjectRow>),
    Error(String),
}

// ---------------------------------------------------------------------------
// Root view
// ---------------------------------------------------------------------------

struct ProjectsView {
    bridge: TokioBridge,
    search_input: gpui::Entity<InputState>,
    table_state: gpui::Entity<TableState<ProjectsDelegate>>,
    data: ProjectsData,
}

impl ProjectsView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let bridge = TokioBridge::new("vrc-get-gpui-runtime");
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search projects"));
        let table_state = cx.new(|cx| TableState::new(ProjectsDelegate::new(), window, cx));

        // Re-filter the table whenever the search input changes.
        cx.observe(&search_input, |view, _, cx| {
            view.apply_search(cx);
        })
        .detach();

        let mut view = Self {
            bridge,
            search_input,
            table_state,
            data: ProjectsData::Loading,
        };
        view.reload(cx);
        view
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.data = ProjectsData::Loading;
        cx.notify();

        let rx = self
            .bridge
            .call(load_projects())
            .expect("tokio bridge still alive");

        cx.spawn(async move |this: gpui::WeakEntity<ProjectsView>, cx: &mut gpui::AsyncApp| {
            match rx.await {
                Ok(Ok(rows)) => {
                    this.update(cx, |view, cx| {
                        let search = view.search_input.read(cx).value().to_lowercase();
                        let filtered = filter_rows(&rows, &search);
                        view.table_state.update(cx, |table, _| {
                            table.delegate_mut().set_rows(filtered);
                        });
                        view.data = ProjectsData::Loaded(rows);
                        cx.notify();
                    })
                    .ok();
                }
                Ok(Err(err)) => {
                    this.update(cx, |view, cx| {
                        view.data = ProjectsData::Error(err.to_string());
                        cx.notify();
                    })
                    .ok();
                }
                Err(_) => {}
            }
        })
        .detach();
    }

    fn apply_search(&mut self, cx: &mut Context<Self>) {
        let ProjectsData::Loaded(ref all_rows) = self.data else {
            return;
        };
        let search = self.search_input.read(cx).value().to_lowercase();
        let rows = filter_rows(all_rows, &search);
        self.table_state.update(cx, |table, _| {
            table.delegate_mut().set_rows(rows);
        });
        cx.notify();
    }
}

fn filter_rows(rows: &[ProjectRow], search: &str) -> Vec<ProjectRow> {
    if search.is_empty() {
        rows.to_vec()
    } else {
        rows.iter()
            .filter(|r| {
                r.name.to_lowercase().contains(search)
                    || r.path.to_lowercase().contains(search)
            })
            .cloned()
            .collect()
    }
}

impl Render for ProjectsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_loading = matches!(self.data, ProjectsData::Loading);
        let error_msg: Option<SharedString> = if let ProjectsData::Error(ref e) = self.data {
            Some(e.clone().into())
        } else {
            None
        };

        // Toolbar
        let search_el = Input::new(&self.search_input).cleanable(true);

        let reload_btn = Button::new("reload")
            .label("Reload")
            .on_click(cx.listener(|view, _event: &gpui::ClickEvent, _window, cx| {
                view.reload(cx);
            }));

        let add_btn = Button::new("add-project")
            .primary()
            .label("Add Project")
            .on_click(|_event, _window, _cx| {
                let _ = rfd::FileDialog::new().pick_folder();
            });

        let toolbar = h_flex()
            .items_center()
            .justify_between()
            .child(search_el)
            .child(h_flex().gap_2().child(reload_btn).child(add_btn));

        // Body
        let body: gpui::AnyElement = if is_loading {
            h_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_2()
                .child(Spinner::new())
                .child(div().child("Loading projects…"))
                .into_any_element()
        } else if let Some(msg) = error_msg {
            div()
                .p_4()
                .text_color(gpui::red())
                .child(format!("Error: {msg}"))
                .into_any_element()
        } else {
            Table::new(&self.table_state)
                .stripe(true)
                .into_any_element()
        };

        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(div().font_bold().child("Projects"))
                    .when(is_loading, |el| el.child(Spinner::new())),
            )
            .child(toolbar)
            .child(body)
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);

        cx.open_window(WindowOptions::default(), |window, cx| {
            let view = cx.new(|cx| ProjectsView::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("opening gpui window");

        cx.activate(true);
    });
}
