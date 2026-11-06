use egui::{
    vec2, Align, Color32, CornerRadius, Layout, Rect, Sense, Stroke,
};

use super::{
    icons, theme::{self, Tokens}, Dialog, LeftPanel, Mode, PdfCraftApp,
    QuickTool, RightPanel,
};
use crate::canvas::Fit;

/// The primary ribbon tabs modelled after Foxit PhantomPDF Standard 7.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RibbonTab {
    #[default]
    Home,
    Edit,
    Organize,
    Comment,
    View,
    Form,
    Protect,
    Convert,
    Sign,
    Help,
}

/// Foxit PhantomPDF plum purple accent colors.
pub const FOXIT_PLUM: Color32 = Color32::from_rgb(112, 32, 97);
pub const FOXIT_PLUM_HOVER: Color32 = Color32::from_rgb(136, 40, 118);
pub const FOXIT_PLUM_ACTIVE: Color32 = Color32::from_rgb(92, 24, 78);
pub const FOXIT_PLUM_LIGHT: Color32 = Color32::from_rgb(243, 230, 241);

/// Action for prominent "Unlock PDF" button (bypasses restrictions/passwords without prompt).
pub fn unlock_pdf_action(app: &mut PdfCraftApp) {
    if app.active.is_none() {
        app.open_dialog();
    } else if app.active_ids().and_then(|(_, id)| app.session.get(id)).is_some_and(|d| d.security_summary().is_some()) {
        app.dialog = Some(Dialog::UnlockPdf);
    } else {
        app.notify_tr("This document isn't password-protected");
    }
}

/// Top Quick Access Toolbar & Title bar (integrated into tab strip)
pub fn quick_access_bar(_app: &mut PdfCraftApp, _ui: &mut egui::Ui) {}

/// The ribbon header row: Foxit's plum FILE tab, the quick access toolbar, the workspace (mode)
/// tabs, and the ribbon tabs themselves. There is deliberately no legacy menu bar: Foxit
/// PhantomPDF has none either, and every menu command is reachable from the ribbon, the FILE
/// home screen or the command palette.
pub const HEADER_HEIGHT: f32 = 36.0;
/// The grouped command strip below the header, at Foxit PhantomPDF's height.
pub const RIBBON_HEIGHT: f32 = 104.0;

pub fn ribbon_header(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());

    egui::Panel::top("ribbon_header")
        .exact_size(HEADER_HEIGHT)
        .frame(
            egui::Frame::NONE
                .fill(if t.dark() { t.chrome } else { Color32::from_rgb(243, 243, 246) })
                .inner_margin(egui::Margin { left: 0, right: 8, top: 0, bottom: 0 })
                .stroke(Stroke::new(1.0, t.divider)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                // The header never overlaps itself: in a narrow window the mode tabs give way
                // first, then the search box shortens, then it goes. The ribbon tabs always stay.
                let width = ui.available_width();

                // 1. Iconic Foxit Plum FILE Button, flush with the left edge.
                let file_active = app.active.is_none();
                let file_btn = render_file_tab_button(ui, file_active);
                if file_btn.clicked() {
                    // Clicking FILE takes user to Home backstage or opens file menu
                    app.active = None;
                    app.combine_tab.focused = false;
                }

                // 2. Quick Access Toolbar
                ui.add_space(4.0);
                let qat_size = 26.0;
                let save_tip = crate::commands::command_tip(ui.ctx(), tl!("Save ({key})"), "file.save");
                if icons::button(ui, "save", qat_size, false, &save_tip).clicked() {
                    app.run_command("file.save");
                }
                let has_doc = app.active.is_some();
                let may_print = !has_doc || pdfcraft_engine::commands::command("print.dialog").is_some_and(|c| app.command_enabled(c));
                let print_tip = crate::commands::command_tip(ui.ctx(), tl!("Print ({key})"), "print.dialog");
                let mut print = ui
                    .add_enabled_ui(has_doc && may_print, |ui| icons::button(ui, "printer", qat_size, false, &print_tip))
                    .inner;
                if has_doc && !may_print {
                    print = print.on_disabled_hover_text(crate::print_refused());
                }
                if print.clicked() {
                    app.run_command("print.dialog");
                }
                let undo_tip = crate::commands::command_tip(ui.ctx(), tl!("Undo ({key})"), "edit.undo");
                if icons::button(ui, "undo-2", qat_size, false, &undo_tip).clicked() {
                    app.run_command("edit.undo");
                }
                let redo_tip = crate::commands::command_tip(ui.ctx(), tl!("Redo ({key})"), "edit.redo");
                if icons::button(ui, "redo-2", qat_size, false, &redo_tip).clicked() {
                    app.run_command("edit.redo");
                }
                let unlock_tip = tl!("Unlock PDF - Remove restrictions and passwords without entering password");
                if icons::button(ui, "lock-open", qat_size, false, unlock_tip).clicked() {
                    unlock_pdf_action(app);
                }

                // 3. Workspace mode tabs (All tools, Edit, Convert, E-Sign), while they fit beside
                // the ribbon tabs and the quick access bar's right-hand end.
                if width >= 1180.0 {
                    ribbon_separator(ui, &t);
                    let aux_tabs = [
                        (tl!("All tools"), app.mode == Mode::AllTools && app.ribbon_tab != RibbonTab::Edit, Mode::AllTools),
                        (tl!("Edit"), app.mode == Mode::Edit, Mode::Edit),
                        (tl!("Convert"), app.mode == Mode::Convert, Mode::Convert),
                        (tl!("E-Sign"), app.mode == Mode::Sign, Mode::Sign),
                    ];

                    for (label, active, mode) in aux_tabs {
                        if render_mode_tab_button(ui, &t, label, active).clicked() {
                            app.switch_mode(mode, true);
                            app.ribbon_tab = match mode {
                                Mode::Edit => RibbonTab::Edit,
                                Mode::Convert => RibbonTab::Convert,
                                Mode::Sign => RibbonTab::Sign,
                                _ => RibbonTab::Home,
                            };
                        }
                    }
                }

                // 4. Ribbon Tabs (HOME, ORGANIZE, COMMENT, VIEW, FORM, PROTECT, HELP)
                ribbon_separator(ui, &t);
                let tabs: [(RibbonTab, &str, Mode); 7] = [
                    (RibbonTab::Home, "HOME", Mode::AllTools),
                    (RibbonTab::Organize, "ORGANIZE", Mode::AllTools),
                    (RibbonTab::Comment, "COMMENT", Mode::AllTools),
                    (RibbonTab::View, "VIEW", Mode::AllTools),
                    (RibbonTab::Form, "FORM", Mode::AllTools),
                    (RibbonTab::Protect, "PROTECT", Mode::AllTools),
                    (RibbonTab::Help, "HELP", Mode::AllTools),
                ];

                for (tab, display_label, mode) in tabs {
                    let active = app.ribbon_tab == tab && app.mode != Mode::Edit && app.mode != Mode::Convert && app.mode != Mode::Sign;
                    if render_ribbon_tab_button(ui, &t, display_label, active).clicked() {
                        app.ribbon_tab = tab;
                        if app.mode != mode && !(tab == RibbonTab::Home && app.mode == Mode::Read) {
                            app.select_mode(mode);
                        }
                    }
                }

                // 5. Far right: Search, the compact menu and the ribbon collapse toggle.
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let collapse_icon = if app.ribbon_collapsed { "chevron-down" } else { "chevron-up" };
                    let collapse_tip = if app.ribbon_collapsed { tl!("Expand ribbon") } else { tl!("Collapse ribbon") };
                    if icons::button(ui, collapse_icon, 26.0, false, collapse_tip).clicked() {
                        app.ribbon_collapsed = !app.ribbon_collapsed;
                    }
                    if icons::button(ui, "align-justify", 26.0, app.menu_bar_open, tl!("Menu")).clicked() {
                        app.menu_bar_open = !app.menu_bar_open;
                    }
                    let search_width = if width >= 1300.0 { 190.0 } else if width >= 1100.0 { 130.0 } else { 0.0 };
                    if search_width > 0.0 && super::widgets::search_box(ui, tl!("Find"), search_width).clicked() {
                        app.palette_open = true;
                    }
                });
            });
        });
}

