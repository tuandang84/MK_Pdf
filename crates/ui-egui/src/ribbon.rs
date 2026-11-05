use egui::{
    vec2, Align, Color32, CornerRadius, Layout, Rect, Sense, Stroke,
};

use super::{
    icons, theme::{self, Tokens}, Dialog, LeftPanel, Mode, PdfCraftApp,
    QuickTool, RightPanel,
};

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

/// Ribbon Tabs Header bar (Plum FILE button + HOME, EDIT, ORGANIZE, COMMENT, VIEW, FORM, PROTECT, HELP, etc.).
pub fn ribbon_header(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());

    egui::Panel::top("ribbon_header")
        .exact_size(24.0)
        .frame(
            egui::Frame::NONE
                .fill(if t.dark() { t.chrome } else { Color32::from_rgb(243, 243, 246) })
                .inner_margin(egui::Margin { left: 8, right: 8, top: 0, bottom: 0 })
                .stroke(Stroke::new(1.0, t.divider)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 1.0;

                // In-window menu bar (File, Edit, View...), if native menu is not present
                if app.native_menu.is_none() {
                    app.menu_bar_ui(ui);
                    ui.add_space(3.0);
                    ui.painter().vline(ui.cursor().left(), ui.max_rect().y_range().shrink(6.0), Stroke::new(1.0, t.divider));
                    ui.add_space(3.0);
                }

                // 1. Mode tabs (All tools, Edit, Convert, E-Sign)
                let aux_tabs = [
                    (tl!("All tools"), app.mode == Mode::AllTools && app.ribbon_tab != RibbonTab::Edit, Mode::AllTools),
                    (tl!("Edit"), app.mode == Mode::Edit || app.ribbon_tab == RibbonTab::Edit, Mode::Edit),
                    (tl!("Convert"), app.mode == Mode::Convert || app.ribbon_tab == RibbonTab::Convert, Mode::Convert),
                    (tl!("E-Sign"), app.mode == Mode::Sign || app.ribbon_tab == RibbonTab::Sign, Mode::Sign),
                ];

                for (label, active, mode) in aux_tabs {
                    let resp = render_mode_tab_button(ui, &t, label, active);
                    if resp.clicked() {
                        app.switch_mode(mode, true);
                        if mode == Mode::Edit {
                            app.ribbon_tab = RibbonTab::Edit;
                        } else if mode == Mode::Convert {
                            app.ribbon_tab = RibbonTab::Convert;
                        } else if mode == Mode::Sign {
                            app.ribbon_tab = RibbonTab::Sign;
                        }
                    }
                }

                ui.add_space(3.0);
                ui.painter().vline(ui.cursor().left(), ui.max_rect().y_range().shrink(6.0), Stroke::new(1.0, t.divider));
                ui.add_space(3.0);

                // 2. Iconic Foxit Plum FILE Button
                let file_active = app.active.is_none();
                let file_btn = render_file_tab_button(ui, file_active);
                if file_btn.clicked() {
                    // Clicking FILE takes user to Home backstage or opens file menu
                    app.active = None;
                    app.combine_tab.focused = false;
                }

                // 3. Ribbon Tabs (HOME, ORGANIZE, COMMENT, VIEW, FORM, PROTECT, HELP)
                let tabs: [(RibbonTab, &str); 7] = [
                    (RibbonTab::Home, "HOME"),
                    (RibbonTab::Organize, "ORGANIZE"),
                    (RibbonTab::Comment, "COMMENT"),
                    (RibbonTab::View, "VIEW"),
                    (RibbonTab::Form, "FORM"),
                    (RibbonTab::Protect, "PROTECT"),
                    (RibbonTab::Help, "HELP"),
                ];

                for (tab, display_label) in tabs {
                    let active = app.ribbon_tab == tab && app.mode != Mode::Edit && app.mode != Mode::Convert && app.mode != Mode::Sign;
                    let resp = render_ribbon_tab_button(ui, &t, display_label, active);
                    if resp.clicked() {
                        app.ribbon_tab = tab;
                        if tab == RibbonTab::Home && app.mode == Mode::Read {
                            app.select_mode(Mode::AllTools);
                        }
                    }
                }

                // 4. Far right: QAT actions (Save, Print, Undo, Redo, Unlock PDF), Search, and Ribbon collapse
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let collapse_icon = if app.ribbon_collapsed { "chevron-down" } else { "chevron-up" };
                    let collapse_tip = if app.ribbon_collapsed { tl!("Expand ribbon") } else { tl!("Collapse ribbon") };
                    if icons::button(ui, collapse_icon, 24.0, false, collapse_tip).clicked() {
                        app.ribbon_collapsed = !app.ribbon_collapsed;
                    }
                    if super::widgets::search_box(ui, tl!("Find"), 110.0).clicked() {
                        app.palette_open = true;
                    }

                    ui.add_space(2.0);
                    ui.painter().vline(ui.cursor().left(), ui.max_rect().y_range().shrink(6.0), Stroke::new(1.0, t.divider));
                    ui.add_space(2.0);

                    // Quick Access Toolbar (rendered right-to-left: Unlock PDF, Redo, Undo, Print, Save)
                    let qat_size = 20.0;
                    let unlock_tip = tl!("Unlock PDF - Remove restrictions and passwords without entering password");
                    if icons::button(ui, "lock-open", qat_size, false, unlock_tip).clicked() {
                        unlock_pdf_action(app);
                    }
                    let redo_tip = crate::commands::command_tip(ui.ctx(), tl!("Redo ({key})"), "app.redo");
                    if icons::button(ui, "redo-2", qat_size, false, &redo_tip).clicked() {
                        app.run_command("app.redo");
                    }
                    let undo_tip = crate::commands::command_tip(ui.ctx(), tl!("Undo ({key})"), "app.undo");
                    if icons::button(ui, "undo-2", qat_size, false, &undo_tip).clicked() {
                        app.run_command("app.undo");
                    }
                    let has_doc = app.active.is_some();
                    let may_print = !has_doc || pdfcraft_engine::commands::command("print.dialog").is_some_and(|c| app.command_enabled(c));
                    let print_tip = crate::commands::command_tip(ui.ctx(), tl!("Print ({key})"), "print.dialog");
                    let mut print = ui.add_enabled_ui(has_doc && may_print, |ui| icons::button(ui, "printer", qat_size, false, &print_tip)).inner;
                    if has_doc && !may_print {
                        print = print.on_disabled_hover_text(crate::print_refused());
                    }
                    if print.clicked() {
                        app.run_command("print.dialog");
                    }
                    let save_tip = crate::commands::command_tip(ui.ctx(), tl!("Save ({key})"), "file.save");
                    if icons::button(ui, "save", qat_size, false, &save_tip).clicked() {
                        app.run_command("file.save");
                    }
                });
            });
        });
}