/// A thin divider between the header's groups.
fn ribbon_separator(ui: &mut egui::Ui, t: &Tokens) {
    ui.add_space(4.0);
    ui.painter().vline(ui.cursor().left(), ui.max_rect().y_range().shrink(8.0), Stroke::new(1.0, t.divider));
    ui.add_space(4.0);
}

/// The compact legacy menu (File, Edit, View…), hidden until the header's Menu button asks for it.
/// The ribbon carries the everyday commands; this keeps the few that have no ribbon home within
/// reach without putting a second menu bar on screen by default.
pub fn legacy_menu_bar(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::top("legacy_menu_bar")
        .exact_size(28.0)
        .frame(
            egui::Frame::NONE
                .fill(if t.dark() { t.chrome } else { Color32::from_rgb(243, 243, 246) })
                .inner_margin(egui::Margin { left: 8, right: 8, top: 0, bottom: 0 })
                .stroke(Stroke::new(1.0, t.divider)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                app.menu_bar_ui(ui);
            });
        });
}

/// The Plum Foxit "FILE" button.
fn render_file_tab_button(ui: &mut egui::Ui, active: bool) -> egui::Response {
    let height = HEADER_HEIGHT;
    let width = 66.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, active, "FILE"));
    let press = theme::Press::track(ui, &resp);

    let bg = press.fill(FOXIT_PLUM, FOXIT_PLUM_HOVER, FOXIT_PLUM_ACTIVE);

    ui.painter().rect_filled(rect, CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 }, bg);

    let text_galley = ui.fonts_mut(|f| f.layout_no_wrap("FILE".to_string(), theme::semibold(13.0), Color32::WHITE));
    let text_pos = egui::pos2(
        rect.center().x - text_galley.size().x / 2.0,
        rect.center().y - text_galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, text_galley, Color32::WHITE);

    theme::hand(resp)
}

/// A Ribbon Tab button (HOME, EDIT, ORGANIZE, etc.).
fn render_ribbon_tab_button(
    ui: &mut egui::Ui,
    t: &Tokens,
    display_label: &str,
    active: bool,
) -> egui::Response {
    let font = if active { theme::semibold(12.5) } else { theme::medium(12.5) };
    let text_color = if active {
        if t.dark() { t.accent_text } else { FOXIT_PLUM }
    } else {
        if t.dark() { t.text_muted } else { Color32::from_rgb(70, 70, 75) }
    };

    let galley = ui.fonts_mut(|f| f.layout_no_wrap(display_label.to_string(), font, text_color));
    let width = galley.size().x + 24.0;
    let height = HEADER_HEIGHT;

    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let tab_widget_label = format!("{display_label} Tab");
    resp.widget_info(move || egui::WidgetInfo::selected(egui::Role::Tab, true, active, tab_widget_label.clone()));
    let press = theme::Press::track(ui, &resp);

    if active {
        let ribbon_bg = if t.dark() { t.panel } else { Color32::WHITE };
        ui.painter().rect_filled(rect, CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 }, ribbon_bg);
        // Border on top, left, right to seamlessly join with the ribbon strip below
        let border_stroke = Stroke::new(1.0, if t.dark() { t.divider } else { Color32::from_rgb(205, 205, 210) });
        ui.painter().line_segment([rect.left_top(), rect.right_top()], border_stroke);
        ui.painter().line_segment([rect.left_top(), rect.left_bottom()], border_stroke);
        ui.painter().line_segment([rect.right_top(), rect.right_bottom()], border_stroke);
    } else if resp.hovered() {
        press.wash(ui, rect, 5, false);
    }

    let pos = egui::pos2(
        rect.center().x - galley.size().x / 2.0,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(pos, galley, text_color);

    theme::hand(resp)
}

/// An auxiliary Mode Tab button (All tools, Edit, Convert, E-Sign) with Role::Button.
fn render_mode_tab_button(
    ui: &mut egui::Ui,
    t: &Tokens,
    label: &str,
    active: bool,
) -> egui::Response {
    let font = if active { theme::semibold(12.5) } else { theme::medium(12.5) };
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), font, if active { t.text } else { t.text_muted }));
    let width = galley.size().x + 18.0;
    let height = 30.0;

    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, ui.is_enabled(), active, label));
    let press = theme::Press::track(ui, &resp);

    if active {
        let ribbon_bg = if t.dark() { t.panel } else { Color32::WHITE };
        ui.painter().rect_filled(rect, CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 }, ribbon_bg);
        let border_stroke = Stroke::new(1.0, if t.dark() { t.divider } else { Color32::from_rgb(205, 205, 210) });
        ui.painter().line_segment([rect.left_top(), rect.right_top()], border_stroke);
        ui.painter().line_segment([rect.left_top(), rect.left_bottom()], border_stroke);
        ui.painter().line_segment([rect.right_top(), rect.right_bottom()], border_stroke);
    } else if resp.hovered() {
        press.wash(ui, rect, 4, false);
    }

    let pos = egui::pos2(
        rect.center().x - galley.size().x / 2.0,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(pos, galley, if active { t.text } else { t.text_muted });

    theme::hand(resp)
}

/// The Foxit PhantomPDF Grouped Command Strip.
pub fn ribbon_command_strip(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());

    egui::Panel::top("ribbon_strip")
        .exact_size(RIBBON_HEIGHT)
        .frame(
            egui::Frame::NONE
                .fill(if t.dark() { t.panel } else { Color32::WHITE })
                .inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 2 })
                .stroke(Stroke::new(1.0, t.divider)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

                match app.ribbon_tab {
                    RibbonTab::Home => show_home_ribbon(app, ui, &t),
                    RibbonTab::Edit => show_edit_ribbon(app, ui, &t),
                    RibbonTab::Organize => show_organize_ribbon(app, ui, &t),
                    RibbonTab::Comment => show_comment_ribbon(app, ui, &t),
                    RibbonTab::View => show_view_ribbon(app, ui, &t),
                    RibbonTab::Form => show_form_ribbon(app, ui, &t),
                    RibbonTab::Protect => show_protect_ribbon(app, ui, &t),
                    RibbonTab::Convert => show_convert_ribbon(app, ui, &t),
                    RibbonTab::Sign => show_sign_ribbon(app, ui, &t),
                    RibbonTab::Help => show_help_ribbon(app, ui, &t),
                }
            });
        });
}

/// Ribbon Group wrapper with bottom title and vertical divider.
fn ribbon_group<R>(
    ui: &mut egui::Ui,
    title: &str,
    t: &Tokens,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let start_x = ui.cursor().left();
    let res = ui.vertical(|ui| {
        let r = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
            add_contents(ui)
        }).inner;
        let end_x = ui.cursor().left();
        let group_w = (end_x - start_x).max(32.0);
        let title_galley = ui.fonts_mut(|f| {
            f.layout_no_wrap(
                tl!(title).to_string(),
                theme::regular(10.5),
                if t.dark() { t.text_muted } else { Color32::from_rgb(125, 125, 135) },
            )
        });
        let label_pos = egui::pos2(
            start_x + (group_w - title_galley.size().x).max(0.0) / 2.0,
            ui.cursor().top() + 1.0,
        );
        ui.painter().galley(label_pos, title_galley, if t.dark() { t.text_muted } else { Color32::from_rgb(125, 125, 135) });
        r
    }).inner;

    let divider_x = ui.cursor().left() + 6.0;
    let divider_y = ui.max_rect().y_range().shrink(4.0);
    ui.painter().vline(divider_x, divider_y, Stroke::new(1.0, t.divider));
    ui.add_space(12.0);

    res
}

/// Large Foxit-style Ribbon Button (icon above label).
pub fn large_ribbon_button(
    ui: &mut egui::Ui,
    t: &Tokens,
    icon: &str,
    label: &str,
    active: bool,
    tooltip: &str,
) -> egui::Response {
    let font = if active { theme::semibold(11.5) } else { theme::medium(11.5) };
    let text = tl!(label);
    let galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(text.to_string(), font, if active { t.accent_text } else { t.text })
    });
    let width = (galley.size().x + 16.0).max(62.0);
    let height = 72.0;

    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, text));
    let press = theme::Press::track(ui, &resp);

    if active {
        ui.painter().rect(
            rect,
            CornerRadius::same(4),
            if t.dark() { t.hover } else { FOXIT_PLUM_LIGHT },
            Stroke::new(1.0, FOXIT_PLUM),
            egui::StrokeKind::Inside,
        );
    } else {
        press.wash(ui, rect, 4, false);
    }

    let body = rect.translate(press.offset());
    let icon_color = if active { FOXIT_PLUM } else { t.icon };
    icons::paint(ui, Rect::from_center_size(body.center_top() + vec2(0.0, 22.0), vec2(28.0, 28.0)), icon, 28.0, icon_color);

    let text_pos = egui::pos2(
        body.center().x - galley.size().x / 2.0,
        body.bottom() - galley.size().y - 6.0,
    );
    ui.painter().galley(text_pos, galley, if active { FOXIT_PLUM } else { t.text });

    let resp = theme::hand(resp);
    if tooltip.is_empty() { resp } else { resp.on_hover_text(tooltip) }
}