/// The Plum Foxit "FILE" button.
fn render_file_tab_button(ui: &mut egui::Ui, active: bool) -> egui::Response {
    let height = 24.0;
    let width = 50.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, active, "FILE"));
    let press = theme::Press::track(ui, &resp);

    let bg = press.fill(FOXIT_PLUM, FOXIT_PLUM_HOVER, FOXIT_PLUM_ACTIVE);

    ui.painter().rect_filled(rect, CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 }, bg);

    let text_galley = ui.fonts_mut(|f| f.layout_no_wrap("FILE".to_string(), theme::semibold(11.5), Color32::WHITE));
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
    let font = if active { theme::semibold(11.0) } else { theme::medium(11.0) };
    let text_color = if active {
        if t.dark() { t.accent_text } else { FOXIT_PLUM }
    } else {
        if t.dark() { t.text_muted } else { Color32::from_rgb(70, 70, 75) }
    };

    let galley = ui.fonts_mut(|f| f.layout_no_wrap(display_label.to_string(), font, text_color));
    let width = galley.size().x + 16.0;
    let height = 24.0;

    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let tab_widget_label = format!("{display_label} Tab");
    resp.widget_info(move || egui::WidgetInfo::selected(egui::Role::Tab, true, active, tab_widget_label.clone()));
    let press = theme::Press::track(ui, &resp);

    if active {
        let ribbon_bg = if t.dark() { t.panel } else { Color32::WHITE };
        ui.painter().rect_filled(rect, CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 }, ribbon_bg);
        // Border on top, left, right to seamlessly join with the ribbon strip below
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
    let font = if active { theme::semibold(11.0) } else { theme::medium(11.0) };
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), font, if active { t.text } else { t.text_muted }));
    let width = galley.size().x + 16.0;
    let height = 24.0;

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