/// Prominent "Unlock PDF" button (highlighted with plum accent).
pub fn unlock_ribbon_button(
    ui: &mut egui::Ui,
    _t: &Tokens,
    active: bool,
    tooltip: &str,
) -> egui::Response {
    let font = theme::semibold(11.5);
    let text = tl!("Unlock PDF");
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), font, Color32::WHITE));
    let width = (galley.size().x + 18.0).max(70.0);
    let height = 72.0;

    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::MenuItem, ui.is_enabled(), active, text));
    let press = theme::Press::track(ui, &resp);

    let fill_color = press.fill(FOXIT_PLUM, FOXIT_PLUM_HOVER, FOXIT_PLUM_ACTIVE);

    ui.painter().rect(
        rect,
        CornerRadius::same(4),
        fill_color,
        Stroke::new(1.0, FOXIT_PLUM_HOVER),
        egui::StrokeKind::Inside,
    );

    let body = rect.translate(press.offset());
    icons::paint(ui, Rect::from_center_size(body.center_top() + vec2(0.0, 22.0), vec2(28.0, 28.0)), "lock-open", 28.0, Color32::WHITE);

    let text_pos = egui::pos2(
        body.center().x - galley.size().x / 2.0,
        body.bottom() - galley.size().y - 6.0,
    );
    ui.painter().galley(text_pos, galley, Color32::WHITE);

    let resp = theme::hand(resp);
    if tooltip.is_empty() { resp } else { resp.on_hover_text(tooltip) }
}

/// Compact Ribbon Button (icon on left, label on right).
pub fn small_ribbon_button(
    ui: &mut egui::Ui,
    t: &Tokens,
    icon: &str,
    label: &str,
    active: bool,
    tooltip: &str,
) -> egui::Response {
    let font = if active { theme::semibold(11.5) } else { theme::medium(11.5) };
    let text = tl!(label);
    let galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(text.to_string(), font, if active { t.accent_text } else { t.text })
    });
    let width = (galley.size().x + 24.0).max(60.0);
    let height = 24.0;

    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, text));
    let press = theme::Press::track(ui, &resp);

    if active {
        ui.painter().rect(
            rect,
            CornerRadius::same(3),
            if t.dark() { t.hover } else { FOXIT_PLUM_LIGHT },
            Stroke::new(1.0, FOXIT_PLUM),
            egui::StrokeKind::Inside,
        );
    } else {
        press.wash(ui, rect, 3, false);
    }

    let body = rect.translate(press.offset());
    let icon_color = if active { FOXIT_PLUM } else { t.icon };
    icons::paint(ui, Rect::from_min_size(body.left_top() + vec2(2.0, 5.0), vec2(14.0, 14.0)), icon, 14.0, icon_color);

    let text_pos = egui::pos2(
        body.left() + 21.0,
        body.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, galley, if active { FOXIT_PLUM } else { t.text });

    let resp = theme::hand(resp);
    if tooltip.is_empty() { resp } else { resp.on_hover_text(tooltip) }
}

/// Run a catalogue command by id. A disabled command says why it can't run (the palette's
/// reasoning) instead of doing nothing, and an id the catalogue doesn't know is a bug: it is
/// reported rather than silently swallowed.
fn run(app: &mut PdfCraftApp, id: &str) {
    if pdfcraft_engine::commands::command(id).is_none() {
        log::error!("ribbon button names the unknown command {id}");
        return;
    }
    app.run_command(id);
}

/// The current page's view, when a document is open.
fn view(app: &mut PdfCraftApp) -> Option<&mut crate::DocView> {
    app.active.and_then(|i| app.views.get_mut(i))
}

impl PdfCraftApp {
    /// Set the current page's base layout and continuous scrolling from a ribbon button.
    fn set_ribbon_display(&mut self, layout: crate::canvas::PageLayout, continuous: bool) {
        if let Some(v) = view(self) {
            v.set_display(layout, continuous);
        }
    }
}