/// The Foxit PhantomPDF Grouped Command Strip (compact height ~36px).
pub fn ribbon_command_strip(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());

    egui::Panel::top("ribbon_strip")
        .exact_size(36.0)
        .frame(
            egui::Frame::NONE
                .fill(if t.dark() { t.panel } else { Color32::WHITE })
                .inner_margin(egui::Margin { left: 8, right: 8, top: 1, bottom: 1 })
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
            ui.spacing_mut().item_spacing = vec2(3.0, 0.0);
            add_contents(ui)
        }).inner;
        let end_x = ui.cursor().left();
        let group_w = (end_x - start_x).max(32.0);
        let title_galley = ui.fonts_mut(|f| {
            f.layout_no_wrap(
                tl!(title).to_string(),
                theme::regular(8.5),
                if t.dark() { t.text_muted } else { Color32::from_rgb(125, 125, 135) },
            )
        });
        let label_pos = egui::pos2(
            start_x + (group_w - title_galley.size().x).max(0.0) / 2.0,
            ui.cursor().top(),
        );
        ui.painter().galley(label_pos, title_galley, if t.dark() { t.text_muted } else { Color32::from_rgb(125, 125, 135) });
        r
    }).inner;

    let divider_x = ui.cursor().left() + 4.0;
    let divider_y = ui.max_rect().y_range().shrink(2.0);
    ui.painter().vline(divider_x, divider_y, Stroke::new(1.0, t.divider));
    ui.add_space(6.0);

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
    let font = if active { theme::semibold(9.0) } else { theme::medium(9.0) };
    let text = tl!(label);
    let galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(text.to_string(), font, if active { t.accent_text } else { t.text })
    });
    let width = (galley.size().x + 8.0).max(32.0);
    let height = 23.0;

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
    icons::paint(ui, Rect::from_center_size(body.center_top() + vec2(0.0, 6.0), vec2(12.0, 12.0)), icon, 12.0, icon_color);

    let text_pos = egui::pos2(
        body.center().x - galley.size().x / 2.0,
        body.bottom() - galley.size().y - 1.0,
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
    let font = theme::semibold(9.0);
    let text = tl!("Unlock PDF");
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), font, Color32::WHITE));
    let width = (galley.size().x + 10.0).max(38.0);
    let height = 23.0;

    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::MenuItem, ui.is_enabled(), active, text));
    let press = theme::Press::track(ui, &resp);

    let fill_color = press.fill(FOXIT_PLUM, FOXIT_PLUM_HOVER, FOXIT_PLUM_ACTIVE);

    ui.painter().rect(
        rect,
        CornerRadius::same(3),
        fill_color,
        Stroke::new(1.0, FOXIT_PLUM_HOVER),
        egui::StrokeKind::Inside,
    );

    let body = rect.translate(press.offset());
    icons::paint(ui, Rect::from_center_size(body.center_top() + vec2(0.0, 6.0), vec2(12.0, 12.0)), "lock-open", 12.0, Color32::WHITE);

    let text_pos = egui::pos2(
        body.center().x - galley.size().x / 2.0,
        body.bottom() - galley.size().y - 1.0,
    );
    ui.painter().galley(text_pos, galley, Color32::WHITE);

    let resp = theme::hand(resp);
    if tooltip.is_empty() { resp } else { resp.on_hover_text(tooltip) }
}