/// --- HOME RIBBON ---
fn show_home_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    // Group 1: Tools
    ribbon_group(ui, "Tools", t, |ui| {
        if large_ribbon_button(ui, t, "hand", "Hand", app.quick_tool == QuickTool::Hand, "Hand Tool").clicked() {
            app.quick_tool = QuickTool::Hand;
        }
        if large_ribbon_button(ui, t, "mouse-pointer-2", "Select", app.quick_tool == QuickTool::Select, "Select Tool").clicked() {
            app.quick_tool = QuickTool::Select;
        }
        if large_ribbon_button(ui, t, "zoom-in", "Zoom", app.quick_tool == QuickTool::MarqueeZoom, "Marquee Zoom").clicked() {
            app.quick_tool = QuickTool::MarqueeZoom;
        }
    });

    // Group 2: Create & Edit
    ribbon_group(ui, "Create & Edit", t, |ui| {
        let edit_active = app.mode == Mode::Edit || matches!(app.left, LeftPanel::Tool("edit"));
        if large_ribbon_button(ui, t, "file-pen-line", "Edit Text", edit_active, tl!("Edit a PDF")).clicked() {
            app.select_mode(Mode::Edit);
            app.ribbon_tab = RibbonTab::Edit;
        }

        let combine_active = matches!(app.left, LeftPanel::Tool("combine"));
        if large_ribbon_button(ui, t, "files", "Combine files", combine_active, tl!("Combine files")).clicked() {
            app.select_tool("combine");
        }

        let organize_active = matches!(app.left, LeftPanel::Tool("organize"))
            || app.active.and_then(|i| app.views.get(i)).is_some_and(|v| v.organize);
        if large_ribbon_button(ui, t, "layout-grid", "Organize", organize_active, tl!("Organize pages")).clicked() {
            app.select_tool("organize");
        }

        let comment_active = matches!(app.left, LeftPanel::Tool("comment"));
        if large_ribbon_button(ui, t, "message-square", "Add comments", comment_active, tl!("Add comments")).clicked() {
            app.select_tool("comment");
        }

        if large_ribbon_button(ui, t, "file-down", "Compress a PDF", false, tl!("Compress a PDF")).clicked() {
            run(app, "optimize.reduce");
        }
    });

    // Group 3: Protect (Prominent Unlock PDF button)
    ribbon_group(ui, "Protect", t, |ui| {
        let unlock_active = app.dialog == Some(Dialog::UnlockPdf);
        if unlock_ribbon_button(ui, t, unlock_active, tl!("Unlock PDF - Remove restrictions and passwords")).clicked() {
            unlock_pdf_action(app);
        }

        if large_ribbon_button(ui, t, "shield-check", "Protect", app.dialog == Some(Dialog::Protect), tl!("Password protect")).clicked() {
            run(app, "protect.password");
        }
    });

    // Group 4: Updates & Help
    ribbon_group(ui, "Help & Updates", t, |ui| {
        if large_ribbon_button(ui, t, "cloud", "Check for updates…", false, tl!("Check for updates…")).clicked() {
            app.check_for_updates();
        }
        if large_ribbon_button(ui, t, "info", "About", false, tl!("About MK PDF")).clicked() {
            run(app, "help.about");
        }
    });
}

/// --- EDIT RIBBON (Foxit PhantomPDF Standard 7 Layout) ---
fn show_edit_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    // Group 1: Tools
    ribbon_group(ui, "Tools", t, |ui| {
        if large_ribbon_button(ui, t, "hand", "Hand", app.quick_tool == QuickTool::Hand, "Hand Tool").clicked() {
            app.quick_tool = QuickTool::Hand;
        }
        if large_ribbon_button(ui, t, "mouse-pointer-2", "Select", app.quick_tool == QuickTool::Select, "Select Tool").clicked() {
            app.quick_tool = QuickTool::Select;
        }
        if large_ribbon_button(ui, t, "zoom-in", "Zoom", app.quick_tool == QuickTool::MarqueeZoom, "Zoom").clicked() {
            app.quick_tool = QuickTool::MarqueeZoom;
        }
    });

    // Group 2: Insert
    ribbon_group(ui, "Insert", t, |ui| {
        if large_ribbon_button(ui, t, "image", "Image", false, "Insert Image").clicked() {
            run(app, "edit.image");
        }
        if large_ribbon_button(ui, t, "bookmark", "Bookmark", false, "Add Bookmark").clicked() {
            run(app, "bookmark.add");
        }
        if large_ribbon_button(ui, t, "link-2", "Link", app.quick_tool == QuickTool::Link, "Add Link").clicked() {
            run(app, "edit.link");
        }
        if large_ribbon_button(ui, t, "paperclip", "File Attachment", false, "Attach File").clicked() {
            run(app, "edit.attach");
        }
    });

    // Group 3: Edit Content
    ribbon_group(ui, "Edit Content", t, |ui| {
        let text_active = app.mode == Mode::Edit;
        if large_ribbon_button(ui, t, "text-cursor-input", "Text", text_active, "Edit Text & Paragraphs").clicked() {
            run(app, "edit.edit_text");
        }
        if large_ribbon_button(ui, t, "image-plus", "Image & Object", text_active, "Edit Images and Objects").clicked() {
            run(app, "edit.edit_text");
        }
    });

    // Group 4: Add
    ribbon_group(ui, "Add", t, |ui| {
        if large_ribbon_button(ui, t, "type", "Text Box", false, "Add a text box").clicked() {
            run(app, "comment.freetext");
        }
        if large_ribbon_button(ui, t, "image-plus", "Image", false, "Insert Image").clicked() {
            run(app, "edit.image");
        }
        if large_ribbon_button(ui, t, "signature", "Sign", false, "Fill & Sign").clicked() {
            run(app, "sign.fill.signature");
        }
    });

    // Group 5: Arrange
    ribbon_group(ui, "Arrange", t, |ui| {
        if large_ribbon_button(ui, t, "rotate-ccw", "Rotate Left", false, "Rotate 90° CCW").clicked()
            && let Some(v) = view(app) {
                v.rotate_view(false);
            }
        if large_ribbon_button(ui, t, "rotate-cw", "Rotate Right", false, "Rotate 90° CW").clicked()
            && let Some(v) = view(app) {
                v.rotate_view(true);
            }
        if large_ribbon_button(ui, t, "crop", "Crop", app.quick_tool == QuickTool::Crop, "Crop Page").clicked() {
            run(app, "page.crop");
        }
    });
}