/// Compact Ribbon Button (icon on left, label on right, 11px height).
pub fn small_ribbon_button(
    ui: &mut egui::Ui,
    t: &Tokens,
    icon: &str,
    label: &str,
    active: bool,
    tooltip: &str,
) -> egui::Response {
    let font = if active { theme::semibold(9.0) } else { theme::medium(9.0) };
    let text = tl!(label);
    let galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(text.to_string(), font, if active { t.accent_text } else { t.text })
    });
    let width = galley.size().x + 16.0;
    let height = 11.0;

    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, text));
    let press = theme::Press::track(ui, &resp);

    if active {
        ui.painter().rect(
            rect,
            CornerRadius::same(2),
            if t.dark() { t.hover } else { FOXIT_PLUM_LIGHT },
            Stroke::new(1.0, FOXIT_PLUM),
            egui::StrokeKind::Inside,
        );
    } else {
        press.wash(ui, rect, 2, false);
    }

    let body = rect.translate(press.offset());
    let icon_color = if active { FOXIT_PLUM } else { t.icon };
    icons::paint(ui, Rect::from_min_size(body.left_top() + vec2(1.0, 0.5), vec2(10.0, 10.0)), icon, 10.0, icon_color);

    let text_pos = egui::pos2(
        body.left() + 13.0,
        body.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, galley, if active { FOXIT_PLUM } else { t.text });

    let resp = theme::hand(resp);
    if tooltip.is_empty() { resp } else { resp.on_hover_text(tooltip) }
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

    // Group 2: Create & Edit (Contains key tools tested by top_toolbar_renders_key_tools_and_unlock_pdf)
    ribbon_group(ui, "Create & Edit", t, |ui| {
        let edit_active = app.mode == Mode::Edit || matches!(app.left, LeftPanel::Tool("edit"));
        if large_ribbon_button(ui, t, "edit-3", "Edit Text", edit_active, tl!("Edit a PDF")).clicked() {
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

        let compress_active = matches!(app.left, LeftPanel::Tool("compress"));
        if large_ribbon_button(ui, t, "file-archive", "Compress a PDF", compress_active, tl!("Compress a PDF")).clicked() {
            app.select_tool("compress");
        }
    });

    // Group 3: Protect (Prominent Unlock PDF button)
    ribbon_group(ui, "Protect", t, |ui| {
        let unlock_active = app.dialog == Some(Dialog::UnlockPdf);
        if unlock_ribbon_button(ui, t, unlock_active, tl!("Unlock PDF - Remove restrictions and passwords")).clicked() {
            unlock_pdf_action(app);
        }

        if large_ribbon_button(ui, t, "shield", "Protect", app.dialog == Some(Dialog::Protect), tl!("Password protect")).clicked() {
            app.dialog = Some(Dialog::Protect);
        }
    });

    // Group 4: Updates & Help
    ribbon_group(ui, "Help & Updates", t, |ui| {
        if large_ribbon_button(ui, t, "rotate-cw", "Check for updates…", false, tl!("Check for updates…")).clicked() {
            app.check_for_updates();
        }
        if large_ribbon_button(ui, t, "circle-help", "About", false, tl!("About MK PDF")).clicked() {
            app.dialog = Some(Dialog::About);
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
            app.run_command("edit.add_image");
        }
        if large_ribbon_button(ui, t, "bookmark", "Bookmark", false, "Add Bookmark").clicked() {
            app.right = Some(RightPanel::Bookmarks);
        }
        if large_ribbon_button(ui, t, "link", "Link", false, "Add Link").clicked() {
            app.select_tool("link");
        }
        if large_ribbon_button(ui, t, "paperclip", "File Attachment", false, "Attach File").clicked() {
            app.right = Some(RightPanel::Attachments);
        }
    });

    // Group 3: Edit Content
    ribbon_group(ui, "Edit Content", t, |ui| {
        let text_active = app.mode == Mode::Edit;
        if large_ribbon_button(ui, t, "edit-3", "Text & Paragraph", text_active, "Edit Text & Paragraphs").clicked() {
            app.select_mode(Mode::Edit);
        }
        if large_ribbon_button(ui, t, "image", "Image & Object", text_active, "Edit Images and Objects").clicked() {
            app.select_mode(Mode::Edit);
        }
    });

    // Group 4: Font & Paragraph
    ribbon_group(ui, "Font & Paragraph", t, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if small_ribbon_button(ui, t, "bold", "Bold", false, "Bold").clicked() {
                    app.run_command("edit.bold");
                }
                if small_ribbon_button(ui, t, "italic", "Italic", false, "Italic").clicked() {
                    app.run_command("edit.italic");
                }
                if small_ribbon_button(ui, t, "underline", "Underline", false, "Underline").clicked() {
                    app.run_command("edit.underline");
                }
            });
            ui.horizontal(|ui| {
                if small_ribbon_button(ui, t, "align-left", "Left", false, "Align Left").clicked() {
                    app.run_command("edit.align_left");
                }
                if small_ribbon_button(ui, t, "align-center", "Center", false, "Align Center").clicked() {
                    app.run_command("edit.align_center");
                }
                if small_ribbon_button(ui, t, "align-right", "Right", false, "Align Right").clicked() {
                    app.run_command("edit.align_right");
                }
            });
        });
    });

    // Group 5: Arrange
    ribbon_group(ui, "Arrange", t, |ui| {
        if large_ribbon_button(ui, t, "rotate-ccw", "Rotate Left", false, "Rotate 90° CCW").clicked() {
            app.run_command("page.rotate_ccw");
        }
        if large_ribbon_button(ui, t, "rotate-cw", "Rotate Right", false, "Rotate 90° CW").clicked() {
            app.run_command("page.rotate_cw");
        }
        if large_ribbon_button(ui, t, "crop", "Crop", false, "Crop Page").clicked() {
            app.select_tool("crop");
        }
    });
}