/// --- ORGANIZE RIBBON ---
fn show_organize_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ribbon_group(ui, "Pages", t, |ui| {
        let org_active = matches!(app.left, LeftPanel::Tool("organize"));
        if large_ribbon_button(ui, t, "layout-grid", "Page Grid", org_active, "Organize Page Grid").clicked() {
            run(app, "page.organize");
        }
        if large_ribbon_button(ui, t, "file-plus-2", "Insert", false, "Insert Pages").clicked() {
            run(app, "page.insert");
        }
        if large_ribbon_button(ui, t, "trash-2", "Delete", false, "Delete Pages").clicked() {
            run(app, "page.delete");
        }
        if large_ribbon_button(ui, t, "file-output", "Extract", false, "Extract Pages").clicked() {
            run(app, "page.extract");
        }
    });

    ribbon_group(ui, "Page Marks", t, |ui| {
        if large_ribbon_button(ui, t, "heading", "Header & Footer", false, "Add Header and Footer").clicked() {
            run(app, "edit.header_footer");
        }
        if large_ribbon_button(ui, t, "stamp", "Watermark", false, "Add Watermark").clicked() {
            run(app, "edit.watermark");
        }
        if large_ribbon_button(ui, t, "hash", "Bates Numbers", false, "Bates Numbering").clicked() {
            run(app, "edit.bates");
        }
    });

    ribbon_group(ui, "Tools", t, |ui| {
        if large_ribbon_button(ui, t, "scissors", "Split", false, "Split Document").clicked() {
            run(app, "page.split");
        }
        if large_ribbon_button(ui, t, "crop", "Crop", app.quick_tool == QuickTool::Crop, "Crop Pages").clicked() {
            run(app, "page.crop");
        }
    });
}

/// --- COMMENT RIBBON ---
fn show_comment_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ribbon_group(ui, "Pin & Notes", t, |ui| {
        if large_ribbon_button(ui, t, "sticky-note", "Note", false, "Sticky Note").clicked() {
            run(app, "comment.note");
        }
        if large_ribbon_button(ui, t, "type", "Text Box", false, "Add a text box").clicked() {
            run(app, "comment.freetext");
        }
        if large_ribbon_button(ui, t, "paperclip", "Attachment", false, "File Attachment").clicked() {
            run(app, "comment.attach");
        }
    });

    ribbon_group(ui, "Text Markup", t, |ui| {
        if large_ribbon_button(ui, t, "highlighter", "Highlight", false, "Highlight Text").clicked() {
            run(app, "comment.highlight");
        }
        if large_ribbon_button(ui, t, "underline", "Underline", false, "Underline Text").clicked() {
            run(app, "comment.underline");
        }
        if large_ribbon_button(ui, t, "strikethrough", "Strikeout", false, "Strikeout Text").clicked() {
            run(app, "comment.strikeout");
        }
    });

    ribbon_group(ui, "Drawing", t, |ui| {
        if large_ribbon_button(ui, t, "pencil", "Pencil", false, "Freehand Pencil").clicked() {
            run(app, "comment.ink");
        }
        if large_ribbon_button(ui, t, "eraser", "Eraser", false, "Eraser").clicked() {
            run(app, "comment.eraser");
        }
        if large_ribbon_button(ui, t, "square", "Rectangle", false, "Rectangle").clicked() {
            run(app, "comment.square");
        }
        if large_ribbon_button(ui, t, "circle", "Oval", false, "Oval / Circle").clicked() {
            run(app, "comment.circle");
        }
        if large_ribbon_button(ui, t, "arrow-up-right", "Arrow", false, "Arrow").clicked() {
            run(app, "comment.arrow");
        }
        if large_ribbon_button(ui, t, "stamp", "Stamp", false, "Add a stamp").clicked() {
            run(app, "comment.stamp");
        }
    });

    ribbon_group(ui, "Manage", t, |ui| {
        let comm_active = app.right == Some(RightPanel::Comments);
        if large_ribbon_button(ui, t, "message-square-text", "Comments", comm_active, "View Comments Panel").clicked() {
            run(app, "comment.list");
        }
        if large_ribbon_button(ui, t, "ruler", "Measure", false, "Measure Distance").clicked() {
            run(app, "measure.distance");
        }
    });
}

/// --- VIEW RIBBON ---
fn show_view_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ribbon_group(ui, "Document Views", t, |ui| {
        if large_ribbon_button(ui, t, "minimize-2", "Normal", app.mode != Mode::Read, "Normal View").clicked() {
            app.select_mode(Mode::AllTools);
        }
        if large_ribbon_button(ui, t, "book-open", "Read Mode", app.mode == Mode::Read, "Read Mode").clicked() {
            run(app, "view.read_mode");
        }
        if large_ribbon_button(ui, t, "maximize", "Full Screen", app.full_screen, "Full Screen").clicked() {
            run(app, "view.full_screen");
        }
    });

    ribbon_group(ui, "Page Display", t, |ui| {
        use crate::canvas::PageLayout;
        let fit = app.active.and_then(|i| app.views.get(i)).map(|v| v.fit);
        let (single, continuous, two_up) = app
            .active
            .and_then(|i| app.views.get(i))
            .map_or((true, true, false), |v| (v.layout == PageLayout::Single, v.continuous(), v.layout == PageLayout::TwoUp));
        if large_ribbon_button(ui, t, "file-text", "Single Page", single && !continuous && !two_up, "Single Page Display").clicked() {
            app.set_ribbon_display(PageLayout::Single, false);
        }
        if large_ribbon_button(ui, t, "arrow-up-down", "Continuous", continuous && !two_up, "Continuous Scrolling").clicked() {
            app.set_ribbon_display(PageLayout::Single, true);
        }
        if large_ribbon_button(ui, t, "columns-2", "Two Pages", two_up, "Two Pages Facing").clicked() {
            app.set_ribbon_display(PageLayout::TwoUp, !continuous);
        }
        if large_ribbon_button(ui, t, "move-horizontal", "Fit Width", fit == Some(Fit::Width), "Fit Width").clicked()
            && let Some(v) = view(app) {
                v.set_fit(Fit::Width);
            }
        if large_ribbon_button(ui, t, "maximize-2", "Fit Page", fit == Some(Fit::Page), "Fit Page").clicked()
            && let Some(v) = view(app) {
                v.set_fit(Fit::Page);
            }
    });

    ribbon_group(ui, "Zoom", t, |ui| {
        let (zoom, fit) = app.active.and_then(|i| app.views.get(i)).map_or((1.0, Fit::None), |v| (v.zoom, v.fit));
        if large_ribbon_button(ui, t, "zoom-in", "Zoom In", false, "Zoom In").clicked()
            && let Some(v) = view(app) {
                v.zoom_step(true);
            }
        if large_ribbon_button(ui, t, "zoom-out", "Zoom Out", false, "Zoom Out").clicked()
            && let Some(v) = view(app) {
                v.zoom_step(false);
            }
        if large_ribbon_button(ui, t, "scan", "Actual Size", (zoom - 1.0).abs() < 0.001 && fit == Fit::None, "100% Actual Size").clicked()
            && let Some(v) = view(app) {
                v.set_zoom(1.0);
            }
    });

    ribbon_group(ui, "Panels", t, |ui| {
        let pages_active = app.right == Some(RightPanel::Pages);
        if large_ribbon_button(ui, t, "files", "Thumbnails", pages_active, "Page Thumbnails").clicked() {
            app.right = Some(RightPanel::Pages);
        }
        let bm_active = app.right == Some(RightPanel::Bookmarks);
        if large_ribbon_button(ui, t, "bookmark", "Bookmarks", bm_active, "Bookmarks").clicked() {
            app.right = Some(RightPanel::Bookmarks);
        }
        let lay_active = app.right == Some(RightPanel::Layers);
        if large_ribbon_button(ui, t, "layers", "Layers", lay_active, "Layers").clicked() {
            app.right = Some(RightPanel::Layers);
        }
    });
}

/// --- FORM RIBBON ---
fn show_form_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ribbon_group(ui, "Form Fields", t, |ui| {
        if large_ribbon_button(ui, t, "text-cursor-input", "Text Field", false, "Add Text Field").clicked() {
            run(app, "form.add.text");
        }
        if large_ribbon_button(ui, t, "check-circle-2", "Check Box", false, "Add Check Box").clicked() {
            run(app, "form.add.checkbox");
        }
        if large_ribbon_button(ui, t, "circle-dot", "Radio Button", false, "Add Radio Button").clicked() {
            run(app, "form.add.radio");
        }
        if large_ribbon_button(ui, t, "chevron-down", "Drop-down", false, "Add Combo Box").clicked() {
            run(app, "form.add.combo");
        }
        if large_ribbon_button(ui, t, "list", "List Box", false, "Add List Box").clicked() {
            run(app, "form.add.list");
        }
        if large_ribbon_button(ui, t, "square", "Button", false, "Add Push Button").clicked() {
            run(app, "form.add.button");
        }
        if large_ribbon_button(ui, t, "clock-3", "Date Field", false, "Add Date Field").clicked() {
            run(app, "form.add.date");
        }
        if large_ribbon_button(ui, t, "signature", "Signature", false, "Add Digital Signature Field").clicked() {
            run(app, "form.add.signature");
        }
    });

    ribbon_group(ui, "Form Data", t, |ui| {
        if large_ribbon_button(ui, t, "eraser", "Reset Form", false, "Reset Form Fields").clicked() {
            run(app, "form.clear");
        }
        if large_ribbon_button(ui, t, "list-ordered", "Calculation Order", false, "Calculation Order").clicked() {
            run(app, "form.calculation_order");
        }
        if large_ribbon_button(ui, t, "layers", "Flatten Fields", false, "Flatten form fields").clicked() {
            run(app, "form.flatten");
        }
    });

    ribbon_group(ui, "Data", t, |ui| {
        if large_ribbon_button(ui, t, "file-input", "Import Data…", false, "Import form data").clicked() {
            run(app, "form.import_data");
        }
        if large_ribbon_button(ui, t, "file-output", "Export Data…", false, "Export form data").clicked() {
            run(app, "form.export_data");
        }
        if large_ribbon_button(ui, t, "scan-text", "Detect Fields", false, "Detect form fields").clicked() {
            run(app, "form.detect");
        }
    });
}