/// --- ORGANIZE RIBBON ---
fn show_organize_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ribbon_group(ui, "Pages", t, |ui| {
        let org_active = matches!(app.left, LeftPanel::Tool("organize"));
        if large_ribbon_button(ui, t, "layout-grid", "Page Grid", org_active, "Organize Page Grid").clicked() {
            app.select_tool("organize");
        }
        if large_ribbon_button(ui, t, "file-plus-2", "Insert", false, "Insert Pages").clicked() {
            app.run_command("page.insert");
        }
        if large_ribbon_button(ui, t, "trash-2", "Delete", false, "Delete Pages").clicked() {
            app.run_command("page.delete");
        }
        if large_ribbon_button(ui, t, "file-output", "Extract", false, "Extract Pages").clicked() {
            app.run_command("page.extract");
        }
    });

    ribbon_group(ui, "Page Marks", t, |ui| {
        if large_ribbon_button(ui, t, "heading", "Header & Footer", false, "Add Header and Footer").clicked() {
            app.run_command("page.header_footer");
        }
        if large_ribbon_button(ui, t, "stamp", "Watermark", false, "Add Watermark").clicked() {
            app.run_command("page.watermark");
        }
        if large_ribbon_button(ui, t, "hash", "Bates Numbers", false, "Bates Numbering").clicked() {
            app.run_command("page.bates_numbering");
        }
    });

    ribbon_group(ui, "Tools", t, |ui| {
        if large_ribbon_button(ui, t, "split", "Split", false, "Split Document").clicked() {
            app.select_tool("split");
        }
        if large_ribbon_button(ui, t, "crop", "Crop", false, "Crop Pages").clicked() {
            app.select_tool("crop");
        }
    });
}