/// --- PROTECT RIBBON ---
fn show_protect_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ribbon_group(ui, "Secure Document", t, |ui| {
        let unlock_active = app.dialog == Some(Dialog::UnlockPdf);
        if unlock_ribbon_button(ui, t, unlock_active, tl!("Unlock PDF - Remove all permissions and password restrictions")).clicked() {
            unlock_pdf_action(app);
        }

        if large_ribbon_button(ui, t, "lock", "Password Protect", app.dialog == Some(Dialog::Protect), "Protect with Password").clicked() {
            run(app, "protect.password");
        }
        if large_ribbon_button(ui, t, "lock-open", "Remove Security", false, "Remove Document Security").clicked() {
            run(app, "protect.remove");
        }
        if large_ribbon_button(ui, t, "eye-off", "Sanitize", false, "Sanitize Hidden Data").clicked() {
            run(app, "redact.sanitize");
        }
    });

    ribbon_group(ui, "Redaction", t, |ui| {
        if large_ribbon_button(ui, t, "rectangle-horizontal", "Mark Redaction", app.quick_tool == QuickTool::Redact, "Mark Text/Area for Redaction").clicked() {
            run(app, "redact.mark");
        }
        if large_ribbon_button(ui, t, "check", "Apply Redactions", false, "Permanently Apply Redactions").clicked() {
            run(app, "redact.apply");
        }
    });

    ribbon_group(ui, "Digital Signatures", t, |ui| {
        if large_ribbon_button(ui, t, "signature", "Sign Document", false, "Place Digital Signature").clicked() {
            run(app, "sign.digital");
        }
        if large_ribbon_button(ui, t, "badge-check", "Validate All", false, "Validate All Signatures").clicked() {
            run(app, "sign.validate");
        }
    });
}

/// --- CONVERT RIBBON ---
fn show_convert_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ribbon_group(ui, "Export PDF", t, |ui| {
        if large_ribbon_button(ui, t, "file-text", "To Word", false, "Export to Microsoft Word").clicked() {
            run(app, "export.docx");
        }
        if large_ribbon_button(ui, t, "file-spreadsheet", "To Excel", false, "Export to Microsoft Excel").clicked() {
            run(app, "export.rtf");
        }
        if large_ribbon_button(ui, t, "presentation", "To PowerPoint", false, "Export to PowerPoint").clicked() {
            run(app, "export.pptx");
        }
        if large_ribbon_button(ui, t, "image", "To Image", false, "Export as Images (PNG, JPEG, TIFF)").clicked() {
            run(app, "export.image");
        }
        if large_ribbon_button(ui, t, "type", "To Text", false, "Export as Plain Text").clicked() {
            run(app, "export.text");
        }
        if large_ribbon_button(ui, t, "file-symlink", "To HTML", false, "Export as HTML").clicked() {
            run(app, "export.html");
        }
    });

    ribbon_group(ui, "Create PDF", t, |ui| {
        if large_ribbon_button(ui, t, "file-input", "From File", false, "Create PDF from a file").clicked() {
            run(app, "create.file");
        }
        if large_ribbon_button(ui, t, "files", "From Several Files", false, "Create PDF from several files").clicked() {
            run(app, "create.multiple");
        }
        if large_ribbon_button(ui, t, "image", "From Images", false, "Create PDF from images").clicked() {
            run(app, "create.images");
        }
        if large_ribbon_button(ui, t, "copy-plus", "From Clipboard", false, "Create PDF from the clipboard").clicked() {
            run(app, "create.clipboard");
        }
    });
}

/// --- SIGN RIBBON ---
fn show_sign_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ribbon_group(ui, "E-Sign", t, |ui| {
        let sign_active = app.mode == Mode::Sign;
        if large_ribbon_button(ui, t, "pen-tool", "Fill & Sign", sign_active, "Fill and Sign PDF").clicked() {
            app.select_mode(Mode::Sign);
        }
        if large_ribbon_button(ui, t, "signature", "Sign", false, "Place a signature").clicked() {
            run(app, "sign.fill.signature");
        }
        if large_ribbon_button(ui, t, "type", "Add Initials", false, "Place Initials").clicked() {
            run(app, "sign.fill.initials");
        }
        if large_ribbon_button(ui, t, "check", "Checkmark", false, "Place a checkmark").clicked() {
            run(app, "sign.fill.check");
        }
        if large_ribbon_button(ui, t, "clock-3", "Date", false, "Stamp today's date").clicked() {
            run(app, "sign.fill.date");
        }
    });

    ribbon_group(ui, "Digital ID", t, |ui| {
        if large_ribbon_button(ui, t, "badge-check", "Digitally Sign", false, "Sign with a Digital ID").clicked() {
            run(app, "sign.digital");
        }
        if large_ribbon_button(ui, t, "shield-check", "Certify", false, "Certify with Digital ID").clicked() {
            run(app, "sign.certify");
        }
        if large_ribbon_button(ui, t, "signature", "Signatures Panel", app.right == Some(RightPanel::Signatures), "Signatures panel").clicked() {
            run(app, "sign.panel");
        }
    });
}

/// --- HELP RIBBON ---
fn show_help_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ribbon_group(ui, "Help & Support", t, |ui| {
        if large_ribbon_button(ui, t, "circle-help", "Keyboard Shortcuts", false, "Keyboard shortcuts").clicked() {
            run(app, "help.shortcuts");
        }
        if large_ribbon_button(ui, t, "triangle-alert", "Report an issue", false, tl!("Report an issue")).clicked() {
            run(app, "help.issues");
        }
        if large_ribbon_button(ui, t, "cloud", "Check for updates…", false, tl!("Check for updates…")).clicked() {
            app.check_for_updates();
        }
        if large_ribbon_button(ui, t, "info", "About MK PDF", false, tl!("About MK PDF")).clicked() {
            run(app, "help.about");
        }
    });

    ribbon_group(ui, "Minh Kim Tech", t, |ui| {
        ui.vertical(|ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("MK PDF Standard").font(theme::semibold(12.0)).color(if t.dark() { t.text } else { FOXIT_PLUM }));
            ui.label(egui::RichText::new("Minh Kim Tech").font(theme::medium(11.0)).color(t.text_muted));
            ui.label(egui::RichText::new("Foxit-Style Ribbon").font(theme::regular(10.0)).color(t.text_muted));
        });
    });
}