/// --- COMMENT RIBBON ---
fn show_comment_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ribbon_group(ui, "Pin & Notes", t, |ui| {
        if large_ribbon_button(ui, t, "message-square", "Note", false, "Sticky Note").clicked() {
            app.run_command("annot.note");
        }
        if large_ribbon_button(ui, t, "paperclip", "Attachment", false, "File Attachment").clicked() {
            app.run_command("annot.file_attachment");
        }
    });

    ribbon_group(ui, "Text Markup", t, |ui| {
        if large_ribbon_button(ui, t, "highlighter", "Highlight", false, "Highlight Text").clicked() {
            app.run_command("annot.highlight");
        }
        if large_ribbon_button(ui, t, "underline", "Underline", false, "Underline Text").clicked() {
            app.run_command("annot.underline");
        }
        if large_ribbon_button(ui, t, "strikethrough", "Strikeout", false, "Strikeout Text").clicked() {
            app.run_command("annot.strikeout");
        }
    });

    ribbon_group(ui, "Drawing", t, |ui| {
        if large_ribbon_button(ui, t, "pencil", "Pencil", false, "Freehand Pencil").clicked() {
            app.run_command("annot.ink");
        }
        if large_ribbon_button(ui, t, "eraser", "Eraser", false, "Eraser").clicked() {
            app.run_command("annot.eraser");
        }
        if large_ribbon_button(ui, t, "square", "Rectangle", false, "Rectangle").clicked() {
            app.run_command("annot.square");
        }
        if large_ribbon_button(ui, t, "circle", "Oval", false, "Oval / Circle").clicked() {
            app.run_command("annot.circle");
        }
    });

    ribbon_group(ui, "Manage", t, |ui| {
        let comm_active = app.right == Some(RightPanel::Comments);
        if large_ribbon_button(ui, t, "message-square-text", "Comments", comm_active, "View Comments Panel").clicked() {
            app.right = Some(RightPanel::Comments);
        }
        if large_ribbon_button(ui, t, "ruler", "Measure", false, "Measure Distance").clicked() {
            app.run_command("measure.distance");
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
            app.run_command("view.read_mode");
        }
        if large_ribbon_button(ui, t, "maximize-2", "Full Screen", false, "Full Screen").clicked() {
            app.run_command("view.full_screen");
        }
    });

    ribbon_group(ui, "Page Display", t, |ui| {
        if large_ribbon_button(ui, t, "file", "Single Page", false, "Single Page Display").clicked() {
            app.run_command("view.page_single");
        }
        if large_ribbon_button(ui, t, "scroll", "Continuous", false, "Continuous Scrolling").clicked() {
            app.run_command("view.page_continuous");
        }
        if large_ribbon_button(ui, t, "columns-2", "Two Pages", false, "Two Pages Facing").clicked() {
            app.run_command("view.page_facing");
        }
    });

    ribbon_group(ui, "Zoom", t, |ui| {
        if large_ribbon_button(ui, t, "zoom-in", "Zoom In", false, "Zoom In").clicked() {
            app.run_command("view.zoom_in");
        }
        if large_ribbon_button(ui, t, "zoom-out", "Zoom Out", false, "Zoom Out").clicked() {
            app.run_command("view.zoom_out");
        }
        if large_ribbon_button(ui, t, "move-horizontal", "Fit Width", false, "Fit Width").clicked() {
            app.run_command("view.fit_width");
        }
        if large_ribbon_button(ui, t, "maximize", "Fit Page", false, "Fit Page").clicked() {
            app.run_command("view.fit_page");
        }
        if large_ribbon_button(ui, t, "scan", "Actual Size", false, "100% Actual Size").clicked() {
            app.run_command("view.actual_size");
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
            app.run_command("form.text_field");
        }
        if large_ribbon_button(ui, t, "check-square", "Check Box", false, "Add Check Box").clicked() {
            app.run_command("form.checkbox");
        }
        if large_ribbon_button(ui, t, "circle-dot", "Radio Button", false, "Add Radio Button").clicked() {
            app.run_command("form.radio");
        }
        if large_ribbon_button(ui, t, "list", "Combo Box", false, "Add Combo Box").clicked() {
            app.run_command("form.combo_box");
        }
        if large_ribbon_button(ui, t, "square-play", "Push Button", false, "Add Push Button").clicked() {
            app.run_command("form.button");
        }
        if large_ribbon_button(ui, t, "pen-tool", "Signature", false, "Add Digital Signature Field").clicked() {
            app.run_command("form.signature_field");
        }
    });

    ribbon_group(ui, "Form Data", t, |ui| {
        if large_ribbon_button(ui, t, "rotate-ccw", "Reset Form", false, "Reset Form Fields").clicked() {
            app.run_command("form.reset");
        }
        if large_ribbon_button(ui, t, "calculator", "Calculation Order", false, "Calculation Order").clicked() {
            app.left = LeftPanel::CalculationOrder;
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
            app.dialog = Some(Dialog::Protect);
        }
        if large_ribbon_button(ui, t, "unlock", "Remove Security", false, "Remove Document Security").clicked() {
            unlock_pdf_action(app);
        }
        if large_ribbon_button(ui, t, "shield-alert", "Sanitize", false, "Sanitize Hidden Data").clicked() {
            app.run_command("protect.sanitize");
        }
    });

    ribbon_group(ui, "Redaction", t, |ui| {
        if large_ribbon_button(ui, t, "eraser", "Mark Redaction", false, "Mark Text/Area for Redaction").clicked() {
            app.select_tool("redact");
        }
        if large_ribbon_button(ui, t, "check-check", "Apply Redactions", false, "Permanently Apply Redactions").clicked() {
            app.run_command("redact.apply");
        }
    });

    ribbon_group(ui, "Digital Signatures", t, |ui| {
        if large_ribbon_button(ui, t, "pen-tool", "Sign Document", false, "Place Digital Signature").clicked() {
            app.select_mode(Mode::Sign);
        }
        if large_ribbon_button(ui, t, "shield-check", "Validate All", false, "Validate All Signatures").clicked() {
            app.run_command("sign.validate");
        }
    });
}

/// --- CONVERT RIBBON ---
fn show_convert_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ribbon_group(ui, "Export PDF", t, |ui| {
        if large_ribbon_button(ui, t, "file-text", "To Word", false, "Export to Microsoft Word").clicked() {
            app.select_tool("export");
        }
        if large_ribbon_button(ui, t, "file-spreadsheet", "To Excel", false, "Export to Microsoft Excel").clicked() {
            app.select_tool("export");
        }
        if large_ribbon_button(ui, t, "presentation", "To PowerPoint", false, "Export to PowerPoint").clicked() {
            app.select_tool("export");
        }
        if large_ribbon_button(ui, t, "image", "To Image", false, "Export as Images (PNG, JPEG, TIFF)").clicked() {
            app.select_tool("export");
        }
        if large_ribbon_button(ui, t, "file-code", "To Text", false, "Export as Plain Text or HTML").clicked() {
            app.select_tool("export");
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
        if large_ribbon_button(ui, t, "type", "Add Initials", false, "Place Initials").clicked() {
            app.run_command("sign.initials");
        }
        if large_ribbon_button(ui, t, "shield-check", "Certify", false, "Certify with Digital ID").clicked() {
            app.run_command("sign.certify");
        }
    });
}

/// --- HELP RIBBON ---
fn show_help_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ribbon_group(ui, "Help & Support", t, |ui| {
        if large_ribbon_button(ui, t, "circle-help", "User Guide", false, "Open User Guide").clicked() {
            app.dialog = Some(Dialog::About);
        }
        if large_ribbon_button(ui, t, "rotate-cw", "Check for updates…", false, tl!("Check for updates…")).clicked() {
            app.check_for_updates();
        }
        if large_ribbon_button(ui, t, "info", "About MK PDF", false, tl!("About MK PDF")).clicked() {
            app.dialog = Some(Dialog::About);
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
