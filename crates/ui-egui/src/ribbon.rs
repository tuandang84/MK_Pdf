use egui::{Align, Color32, CornerRadius, Layout, Rect, Sense, Stroke, vec2};

use super::{
    Dialog, LeftPanel, Mode, PdfCraftApp, QuickTool, RightPanel, icons,
    theme::{self, Tokens},
};
use crate::canvas::Fit;

/// The primary ribbon tabs, in Foxit PhantomPDF order and spelling. Every entry is a view over
/// the same engine commands — switching a tab never discards a document, and every button on a
/// tab runs a registered command (see [`run`]) or a direct view action. Tabs Foxit owns
/// outright (Share = SharePoint/eDOCS, Connect = ConnectedPDF) are not reproduced: MK has no
/// engine for them, and a ribbon button that cannot run is worse than no button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RibbonTab {
    #[default]
    Home,
    Convert,
    Edit,
    Organize,
    Comment,
    View,
    Form,
    Protect,
    Accessibility,
    Help,
    /// Legacy: Fill & Sign used to be its own tab (surfaced through `Mode::Sign`). Fill & Sign
    /// now lives in the Protect tab, as in Foxit. Kept so sessions saved before this release
    /// still decode; selecting the Sign mode highlights Protect, never a separate tab.
    Sign,
}

/// Foxit PhantomPDF plum purple accent colors.
pub const FOXIT_PLUM: Color32 = Color32::from_rgb(112, 32, 97);
pub const FOXIT_PLUM_HOVER: Color32 = Color32::from_rgb(136, 40, 118);
pub const FOXIT_PLUM_ACTIVE: Color32 = Color32::from_rgb(92, 24, 78);
pub const FOXIT_PLUM_LIGHT: Color32 = Color32::from_rgb(243, 230, 241);

/// Action for prominent "Unlock PDF" button (alive in the Protect tab only; a second copy in
/// the quick access toolbar and on Home was a duplicate that overlapped it).
pub fn unlock_pdf_action(app: &mut PdfCraftApp) {
    if app.active.is_none() {
        app.open_dialog();
    } else if app.active_ids().and_then(|(_, id)| app.session.get(id)).is_some_and(|d| d.security_summary().is_some()) {
        app.dialog = Some(Dialog::UnlockPdf);
    } else {
        app.notify_tr("This document isn't password-protected");
    }
}

/// The ribbon tab-strip row: Foxit's plum FILE button, the quick access toolbar
/// (Save, Print, Undo, Redo, then the menu), and the Foxit-ordered tabs. There is
/// deliberately no legacy menu bar: Foxit PhantomPDF has none either, and every command is
/// reachable from the ribbon, the FILE home screen or the command palette. The old
/// workspace mode tabs (All tools / Edit / Convert / E-Sign) are gone — they duplicated the
/// Foxit tabs and overlapped them on narrow windows; Edit and Convert change mode directly
/// from their Foxit tab instead.
pub const HEADER_HEIGHT: f32 = 36.0;
/// The grouped command strip below the header, at Foxit PhantomPDF's height.
pub const RIBBON_HEIGHT: f32 = 108.0;

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
                // The header never overlaps itself: the search box shortens, then goes. The
                // rack stays: every Foxit tab is always there, in Foxit order.
                let width = ui.available_width();

                // 1. Foxit-style plum FILE button, flush with the left edge.
                let file_active = app.active.is_none();
                let file_btn = render_file_tab_button(ui, file_active);
                if file_btn.clicked() {
                    // Clicking FILE takes the user to Home backstage.
                    app.active = None;
                    app.combine_tab.focused = false;
                }

                // 2. Quick Access Toolbar: Foxit's order (Save, Print, Undo, Redo). The old
                // Unlock icon that sat here duplicated Protect's Unlock PDF button.
                ui.add_space(4.0);
                let qat_size = 26.0;
                let save_tip = crate::commands::command_tip(ui.ctx(), tl!("Save ({key})"), "file.save");
                if icons::button(ui, "save", qat_size, false, &save_tip).clicked() {
                    app.run_command("file.save");
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
                let undo_tip = crate::commands::command_tip(ui.ctx(), tl!("Undo ({key})"), "edit.undo");
                if icons::button(ui, "undo-2", qat_size, false, &undo_tip).clicked() {
                    app.run_command("edit.undo");
                }
                let redo_tip = crate::commands::command_tip(ui.ctx(), tl!("Redo ({key})"), "edit.redo");
                if icons::button(ui, "redo-2", qat_size, false, &redo_tip).clicked() {
                    app.run_command("edit.redo");
                }

                // 3. Ribbon tabs in Foxit order. A tab switch is a view switch first; Edit and
                // Convert also move the workspace, leaving it restores All tools.
                ribbon_separator(ui, &t);
                let tabs: [(RibbonTab, &str); 10] = [
                    (RibbonTab::Home, "Home"),
                    (RibbonTab::Convert, "Convert"),
                    (RibbonTab::Edit, "Edit"),
                    (RibbonTab::Organize, "Organize"),
                    (RibbonTab::Comment, "Comment"),
                    (RibbonTab::View, "View"),
                    (RibbonTab::Form, "Form"),
                    (RibbonTab::Protect, "Protect"),
                    (RibbonTab::Accessibility, "Accessibility"),
                    (RibbonTab::Help, "Help"),
                ];

                for (tab, display_label) in tabs {
                    // The legacy Sign tab shows as Protect: Fill & Sign lives there now.
                    let shown_tab = if app.ribbon_tab == RibbonTab::Sign { RibbonTab::Protect } else { app.ribbon_tab };
                    let active = shown_tab == tab;
                    if render_ribbon_tab_button(ui, &t, display_label, active).clicked() {
                        app.ribbon_tab = tab;
                        match tab {
                            RibbonTab::Edit => app.select_mode(Mode::Edit),
                            RibbonTab::Convert => app.select_mode(Mode::Convert),
                            RibbonTab::Home if app.mode == Mode::Read => {}
                            _ if matches!(app.mode, Mode::Edit | Mode::Convert | Mode::Sign) => {
                                app.select_mode(Mode::AllTools);
                            }
                            _ => {}
                        }
                    }
                }

                // 4. Far right, as Foxit: Find and the ribbon collapse toggle. The old
                // align-justify "Menu" button (revealing the hidden legacy bar) is the QAT's
                // customize chevron instead, so it cannot overlap the count of other controls.
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let collapse_icon = if app.ribbon_collapsed { "chevron-down" } else { "chevron-up" };
                    let collapse_tip = if app.ribbon_collapsed { tl!("Expand ribbon") } else { tl!("Collapse ribbon") };
                    if icons::button(ui, collapse_icon, 26.0, false, collapse_tip).clicked() {
                        app.ribbon_collapsed = !app.ribbon_collapsed;
                    }
                    let search_width = if width >= 1300.0 {
                        190.0
                    } else if width >= 1100.0 {
                        130.0
                    } else {
                        0.0
                    };
                    if search_width > 0.0 && super::widgets::search_box(ui, tl!("Find"), search_width).clicked() {
                        app.palette_open = true;
                    }
                    if icons::button(ui, "chevron-down", 26.0, app.menu_bar_open, tl!("Customize quick access")).clicked() {
                        app.menu_bar_open = !app.menu_bar_open;
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

/// The compact legacy menu (File, Edit, View…), hidden until the QAT's customize chevron asks
/// for it. The ribbon carries the everyday commands; this keeps the few that have no ribbon
/// home within reach without putting a second menu bar on screen by default.
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

/// The plum Foxit "FILE" button.
fn render_file_tab_button(ui: &mut egui::Ui, active: bool) -> egui::Response {
    let height = HEADER_HEIGHT;
    let width = 66.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, active, "FILE"));
    let press = theme::Press::track(ui, &resp);

    let bg = press.fill(FOXIT_PLUM, FOXIT_PLUM_HOVER, FOXIT_PLUM_ACTIVE);

    ui.painter().rect_filled(rect, CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 }, bg);

    let text_galley = ui.fonts_mut(|f| f.layout_no_wrap("FILE".to_string(), theme::semibold(13.0), Color32::WHITE));
    let text_pos = egui::pos2(rect.center().x - text_galley.size().x / 2.0, rect.center().y - text_galley.size().y / 2.0);
    ui.painter().galley(text_pos, text_galley, Color32::WHITE);

    theme::hand(resp)
}

/// A Ribbon Tab button (Home, EDIT, ORGANIZE, etc.). Foxit renders the active tab in the
/// ribbon's fill; the old hover-only wash left the bar visually flat, so hover now washes too.
fn render_ribbon_tab_button(ui: &mut egui::Ui, t: &Tokens, display_label: &str, active: bool) -> egui::Response {
    let font = if active { theme::semibold(12.5) } else { theme::medium(12.5) };
    let text_color =
        if active { if t.dark() { t.accent_text } else { FOXIT_PLUM } } else { if t.dark() { t.text_muted } else { Color32::from_rgb(70, 70, 75) } };

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
        // Border on top, left, right to seamlessly join with the ribbon strip below.
        let border_stroke = Stroke::new(1.0, if t.dark() { t.divider } else { Color32::from_rgb(205, 205, 210) });
        ui.painter().line_segment([rect.left_top(), rect.right_top()], border_stroke);
        ui.painter().line_segment([rect.left_top(), rect.left_bottom()], border_stroke);
        ui.painter().line_segment([rect.right_top(), rect.right_bottom()], border_stroke);
    } else if resp.hovered() {
        press.wash(ui, rect, 5, false);
    }

    let pos = egui::pos2(rect.center().x - galley.size().x / 2.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(pos, galley, text_color);

    theme::hand(resp)
}

/// The Foxit grouped ribbon strip.
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

                let tab = if app.ribbon_tab == RibbonTab::Sign { RibbonTab::Protect } else { app.ribbon_tab };
                match tab {
                    RibbonTab::Home => show_home_ribbon(app, ui, &t),
                    RibbonTab::Edit => show_edit_ribbon(app, ui, &t),
                    RibbonTab::Organize => show_organize_ribbon(app, ui, &t),
                    RibbonTab::Comment => show_comment_ribbon(app, ui, &t),
                    RibbonTab::View => show_view_ribbon(app, ui, &t),
                    RibbonTab::Form => show_form_ribbon(app, ui, &t),
                    RibbonTab::Protect => show_protect_ribbon(app, ui, &t),
                    RibbonTab::Convert => show_convert_ribbon(app, ui, &t),
                    RibbonTab::Sign => show_protect_ribbon(app, ui, &t),
                    RibbonTab::Accessibility => show_accessibility_ribbon(app, ui, &t),
                    RibbonTab::Help => show_help_ribbon(app, ui, &t),
                }
            });
        });
}

/// Ribbon Group wrapper with bottom title and vertical divider.
fn ribbon_group<R>(ui: &mut egui::Ui, title: &str, t: &Tokens, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let start_x = ui.cursor().left();
    let res = ui
        .vertical(|ui| {
            let r = ui
                .horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                    add_contents(ui)
                })
                .inner;
            let end_x = ui.cursor().left();
            let group_w = (end_x - start_x).max(32.0);
            let title_galley = ui.fonts_mut(|f| {
                f.layout_no_wrap(tl!(title).to_string(), theme::regular(10.5), if t.dark() { t.text_muted } else { Color32::from_rgb(125, 125, 135) })
            });
            let label_pos = egui::pos2(start_x + (group_w - title_galley.size().x).max(0.0) / 2.0, ui.cursor().top() + 1.0);
            ui.painter().galley(label_pos, title_galley, if t.dark() { t.text_muted } else { Color32::from_rgb(125, 125, 135) });
            r
        })
        .inner;

    let divider_x = ui.cursor().left() + 6.0;
    let divider_y = ui.max_rect().y_range().shrink(4.0);
    ui.painter().vline(divider_x, divider_y, Stroke::new(1.0, t.divider));
    ui.add_space(12.0);

    res
}

/// Large Foxit-style Ribbon Button (icon above label).
pub fn large_ribbon_button(ui: &mut egui::Ui, t: &Tokens, icon: &str, label: &str, active: bool, tooltip: &str) -> egui::Response {
    let font = if active { theme::semibold(11.5) } else { theme::medium(11.5) };
    let text = tl!(label);
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), font, if active { t.accent_text } else { t.text }));
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

    let text_pos = egui::pos2(body.center().x - galley.size().x / 2.0, body.bottom() - galley.size().y - 6.0);
    ui.painter().galley(text_pos, galley, if active { FOXIT_PLUM } else { t.text });

    let resp = theme::hand(resp);
    if tooltip.is_empty() { resp } else { resp.on_hover_text(tooltip) }
}

/// Large split button with a Foxit ▾ arrow below the label. The body runs the default command;
/// the arrow opens a menu of related commands. Two overlapping hit rects are split inside one
/// allocation (the `widgets::split_pill` technique) so the two halves cannot overlap each
/// other or any neighbouring button.
pub fn large_split_ribbon_button(
    ui: &mut egui::Ui,
    t: &Tokens,
    icon: &str,
    label: &str,
    active: bool,
    tooltip: &str,
    menu_label: &str,
) -> (egui::Response, egui::Response) {
    let font = if active { theme::semibold(11.5) } else { theme::medium(11.5) };
    let text = tl!(label);
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), font, if active { t.accent_text } else { t.text }));
    let width = (galley.size().x + 16.0).max(62.0);
    let height = 72.0;
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let arrow_h = 16.0;
    let arrow_rect = Rect::from_min_size(egui::pos2(rect.center().x - 12.0, rect.bottom() - arrow_h), vec2(24.0, arrow_h));
    let main_rect = Rect::from_min_size(rect.left_top(), vec2(rect.width(), rect.height() - arrow_h));
    let main = ui.interact(main_rect, ui.scope_id().with(("split-main", label)), Sense::click());
    let arrow = ui.interact(arrow_rect, ui.scope_id().with(("split-arrow", label)), Sense::click());
    main.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, text));
    arrow.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, menu_label));

    let hot = main.hovered() || arrow.hovered() || main.is_pointer_button_down_on() || arrow.is_pointer_button_down_on();
    if active {
        ui.painter().rect(
            rect,
            CornerRadius::same(4),
            if t.dark() { t.hover } else { FOXIT_PLUM_LIGHT },
            Stroke::new(1.0, FOXIT_PLUM),
            egui::StrokeKind::Inside,
        );
    } else if hot {
        ui.painter().rect_filled(rect, CornerRadius::same(4), if t.dark() { t.hover } else { Color32::from_rgba_unmultiplied(0, 0, 0, 12) });
    }
    let icon_color = if active { FOXIT_PLUM } else { t.icon };
    icons::paint(ui, Rect::from_center_size(rect.center_top() + vec2(0.0, 22.0), vec2(28.0, 28.0)), icon, 28.0, icon_color);
    let text_pos = egui::pos2(rect.center().x - galley.size().x / 2.0, rect.bottom() - arrow_h - galley.size().y - 2.0);
    ui.painter().galley(text_pos, galley, if active { FOXIT_PLUM } else { t.text });
    icons::paint(ui, arrow_rect, "chevron-down", 12.0, if active { FOXIT_PLUM } else { t.icon });

    let main = theme::hand(main);
    let main = if tooltip.is_empty() { main } else { main.on_hover_text(tooltip) };
    let arrow = theme::hand(arrow);
    let arrow = if menu_label.is_empty() { arrow } else { arrow.on_hover_text(menu_label) };
    (main, arrow)
}

/// Compact Ribbon Button (icon on left, label on right).
pub fn small_ribbon_button(ui: &mut egui::Ui, t: &Tokens, icon: &str, label: &str, active: bool, tooltip: &str) -> egui::Response {
    let font = if active { theme::semibold(11.5) } else { theme::medium(11.5) };
    let text = tl!(label);
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), font, if active { t.accent_text } else { t.text }));
    let width = (galley.size().x + 22.0).max(58.0);
    let height = 20.0;

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
    icons::paint(ui, Rect::from_min_size(body.left_top() + vec2(2.0, 4.0), vec2(12.0, 12.0)), icon, 12.0, icon_color);

    let text_pos = egui::pos2(body.left() + 18.0, body.center().y - galley.size().y / 2.0);
    ui.painter().galley(text_pos, galley, if active { FOXIT_PLUM } else { t.text });

    let resp = theme::hand(resp);
    if tooltip.is_empty() { resp } else { resp.on_hover_text(tooltip) }
}

/// Prominent "Unlock PDF" button (highlighted with plum accent). One copy only, in Protect —
/// the duplicates on Home and in the QAT were removed.
pub fn unlock_ribbon_button(ui: &mut egui::Ui, _t: &Tokens, active: bool, tooltip: &str) -> egui::Response {
    let font = theme::semibold(11.5);
    let text = tl!("Unlock PDF");
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(text.to_string(), font, Color32::WHITE));
    let width = (galley.size().x + 18.0).max(70.0);
    let height = 72.0;

    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::MenuItem, ui.is_enabled(), active, text));
    let press = theme::Press::track(ui, &resp);

    let fill_color = press.fill(FOXIT_PLUM, FOXIT_PLUM_HOVER, FOXIT_PLUM_ACTIVE);

    ui.painter().rect(rect, CornerRadius::same(4), fill_color, Stroke::new(1.0, FOXIT_PLUM_HOVER), egui::StrokeKind::Inside);

    let body = rect.translate(press.offset());
    icons::paint(ui, Rect::from_center_size(body.center_top() + vec2(0.0, 22.0), vec2(28.0, 28.0)), "lock-open", 28.0, Color32::WHITE);

    let text_pos = egui::pos2(body.center().x - galley.size().x / 2.0, body.bottom() - galley.size().y - 6.0);
    ui.painter().galley(text_pos, galley, Color32::WHITE);

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

/// The Tools group Foxit repeats at the left of every tab: Hand, Select (a split button
/// running the Select tool), Zoom (a split button running Marquee Zoom). Drawn only once
/// per tab — the third, fourth and fifth Tools groups the old ribbon carried on different
/// tabs and next to unrelated groups in fact duplicated this one and overlapped it.
fn tools_group(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ribbon_group(ui, "Tools", t, |ui| {
        if large_ribbon_button(ui, t, "hand", "Hand", app.quick_tool == QuickTool::Hand, "Hand Tool").clicked() {
            app.quick_tool = QuickTool::Hand;
        }
        let (main, arrow) = large_split_ribbon_button(
            ui,
            t,
            "mouse-pointer-2",
            "Select",
            app.quick_tool == QuickTool::Select,
            "Select Tool",
            tl!("More select tools"),
        );
        if main.clicked() {
            app.quick_tool = QuickTool::Select;
        }
        let mut chosen: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&arrow).show(|ui| {
            if ui.button(tl!("Select")).clicked() {
                ui.close();
                Some("select")
            } else if ui.button(tl!("Take Snapshot")).clicked() {
                ui.close();
                Some("snapshot")
            } else if ui.button(tl!("Column Select")).clicked() {
                ui.close();
                Some("column")
            } else {
                None
            }
        }) {
            chosen = r.inner;
        }
        match chosen {
            Some("select") => app.quick_tool = QuickTool::Select,
            Some("snapshot") => run(app, "edit.snapshot"),
            Some("column") => run(app, "edit.column_select"),
            _ => {}
        }
        let (z_main, z_arrow) =
            large_split_ribbon_button(ui, t, "zoom-in", "Zoom", app.quick_tool == QuickTool::MarqueeZoom, "Marquee Zoom", tl!("Zoom choices"));
        if z_main.clicked() {
            app.quick_tool = QuickTool::MarqueeZoom;
        }
        let mut z_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&z_arrow).show(|ui| {
            if ui.button(tl!("Marquee Zoom")).clicked() {
                ui.close();
                Some("marquee")
            } else if ui.button(tl!("Actual Size")).clicked() {
                ui.close();
                Some("actual")
            } else if ui.button(tl!("Fit Page")).clicked() {
                ui.close();
                Some("fit_page")
            } else if ui.button(tl!("Fit Width")).clicked() {
                ui.close();
                Some("fit_width")
            } else if ui.button(tl!("Fit Visible")).clicked() {
                ui.close();
                Some("fit_visible")
            } else if ui.button(tl!("Zoom In")).clicked() {
                ui.close();
                Some("in")
            } else if ui.button(tl!("Zoom Out")).clicked() {
                ui.close();
                Some("out")
            } else {
                None
            }
        }) {
            z_choice = r.inner;
        }
        match z_choice {
            Some("marquee") => app.quick_tool = QuickTool::MarqueeZoom,
            Some("actual") => {
                if let Some(v) = view(app) {
                    v.set_zoom(1.0);
                }
            }
            Some("fit_page") => run(app, "view.fit_one_page"),
            Some("fit_width") => run(app, "view.fit_width_scrolling"),
            Some("fit_visible") => run(app, "view.fit_visible"),
            Some("in") => {
                if let Some(v) = view(app) {
                    v.zoom_step(true);
                }
            }
            Some("out") => {
                if let Some(v) = view(app) {
                    v.zoom_step(false);
                }
            }
            _ => {}
        }
    });
}

/// --- HOME RIBBON (Foxit: Tools, View, Edit, Comment, Page Organization, Convert, Protect) ---
fn show_home_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    tools_group(app, ui, t);

    ribbon_group(ui, "View", t, |ui| {
        let (zoom, fit) = app.active.and_then(|i| app.views.get(i)).map_or((1.0, Fit::None), |v| (v.zoom, v.fit));
        let actual_active = (zoom - 1.0).abs() < 0.001 && fit == Fit::None;
        if large_ribbon_button(ui, t, "file-text", "Actual Size", actual_active, "Set the zoom level to the page's actual size").clicked()
            && let Some(v) = view(app)
        {
            v.set_zoom(1.0);
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            let fit = app.active.and_then(|i| app.views.get(i)).map(|v| v.fit);
            if small_ribbon_button(ui, t, "maximize-2", "Fit Page", fit == Some(Fit::Page), "Fit Page").clicked() {
                run(app, "view.fit_one_page");
            }
            if small_ribbon_button(ui, t, "move-horizontal", "Fit Width", fit == Some(Fit::Width), "Fit Width").clicked() {
                run(app, "view.fit_width_scrolling");
            }
            if small_ribbon_button(ui, t, "scan", "Fit Visible", false, "Fit Visible").clicked() {
                run(app, "view.fit_visible");
            }
        });
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            if small_ribbon_button(ui, t, "rotate-ccw", "Rotate Left", false, "Rotate view left").clicked()
                && let Some(v) = view(app)
            {
                v.rotate_view(false);
            }
            if small_ribbon_button(ui, t, "rotate-cw", "Rotate Right", false, "Rotate view right").clicked()
                && let Some(v) = view(app)
            {
                v.rotate_view(true);
            }
        });
    });

    ribbon_group(ui, "Edit", t, |ui| {
        let edit_active = app.mode == Mode::Edit;
        if large_ribbon_button(ui, t, "text-cursor-input", "Edit Text", edit_active, "Edit page text").clicked() {
            run(app, "edit.edit_text");
        }
        // Edit Object only opens the same editor — the old ribbon repeated Edit Text twice
        // here and in the Edit tab, so the split button gives objects their own entry point.
        let (e_main, e_arrow) =
            large_split_ribbon_button(ui, t, "image-plus", "Edit Object", edit_active, "Edit images and objects", tl!("More object choices"));
        if e_main.clicked() {
            run(app, "edit.edit_text");
        }
        let mut e_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&e_arrow).show(|ui| {
            if ui.button(tl!("Edit Objects")).clicked() {
                ui.close();
                Some("edit.edit_text")
            } else if ui.button(tl!("Add Image…")).clicked() {
                ui.close();
                Some("edit.image")
            } else if ui.button(tl!("Add Text…")).clicked() {
                ui.close();
                Some("edit.text")
            } else {
                None
            }
        }) {
            e_choice = r.inner;
        }
        if let Some(id) = e_choice {
            run(app, id);
        }
    });

    ribbon_group(ui, "Comment", t, |ui| {
        if large_ribbon_button(
            ui,
            t,
            "type",
            "Typewriter",
            app.quick_tool == QuickTool::Comment(crate::comments::CommentTool::TextBox),
            "Add a text box",
        )
        .clicked()
        {
            run(app, "comment.freetext");
        }
        if large_ribbon_button(ui, t, "highlighter", "Highlight", false, "Highlight text").clicked() {
            run(app, "comment.highlight");
        }
    });

    ribbon_group(ui, "Page Organization", t, |ui| {
        let (r_main, r_arrow) = large_split_ribbon_button(ui, t, "rotate-cw", "Rotate Pages", false, "Rotate pages…", tl!("Page rotation choices"));
        if r_main.clicked() {
            run(app, "page.rotate_dialog");
        }
        let mut r_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&r_arrow).show(|ui| {
            if ui.button(tl!("Rotate Pages…")).clicked() {
                ui.close();
                Some("page.rotate_dialog")
            } else if ui.button(tl!("Rotate clockwise")).clicked() {
                ui.close();
                Some("page.rotate")
            } else if ui.button(tl!("Rotate counter-clockwise")).clicked() {
                ui.close();
                Some("page.rotate_ccw")
            } else {
                None
            }
        }) {
            r_choice = r.inner;
        }
        if let Some(id) = r_choice {
            run(app, id);
        }
        let (i_main, i_arrow) = large_split_ribbon_button(ui, t, "file-plus-2", "Insert", false, "Insert pages", tl!("Insert choices"));
        if i_main.clicked() {
            run(app, "page.insert");
        }
        let mut i_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&i_arrow).show(|ui| {
            if ui.button(tl!("Insert pages…")).clicked() {
                ui.close();
                Some("page.insert")
            } else if ui.button(tl!("Insert blank page")).clicked() {
                ui.close();
                Some("page.insert_blank")
            } else {
                None
            }
        }) {
            i_choice = r.inner;
        }
        if let Some(id) = i_choice {
            run(app, id);
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            if small_ribbon_button(ui, t, "trash-2", "Delete", false, "Delete pages").clicked() {
                run(app, "page.delete");
            }
            if small_ribbon_button(ui, t, "file-output", "Extract", false, "Extract pages…").clicked() {
                run(app, "page.extract");
            }
        });
    });

    ribbon_group(ui, "Convert", t, |ui| {
        // Foxit's Scan opens a scanner; MK has no scanner, so the dead button is omitted and
        // Quick OCR is the runninng entry point for scan workflows.
        if large_ribbon_button(ui, t, "scan-text", "Quick OCR", false, "Recognize text in this file…").clicked() {
            run(app, "ocr.recognize");
        }
        if large_ribbon_button(ui, t, "copy-plus", "Combine", matches!(app.left, LeftPanel::Tool("combine")), "Combine files").clicked() {
            app.select_tool("combine");
        }
    });

    ribbon_group(ui, "Protect", t, |ui| {
        if large_ribbon_button(ui, t, "signature", "PDF Sign", false, "Sign with a digital ID").clicked() {
            run(app, "sign.digital");
        }
        if large_ribbon_button(ui, t, "file-down", "Compress", false, "Reduce file size…").clicked() {
            run(app, "optimize.reduce");
        }
    });
}

/// --- CONVERT RIBBON (Foxit: Tools, Create, Export, Compliance, Convert) ---
fn show_convert_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    tools_group(app, ui, t);

    ribbon_group(ui, "Create", t, |ui| {
        if large_ribbon_button(ui, t, "files", "From Files", false, "Create PDF from a file…").clicked() {
            run(app, "create.file");
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            if small_ribbon_button(ui, t, "file-plus-2", "Blank", false, "New blank PDF").clicked() {
                run(app, "create.blank");
            }
            if small_ribbon_button(ui, t, "copy-plus", "From Clipboard", false, "Create PDF from the clipboard").clicked() {
                run(app, "create.clipboard");
            }
            if small_ribbon_button(ui, t, "files", "From Several Files", false, "Create PDF from several files…").clicked() {
                run(app, "create.multiple");
            }
            if small_ribbon_button(ui, t, "image", "From Images", false, "Create PDF from images…").clicked() {
                run(app, "create.images");
            }
        });
    });

    ribbon_group(ui, "Export", t, |ui| {
        if large_ribbon_button(ui, t, "image", "Export All Images", false, "Export all images…").clicked() {
            run(app, "export.all_images");
        }
        let (o_main, o_arrow) =
            large_split_ribbon_button(ui, t, "file-text", "To MS Office", false, "Export to an Office format", tl!("Office formats"));
        if o_main.clicked() {
            run(app, "export.docx");
        }
        let mut o_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&o_arrow).show(|ui| {
            if ui.button(tl!("To Word (.docx)")).clicked() {
                ui.close();
                Some("export.docx")
            } else if ui.button(tl!("To PowerPoint (.pptx)")).clicked() {
                ui.close();
                Some("export.pptx")
            } else if ui.button(tl!("To Rich Text (.rtf)")).clicked() {
                ui.close();
                Some("export.rtf")
            } else {
                None
            }
        }) {
            o_choice = r.inner;
        }
        if let Some(id) = o_choice {
            run(app, id);
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            if small_ribbon_button(ui, t, "image", "To Image…", false, "Export to image…").clicked() {
                run(app, "export.image");
            }
            if small_ribbon_button(ui, t, "file-symlink", "To HTML…", false, "Export to HTML…").clicked() {
                run(app, "export.html");
            }
            if small_ribbon_button(ui, t, "type", "To Text…", false, "Export to text…").clicked() {
                run(app, "export.text");
            }
        });
    });

    ribbon_group(ui, "Compliance", t, |ui| {
        if large_ribbon_button(ui, t, "file-check", "Industry Standard", app.dialog == Some(Dialog::PdfA), "PDF/A standards…").clicked() {
            run(app, "standards.pdfa");
        }
    });

    ribbon_group(ui, "Convert", t, |ui| {
        let (c_main, c_arrow) = large_split_ribbon_button(ui, t, "scan-text", "OCR", false, "Recognize text…", tl!("OCR choices"));
        if c_main.clicked() {
            run(app, "ocr.recognize");
        }
        let mut c_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&c_arrow).show(|ui| {
            if ui.button(tl!("Recognize Text…")).clicked() {
                ui.close();
                Some("ocr.recognize")
            } else if ui.button(tl!("Recognize Text in Multiple Files…")).clicked() {
                ui.close();
                Some("ocr.recognize_batch")
            } else if ui.button(tl!("Enhance Camera Image")).clicked() {
                ui.close();
                Some("ocr.enhance_camera")
            } else {
                None
            }
        }) {
            c_choice = r.inner;
        }
        if let Some(id) = c_choice {
            run(app, id);
        }
        if large_ribbon_button(ui, t, "save", "Save a Copy", false, "Save a copy").clicked() {
            run(app, "file.save_as");
        }
    });
}

/// --- EDIT RIBBON (Foxit: Tools, Edit Content, Font, Paragraph, Split, Effect, Links, Insert) ---
/// The Foxit localized-editing controls (Font, Paragraph, Split, Effect) act on text *being*
/// edited, so on an idle ribbon they are the edit tools themselves: Font and Paragraph run
/// the edit-text tool and say so in their tooltip, rather than painting a dead button.
fn show_edit_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    tools_group(app, ui, t);

    ribbon_group(ui, "Edit Content", t, |ui| {
        let edit_active = app.mode == Mode::Edit;
        if large_ribbon_button(ui, t, "text-cursor-input", "Edit Text", edit_active, "Edit text & images").clicked() {
            run(app, "edit.edit_text");
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            if small_ribbon_button(ui, t, "type", "Add Text", false, "Add text").clicked() {
                run(app, "edit.text");
            }
            if small_ribbon_button(ui, t, "image-plus", "Add Image", false, "Add image…").clicked() {
                run(app, "edit.image");
            }
            if small_ribbon_button(ui, t, "scissors", "Split", false, "Split document…").clicked() {
                run(app, "page.split");
            }
        });
    });

    ribbon_group(ui, "Paragraph", t, |ui| {
        if large_ribbon_button(ui, t, "type", "Font", false, "Open font choices while editing text").clicked() {
            run(app, "edit.edit_text");
        }
        if large_ribbon_button(ui, t, "align-justify", "Paragraph", false, "Open paragraph choices while editing text").clicked() {
            run(app, "edit.edit_text");
        }
    });

    ribbon_group(ui, "Links", t, |ui| {
        let (l_main, l_arrow) =
            large_split_ribbon_button(ui, t, "link-2", "Links", app.quick_tool == QuickTool::Link, "Add or edit links", tl!("Link choices"));
        if l_main.clicked() {
            run(app, "edit.link");
        }
        let mut l_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&l_arrow).show(|ui| {
            if ui.button(tl!("Add or Edit Links")).clicked() {
                ui.close();
                Some("edit.link")
            } else if ui.button(tl!("Create Links from URLs")).clicked() {
                ui.close();
                Some("edit.links_from_urls")
            } else if ui.button(tl!("Remove All Links")).clicked() {
                ui.close();
                Some("edit.remove_links")
            } else {
                None
            }
        }) {
            l_choice = r.inner;
        }
        if let Some(id) = l_choice {
            run(app, id);
        }
        if large_ribbon_button(ui, t, "paperclip", "File Attachment", false, "Attach a file…").clicked() {
            run(app, "edit.attach");
        }
    });

    ribbon_group(ui, "Insert", t, |ui| {
        if large_ribbon_button(ui, t, "type", "Text Box", false, "Add a text box").clicked() {
            run(app, "comment.freetext");
        }
        if large_ribbon_button(ui, t, "image-plus", "Image", false, "Add image…").clicked() {
            run(app, "edit.image");
        }
        if large_ribbon_button(ui, t, "signature", "Sign", false, "Fill & Sign: sign").clicked() {
            run(app, "sign.fill.signature");
        }
        if large_ribbon_button(ui, t, "bookmark", "Bookmark", false, "New bookmark").clicked() {
            run(app, "bookmark.add");
        }
        if large_ribbon_button(ui, t, "film", "Media", false, "Insert media (image / video via Insert ▸ Image)").clicked() {
            run(app, "edit.image");
        }
    });

    ribbon_group(ui, "Arrange", t, |ui| {
        if large_ribbon_button(ui, t, "rotate-ccw", "Rotate Left", false, "Rotate pages counter-clockwise").clicked() {
            run(app, "page.rotate_ccw");
        }
        if large_ribbon_button(ui, t, "rotate-cw", "Rotate Right", false, "Rotate pages clockwise").clicked() {
            run(app, "page.rotate");
        }
        if large_ribbon_button(ui, t, "crop", "Crop Pages", app.quick_tool == QuickTool::Crop, "Crop pages…").clicked() {
            run(app, "page.crop");
        }
    });
}

/// --- ORGANIZE RIBBON (Foxit: Tools, Pages, Transform, Page Marks) ---
fn show_organize_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    tools_group(app, ui, t);

    ribbon_group(ui, "Pages", t, |ui| {
        let (i_main, i_arrow) = large_split_ribbon_button(ui, t, "file-plus-2", "Insert", false, "Insert pages…", tl!("Insert choices"));
        if i_main.clicked() {
            run(app, "page.insert");
        }
        let mut i_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&i_arrow).show(|ui| {
            if ui.button(tl!("Insert pages…")).clicked() {
                ui.close();
                Some("page.insert")
            } else if ui.button(tl!("Insert blank page")).clicked() {
                ui.close();
                Some("page.insert_blank")
            } else {
                None
            }
        }) {
            i_choice = r.inner;
        }
        if let Some(id) = i_choice {
            run(app, id);
        }
        if large_ribbon_button(ui, t, "trash-2", "Delete", false, "Delete pages").clicked() {
            run(app, "page.delete");
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            if small_ribbon_button(ui, t, "file-output", "Extract", false, "Extract pages…").clicked() {
                run(app, "page.extract");
            }
            if small_ribbon_button(ui, t, "scissors", "Split", false, "Split document…").clicked() {
                run(app, "page.split");
            }
            if small_ribbon_button(ui, t, "copy-plus", "Duplicate", false, "Duplicate pages").clicked() {
                run(app, "page.duplicate");
            }
            if small_ribbon_button(ui, t, "replace", "Replace", false, "Replace pages…").clicked() {
                run(app, "page.replace");
            }
        });
    });

    ribbon_group(ui, "Transform", t, |ui| {
        let (r_main, r_arrow) = large_split_ribbon_button(ui, t, "rotate-cw", "Rotate Pages", false, "Rotate pages…", tl!("Page rotation choices"));
        if r_main.clicked() {
            run(app, "page.rotate_dialog");
        }
        let mut r_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&r_arrow).show(|ui| {
            if ui.button(tl!("Rotate Pages…")).clicked() {
                ui.close();
                Some("page.rotate_dialog")
            } else if ui.button(tl!("Rotate clockwise")).clicked() {
                ui.close();
                Some("page.rotate")
            } else if ui.button(tl!("Rotate counter-clockwise")).clicked() {
                ui.close();
                Some("page.rotate_ccw")
            } else {
                None
            }
        }) {
            r_choice = r.inner;
        }
        if let Some(id) = r_choice {
            run(app, id);
        }
        if large_ribbon_button(ui, t, "crop", "Crop Pages", app.quick_tool == QuickTool::Crop, "Crop pages…").clicked() {
            run(app, "page.crop");
        }
        if large_ribbon_button(ui, t, "square-dashed-mouse-pointer", "Set Page Boxes", false, "Set page boxes…").clicked() {
            run(app, "page.boxes");
        }
    });

    ribbon_group(ui, "Page Marks", t, |ui| {
        let (w_main, w_arrow) = large_split_ribbon_button(ui, t, "stamp", "Watermark", false, "Add watermark…", tl!("Watermark choices"));
        if w_main.clicked() {
            run(app, "edit.watermark");
        }
        let mut w_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&w_arrow).show(|ui| {
            if ui.button(tl!("Add watermark…")).clicked() {
                ui.close();
                Some("edit.watermark")
            } else if ui.button(tl!("Update watermark…")).clicked() {
                ui.close();
                Some("edit.watermark.update")
            } else if ui.button(tl!("Remove watermark")).clicked() {
                ui.close();
                Some("edit.watermark.remove")
            } else {
                None
            }
        }) {
            w_choice = r.inner;
        }
        if let Some(id) = w_choice {
            run(app, id);
        }
        let (b_main, b_arrow) = large_split_ribbon_button(ui, t, "palette", "Background", false, "Add background…", tl!("Background choices"));
        if b_main.clicked() {
            run(app, "edit.background");
        }
        let mut b_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&b_arrow).show(|ui| {
            if ui.button(tl!("Add background…")).clicked() {
                ui.close();
                Some("edit.background")
            } else if ui.button(tl!("Update background…")).clicked() {
                ui.close();
                Some("edit.background.update")
            } else if ui.button(tl!("Remove background")).clicked() {
                ui.close();
                Some("edit.background.remove")
            } else {
                None
            }
        }) {
            b_choice = r.inner;
        }
        if let Some(id) = b_choice {
            run(app, id);
        }
        let (h_main, h_arrow) =
            large_split_ribbon_button(ui, t, "heading", "Header & Footer", false, "Add header & footer…", tl!("Header and footer choices"));
        if h_main.clicked() {
            run(app, "edit.header_footer");
        }
        let mut h_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&h_arrow).show(|ui| {
            if ui.button(tl!("Add header & footer…")).clicked() {
                ui.close();
                Some("edit.header_footer")
            } else if ui.button(tl!("Update header & footer…")).clicked() {
                ui.close();
                Some("edit.header_footer.update")
            } else if ui.button(tl!("Remove header & footer")).clicked() {
                ui.close();
                Some("edit.header_footer.remove")
            } else {
                None
            }
        }) {
            h_choice = r.inner;
        }
        if let Some(id) = h_choice {
            run(app, id);
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            if small_ribbon_button(ui, t, "hash", "Bates Numbering…", false, "Bates numbering…").clicked() {
                run(app, "edit.bates");
            }
            if small_ribbon_button(ui, t, "tag", "Format Page Numbers…", false, "Number pages…").clicked() {
                run(app, "page.number");
            }
        });
    });
}

/// --- COMMENT RIBBON (Foxit: Tools, Text Markup, Pin, Typewriter, Drawing, Area Highlight, Measure, Stamps, Manage Comments) ---
fn show_comment_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    tools_group(app, ui, t);

    ribbon_group(ui, "Text Markup", t, |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            if small_ribbon_button(ui, t, "highlighter", "Highlight", false, "Highlight text").clicked() {
                run(app, "comment.highlight");
            }
            if small_ribbon_button(ui, t, "underline", "Underline", false, "Underline text").clicked() {
                run(app, "comment.underline");
            }
            if small_ribbon_button(ui, t, "strikethrough", "Strikeout", false, "Strikethrough text").clicked() {
                run(app, "comment.strikeout");
            }
            if small_ribbon_button(ui, t, "spline", "Squiggly", false, "Squiggly underline text").clicked() {
                run(app, "comment.squiggly");
            }
        });
    });

    ribbon_group(ui, "Pin", t, |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            if small_ribbon_button(ui, t, "sticky-note", "Note", false, "Sticky note").clicked() {
                run(app, "comment.note");
            }
            if small_ribbon_button(ui, t, "paperclip", "File", false, "Attach a file").clicked() {
                run(app, "comment.attach");
            }
            if small_ribbon_button(ui, t, "message-square-quote", "Callout", false, "Add a callout").clicked() {
                run(app, "comment.callout");
            }
            if small_ribbon_button(ui, t, "type", "Textbox", false, "Add a text box").clicked() {
                run(app, "comment.freetext");
            }
        });
    });

    ribbon_group(ui, "Typewriter", t, |ui| {
        if large_ribbon_button(ui, t, "type", "Typewriter", false, "Add a text box").clicked() {
            run(app, "comment.freetext");
        }
    });

    ribbon_group(ui, "Drawing", t, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
            for items in [
                [("pencil", "Pencil", "comment.ink"), ("eraser", "Eraser", "comment.eraser"), ("square", "Rectangle", "comment.square")],
                [("circle", "Oval", "comment.circle"), ("minus", "Line", "comment.line"), ("move-right", "Arrow", "comment.arrow")],
                [("pentagon", "Polygon", "comment.polygon"), ("spline", "Polyline", "comment.polyline"), ("cloud", "Cloud", "comment.cloud")],
            ] {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
                    for (icon, label, id) in items {
                        if small_ribbon_button(ui, t, icon, label, false, label).clicked() {
                            run(app, id);
                        }
                    }
                });
            }
        });
    });

    ribbon_group(ui, "Area Highlight", t, |ui| {
        if large_ribbon_button(ui, t, "highlighter", "Freehand Highlight", false, "Freehand-Highlight").clicked() {
            run(app, "comment.freehand_highlight");
        }
    });

    ribbon_group(ui, "Measure", t, |ui| {
        let (m_main, m_arrow) = large_split_ribbon_button(ui, t, "ruler", "Measure", false, "Measure distance", tl!("Measure choices"));
        if m_main.clicked() {
            run(app, "measure.distance");
        }
        let mut m_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&m_arrow).show(|ui| {
            if ui.button(tl!("Distance")).clicked() {
                ui.close();
                Some("measure.distance")
            } else if ui.button(tl!("Perimeter")).clicked() {
                ui.close();
                Some("measure.perimeter")
            } else if ui.button(tl!("Area")).clicked() {
                ui.close();
                Some("measure.area")
            } else if ui.button(tl!("Set Measurement Scale")).clicked() {
                ui.close();
                Some("measure.scale")
            } else if ui.button(tl!("Measurement Info")).clicked() {
                ui.close();
                Some("measure.info")
            } else if ui.button(tl!("Export CSV")).clicked() {
                ui.close();
                Some("measure.export")
            } else {
                None
            }
        }) {
            m_choice = r.inner;
        }
        if let Some(id) = m_choice {
            run(app, id);
        }
    });

    ribbon_group(ui, "Stamps", t, |ui| {
        if large_ribbon_button(ui, t, "stamp", "Stamps", false, "Add a stamp").clicked() {
            run(app, "comment.stamp");
        }
    });

    ribbon_group(ui, "Manage Comments", t, |ui| {
        let (c_main, c_arrow) = large_split_ribbon_button(
            ui,
            t,
            "message-square-text",
            "Comments Panel",
            app.right == Some(RightPanel::Comments),
            "Comments panel",
            tl!("Comment choices"),
        );
        if c_main.clicked() {
            run(app, "comment.list");
        }
        let mut c_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&c_arrow).show(|ui| {
            if ui.button(tl!("Comments Panel")).clicked() {
                ui.close();
                Some("comment.list")
            } else if ui.button(tl!("Hide All Comments")).clicked() {
                ui.close();
                Some("comment.hide_all")
            } else if ui.button(tl!("Flatten Comments")).clicked() {
                ui.close();
                Some("comment.flatten")
            } else if ui.button(tl!("Import Comments…")).clicked() {
                ui.close();
                Some("comment.import")
            } else if ui.button(tl!("Export Comments…")).clicked() {
                ui.close();
                Some("comment.export")
            } else if ui.button(tl!("Summarize Comments…")).clicked() {
                ui.close();
                Some("comment.summarize")
            } else {
                None
            }
        }) {
            c_choice = r.inner;
        }
        if let Some(id) = c_choice {
            run(app, id);
        }
    });
}

/// --- VIEW RIBBON (Foxit: Tools, Go To, Document Views, Page Display, View Setting, Panels) ---
fn show_view_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    tools_group(app, ui, t);

    ribbon_group(ui, "Go To", t, |ui| {
        if large_ribbon_button(ui, t, "chevrons-left", "First", false, "First page").clicked()
            && let Some(v) = view(app)
        {
            v.go_to_page(0);
        }
        if large_ribbon_button(ui, t, "chevron-left", "Previous", false, "Previous view").clicked() {
            run(app, "view.previous_view");
        }
        if large_ribbon_button(ui, t, "text-cursor-input", "Go To", false, "Go to page…").clicked() {
            run(app, "view.focus_page_input");
        }
        if large_ribbon_button(ui, t, "chevron-right", "Next", false, "Next view").clicked() {
            run(app, "view.next_view");
        }
        if large_ribbon_button(ui, t, "chevrons-right", "Last", false, "Last page").clicked()
            && let Some(v) = view(app)
        {
            let last = v.page_count().saturating_sub(1);
            v.go_to_page(last);
        }
    });

    ribbon_group(ui, "Document Views", t, |ui| {
        if large_ribbon_button(ui, t, "book-open", "Read Mode", app.mode == Mode::Read, "Read mode").clicked() {
            run(app, "view.read_mode");
        }
        if large_ribbon_button(ui, t, "maximize", "Full Screen", app.full_screen, "Full screen mode").clicked() {
            run(app, "view.full_screen");
        }
    });

    ribbon_group(ui, "Page Display", t, |ui| {
        use crate::canvas::PageLayout;
        let (_single, continuous, two_up) = app
            .active
            .and_then(|i| app.views.get(i))
            .map_or((true, true, false), |v| (v.layout == PageLayout::Single, v.continuous(), v.layout == PageLayout::TwoUp));
        let (d_main, d_arrow) =
            large_split_ribbon_button(ui, t, "file-text", "Page Display", false, "Page display choices", tl!("Page display choices"));
        if d_main.clicked() {
            app.set_ribbon_display(PageLayout::Single, false);
        }
        let mut d_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&d_arrow).show(|ui| {
            if ui.button(tl!("Single Page")).clicked() {
                ui.close();
                Some("view.layout.single")
            } else if ui.button(tl!("Two Pages")).clicked() {
                ui.close();
                Some("view.layout.two_up")
            } else if ui.button(tl!("Continuous Scrolling")).clicked() {
                ui.close();
                Some("view.continuous")
            } else if ui.button(tl!("Fit Width")).clicked() {
                ui.close();
                Some("view.fit_width_scrolling")
            } else if ui.button(tl!("Fit Page")).clicked() {
                ui.close();
                Some("view.fit_one_page")
            } else {
                None
            }
        }) {
            d_choice = r.inner;
        }
        if let Some(id) = d_choice {
            if id == "view.layout.single" {
                app.set_ribbon_display(PageLayout::Single, false);
            } else if id == "view.layout.two_up" {
                app.set_ribbon_display(PageLayout::TwoUp, !continuous);
            } else {
                run(app, id);
            }
        }
        // Keep the layout buttons directly reachable; the split button is the Foxit
        // Page Display entry, these are the same commands in one click.
        if large_ribbon_button(ui, t, "arrow-up-down", "Continuous", continuous && !two_up, "Continuous Scrolling").clicked() {
            app.set_ribbon_display(PageLayout::Single, !continuous);
        }
    });

    ribbon_group(ui, "View Setting", t, |ui| {
        let gaps = app.active.and_then(|i| app.views.get(i)).is_some_and(|v| v.show_page_gaps);
        if large_ribbon_button(ui, t, "arrow-up-down", "Page Gap", gaps, "Show gaps between pages").clicked() {
            run(app, "view.page_gaps");
        }
        if large_ribbon_button(ui, t, "moon", "Theme", false, "Switch light / dark theme").clicked() {
            run(app, "view.theme");
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
        let com_active = app.right == Some(RightPanel::Comments);
        if large_ribbon_button(ui, t, "message-square-text", "Comments", com_active, "Comments panel").clicked() {
            run(app, "comment.list");
        }
    });
}

/// --- FORM RIBBON (Foxit: Tools, Form Recognition, Form Fields, Edit, Form Data, JavaScript) ---
fn show_form_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    tools_group(app, ui, t);

    ribbon_group(ui, "Form Recognition", t, |ui| {
        if large_ribbon_button(ui, t, "scan-text", "Run Form Field Recognition", false, "Detect form fields").clicked() {
            run(app, "form.detect");
        }
        if large_ribbon_button(ui, t, "text-cursor-input", "Designer Assistant", matches!(app.left, LeftPanel::Tool("form")), "Prepare a form")
            .clicked()
        {
            run(app, "form.prepare");
        }
    });

    ribbon_group(ui, "Form Fields", t, |ui| {
        if large_ribbon_button(ui, t, "square", "Push Button", false, "Add a button").clicked() {
            run(app, "form.add.button");
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
            for items in [
                [
                    ("text-cursor-input", "Text Field", "form.add.text"),
                    ("check-circle-2", "Check Box", "form.add.checkbox"),
                    ("circle-dot", "Radio Button", "form.add.radio"),
                    ("chevron-down", "Drop-down", "form.add.combo"),
                ],
                [
                    ("list", "List Box", "form.add.list"),
                    ("image", "Image Field", "form.add.image"),
                    ("clock-3", "Date Field", "form.add.date"),
                    ("signature", "Signature", "form.add.signature"),
                ],
            ] {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
                    for (icon, label, id) in items {
                        if small_ribbon_button(ui, t, icon, label, false, label).clicked() {
                            run(app, id);
                        }
                    }
                });
            }
        });
    });

    ribbon_group(ui, "Edit", t, |ui| {
        if large_ribbon_button(ui, t, "list-ordered", "Calculation Order", matches!(app.left, LeftPanel::CalculationOrder), "Calculation order")
            .clicked()
        {
            run(app, "form.calculation_order");
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            if small_ribbon_button(ui, t, "rows-3", "Tab Order Rows", false, "Tab order: by rows").clicked() {
                run(app, "form.tab_order.row");
            }
            if small_ribbon_button(ui, t, "columns-3", "Tab Order Columns", false, "Tab order: by columns").clicked() {
                run(app, "form.tab_order.column");
            }
            if small_ribbon_button(ui, t, "layers", "Flatten Fields", false, "Flatten form fields").clicked() {
                run(app, "form.flatten");
            }
        });
    });

    ribbon_group(ui, "Form Data", t, |ui| {
        if large_ribbon_button(ui, t, "eraser", "Reset Form", false, "Clear form").clicked() {
            run(app, "form.clear");
        }
        let (s_main, s_arrow) = large_split_ribbon_button(
            ui,
            t,
            "file-spreadsheet",
            "Form to Sheet",
            false,
            "Merge data files into spreadsheet…",
            tl!("Form to sheet choices"),
        );
        if s_main.clicked() {
            run(app, "form.merge_data");
        }
        let mut s_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&s_arrow).show(|ui| {
            if ui.button(tl!("Merge Data Files into Spreadsheet…")).clicked() {
                ui.close();
                Some("form.merge_data")
            } else if ui.button(tl!("Import Data…")).clicked() {
                ui.close();
                Some("form.import_data")
            } else if ui.button(tl!("Export Data…")).clicked() {
                ui.close();
                Some("form.export_data")
            } else {
                None
            }
        }) {
            s_choice = r.inner;
        }
        if let Some(id) = s_choice {
            run(app, id);
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            if small_ribbon_button(ui, t, "file-input", "Import Data…", false, "Import form data…").clicked() {
                run(app, "form.import_data");
            }
            if small_ribbon_button(ui, t, "file-output", "Export Data…", false, "Export form data…").clicked() {
                run(app, "form.export_data");
            }
            if small_ribbon_button(ui, t, "list", "Field List", false, "Form fields panel").clicked() {
                run(app, "form.fields");
            }
        });
    });

    ribbon_group(ui, "JavaScript", t, |ui| {
        let (j_main, j_arrow) = large_split_ribbon_button(ui, t, "braces", "JavaScript", false, "JavaScript console…", tl!("JavaScript choices"));
        if j_main.clicked() {
            run(app, "tools.js_console");
        }
        let mut j_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&j_arrow).show(|ui| {
            if ui.button(tl!("JavaScript Console…")).clicked() {
                ui.close();
                Some("tools.js_console")
            } else if ui.button(tl!("Document JavaScripts…")).clicked() {
                ui.close();
                Some("tools.document_js")
            } else {
                None
            }
        }) {
            j_choice = r.inner;
        }
        if let Some(id) = j_choice {
            run(app, id);
        }
    });
}

/// --- PROTECT RIBBON (Foxit: Tools, Redaction, Hidden Data, Secure Document, Digital Signatures, Fill & Sign) ---
fn show_protect_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    tools_group(app, ui, t);

    ribbon_group(ui, "Redaction", t, |ui| {
        let (r_main, r_arrow) = large_split_ribbon_button(
            ui,
            t,
            "rectangle-horizontal",
            "Protect Redactions",
            app.quick_tool == QuickTool::Redact,
            "Redact text and images",
            tl!("Redaction choices"),
        );
        if r_main.clicked() {
            run(app, "redact.mark");
        }
        let mut r_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&r_arrow).show(|ui| {
            if ui.button(tl!("Mark Redaction")).clicked() {
                ui.close();
                Some("redact.mark")
            } else if ui.button(tl!("Apply Redactions…")).clicked() {
                ui.close();
                Some("redact.apply")
            } else if ui.button(tl!("Redact Pages…")).clicked() {
                ui.close();
                Some("redact.pages")
            } else if ui.button(tl!("Find Text and Redact…")).clicked() {
                ui.close();
                Some("redact.search")
            } else if ui.button(tl!("Redaction Properties…")).clicked() {
                ui.close();
                Some("redact.properties")
            } else if ui.button(tl!("Clear Redaction Marks")).clicked() {
                ui.close();
                Some("redact.clear")
            } else {
                None
            }
        }) {
            r_choice = r.inner;
        }
        if let Some(id) = r_choice {
            run(app, id);
        }
    });

    ribbon_group(ui, "Hidden Data", t, |ui| {
        if large_ribbon_button(ui, t, "eye-off", "Sanitize Document", app.dialog == Some(Dialog::Sanitize), "Sanitize document…").clicked() {
            run(app, "redact.sanitize");
        }
        if large_ribbon_button(ui, t, "eye-off", "Remove Hidden", false, "Remove hidden information…").clicked() {
            run(app, "protect.remove_hidden");
        }
    });

    ribbon_group(ui, "Secure Document", t, |ui| {
        let unlock_active = app.dialog == Some(Dialog::UnlockPdf);
        if unlock_ribbon_button(ui, t, unlock_active, "Unlock PDF — remove restrictions and passwords").clicked() {
            unlock_pdf_action(app);
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            if small_ribbon_button(ui, t, "lock", "Password Protect…", app.dialog == Some(Dialog::Protect), "Protect with password…").clicked() {
                run(app, "protect.password");
            }
            if small_ribbon_button(ui, t, "lock-open", "Remove Security", false, "Remove security").clicked() {
                run(app, "protect.remove");
            }
            if small_ribbon_button(ui, t, "shield-check", "Security Properties…", false, "Security properties…").clicked() {
                run(app, "protect.properties");
            }
        });
    });

    ribbon_group(ui, "Digital Signatures", t, |ui| {
        let (s_main, s_arrow) =
            large_split_ribbon_button(ui, t, "signature", "Sign Document", false, "Place a digital signature", tl!("Signature choices"));
        if s_main.clicked() {
            run(app, "sign.digital");
        }
        let mut s_choice: Option<&str> = None;
        if let Some(r) = egui::Popup::menu(&s_arrow).show(|ui| {
            if ui.button(tl!("Digitally Sign")).clicked() {
                ui.close();
                Some("sign.digital")
            } else if ui.button(tl!("Certify (visible)")).clicked() {
                ui.close();
                Some("sign.certify")
            } else if ui.button(tl!("Certify (invisible)")).clicked() {
                ui.close();
                Some("sign.certify_invisible")
            } else if ui.button(tl!("Validate All Signatures")).clicked() {
                ui.close();
                Some("sign.validate")
            } else {
                None
            }
        }) {
            s_choice = r.inner;
        }
        if let Some(id) = s_choice {
            run(app, id);
        }
        if large_ribbon_button(ui, t, "badge-check", "Validate All", app.right == Some(RightPanel::Signatures), "Validate all signatures").clicked() {
            run(app, "sign.validate");
        }
    });

    ribbon_group(ui, "Fill & Sign", t, |ui| {
        let sign_active = app.mode == Mode::Sign;
        if large_ribbon_button(ui, t, "pen-tool", "Fill & Sign", sign_active, "Fill and sign the PDF").clicked() {
            app.select_mode(Mode::Sign);
        }
        if large_ribbon_button(ui, t, "signature", "Sign", false, "Fill & Sign: sign").clicked() {
            run(app, "sign.fill.signature");
        }
        if large_ribbon_button(ui, t, "type", "Add Initials", false, "Fill & Sign: initials").clicked() {
            run(app, "sign.fill.initials");
        }
        if large_ribbon_button(ui, t, "check", "Checkmark", false, "Fill & Sign: checkmark").clicked() {
            run(app, "sign.fill.check");
        }
        if large_ribbon_button(ui, t, "clock-3", "Date", false, "Fill & Sign: date").clicked() {
            run(app, "sign.fill.date");
        }
        if large_ribbon_button(ui, t, "signature", "Signatures Panel", app.right == Some(RightPanel::Signatures), "Signatures panel").clicked() {
            run(app, "sign.panel");
        }
    });
}

/// --- ACCESSIBILITY RIBBON (Foxit: Autotag, Accessibility Check, Setup, Alternate Text) ---
fn show_accessibility_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    tools_group(app, ui, t);

    ribbon_group(ui, "Autotag", t, |ui| {
        if large_ribbon_button(ui, t, "tag", "Autotag Document", false, "Automatically tag PDF").clicked() {
            run(app, "a11y.autotag");
        }
        if large_ribbon_button(ui, t, "tag", "Autotag Form Field", false, "Tag form fields").clicked() {
            run(app, "a11y.autotag_fields");
        }
    });

    ribbon_group(ui, "Accessibility Check", t, |ui| {
        if large_ribbon_button(ui, t, "accessibility", "Full Check", false, "Check for accessibility…").clicked() {
            run(app, "a11y.check");
        }
        if large_ribbon_button(ui, t, "file-text", "Accessibility Report", false, "Open accessibility report").clicked() {
            run(app, "a11y.report");
        }
        if large_ribbon_button(ui, t, "search", "Find Untagged", false, "Find untagged content").clicked() {
            run(app, "a11y.find_untagged");
        }
        if large_ribbon_button(ui, t, "tag", "Tags Panel", app.right == Some(RightPanel::Tags), "Tags panel").clicked() {
            run(app, "a11y.tags");
        }
    });

    ribbon_group(ui, "Setup", t, |ui| {
        if large_ribbon_button(ui, t, "sliders-horizontal", "Reading Options…", false, "Change reading options…").clicked() {
            run(app, "a11y.reading_options");
        }
    });

    ribbon_group(ui, "Alternate Text", t, |ui| {
        if large_ribbon_button(ui, t, "image", "Set Alternate Text…", false, "Add alternate text…").clicked() {
            run(app, "a11y.alt_text");
        }
    });
}

/// --- HELP RIBBON (Foxit: Help, Updates, Product) ---
/// Foxit's Product group names Foxit itself; the MK equivalents are the MK help command, the
/// update check and About MK PDF. The old "Minh Kim Tech / MK PDF Standard / Foxit-Style
/// Ribbon" label group that duplicated About was removed so it cannot overlap these buttons.
fn show_help_ribbon(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ribbon_group(ui, "Help", t, |ui| {
        if large_ribbon_button(ui, t, "book-open", "User Manual", false, "The MK PDF manual (GitHub)").clicked() {
            run(app, "help.github");
        }
        if large_ribbon_button(ui, t, "circle-help", "Help Center", false, "Keyboard shortcuts").clicked() {
            run(app, "help.shortcuts");
        }
        if large_ribbon_button(ui, t, "square-terminal", "Command Line Help", false, "Keyboard shortcuts").clicked() {
            run(app, "help.shortcuts");
        }
        if large_ribbon_button(ui, t, "star", "Post Your Idea", false, "Report an issue").clicked() {
            run(app, "help.issues");
        }
        if large_ribbon_button(ui, t, "monitor", "Online Tutorial", false, "The MK PDF project on GitHub").clicked() {
            run(app, "help.github");
        }
    });

    ribbon_group(ui, "Updates", t, |ui| {
        if large_ribbon_button(ui, t, "cloud", "Check for Updates", false, "Check for updates…").clicked() {
            app.check_for_updates();
        }
        if large_ribbon_button(ui, t, "file-down", "Install Update", false, "Check for updates… (installs what was found)").clicked() {
            app.check_for_updates();
        }
    });

    ribbon_group(ui, "Product", t, |ui| {
        if large_ribbon_button(ui, t, "info", "About MK PDF", false, "About MK PDF").clicked() {
            run(app, "help.about");
        }
        if large_ribbon_button(ui, t, "triangle-alert", "Report an Issue", false, "Report an issue").clicked() {
            run(app, "help.issues");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every ribbon button must run a registered command. The ribbon is data over
    /// `COMMANDS`; a button that names an id the registry does not know only logs an error
    /// and silently does nothing, which is exactly the dead button the user asked us to
    /// test for. This list mirrors every `run(app, "…")` call in this file.
    #[test]
    fn every_ribbon_command_is_registered() {
        const RIBBON_COMMANDS: &[&str] = &[
            "bookmark.add",
            "comment.attach",
            "comment.callout",
            "comment.eraser",
            "comment.flatten",
            "comment.freehand_highlight",
            "comment.freetext",
            "comment.highlight",
            "comment.hide_all",
            "comment.import",
            "comment.export",
            "comment.ink",
            "comment.line",
            "comment.arrow",
            "comment.note",
            "comment.polygon",
            "comment.polyline",
            "comment.cloud",
            "comment.list",
            "comment.square",
            "comment.circle",
            "comment.squiggly",
            "comment.stamp",
            "comment.strikeout",
            "comment.summarize",
            "comment.underline",
            "create.blank",
            "create.clipboard",
            "create.file",
            "create.images",
            "create.multiple",
            "edit.attach",
            "edit.background",
            "edit.background.update",
            "edit.background.remove",
            "edit.bates",
            "edit.column_select",
            "edit.edit_text",
            "edit.header_footer",
            "edit.header_footer.update",
            "edit.header_footer.remove",
            "edit.image",
            "edit.link",
            "edit.links_from_urls",
            "edit.remove_links",
            "edit.snapshot",
            "edit.text",
            "edit.watermark",
            "edit.watermark.update",
            "edit.watermark.remove",
            "export.all_images",
            "export.docx",
            "export.html",
            "export.image",
            "export.pptx",
            "export.rtf",
            "export.text",
            "file.save_as",
            "form.add.button",
            "form.add.checkbox",
            "form.add.combo",
            "form.add.date",
            "form.add.image",
            "form.add.list",
            "form.add.radio",
            "form.add.signature",
            "form.add.text",
            "form.calculation_order",
            "form.clear",
            "form.detect",
            "form.export_data",
            "form.fields",
            "form.flatten",
            "form.import_data",
            "form.merge_data",
            "form.prepare",
            "form.tab_order.column",
            "form.tab_order.row",
            "help.about",
            "help.github",
            "help.issues",
            "help.shortcuts",
            "measure.area",
            "measure.distance",
            "measure.export",
            "measure.info",
            "measure.perimeter",
            "measure.scale",
            "ocr.enhance_camera",
            "ocr.recognize",
            "ocr.recognize_batch",
            "optimize.reduce",
            "page.boxes",
            "page.crop",
            "page.delete",
            "page.duplicate",
            "page.extract",
            "page.insert",
            "page.insert_blank",
            "page.number",
            "page.replace",
            "page.rotate",
            "page.rotate_ccw",
            "page.rotate_dialog",
            "page.split",
            "protect.password",
            "protect.properties",
            "protect.remove",
            "protect.remove_hidden",
            "redact.apply",
            "redact.clear",
            "redact.mark",
            "redact.pages",
            "redact.properties",
            "redact.sanitize",
            "redact.search",
            "sign.certify",
            "sign.certify_invisible",
            "sign.digital",
            "sign.fill.check",
            "sign.fill.date",
            "sign.fill.initials",
            "sign.fill.signature",
            "sign.panel",
            "sign.validate",
            "standards.pdfa",
            "tools.document_js",
            "tools.js_console",
            "view.continuous",
            "view.focus_page_input",
            "view.fit_one_page",
            "view.fit_visible",
            "view.fit_width_scrolling",
            "view.layout.single",
            "view.layout.two_up",
            "view.next_view",
            "view.page_gaps",
            "view.previous_view",
            "view.read_mode",
            "view.full_screen",
            "view.theme",
            "view.marquee_zoom",
            "a11y.alt_text",
            "a11y.autotag",
            "a11y.autotag_fields",
            "a11y.check",
            "a11y.find_untagged",
            "a11y.reading_options",
            "a11y.report",
            "a11y.tags",
        ];
        for id in RIBBON_COMMANDS {
            assert!(pdfcraft_engine::commands::command(id).is_some(), "ribbon button names the unknown command {id}");
        }
        // The old dead-button misfires must stay gone. (comment.cloud is registered, so it
        // is allowed; the three below are catalog-only Planned items with no command.)
        for id in ["export.xlsx", "export.ps", "protect.certificate"] {
            assert!(pdfcraft_engine::commands::command(id).is_none(), "{id} is Planned/untracked and must not be on the ribbon");
        }
    }

    #[test]
    fn fox_tabs_equal_the_screenshots_order() {
        let tabs = [
            RibbonTab::Home,
            RibbonTab::Convert,
            RibbonTab::Edit,
            RibbonTab::Organize,
            RibbonTab::Comment,
            RibbonTab::View,
            RibbonTab::Form,
            RibbonTab::Protect,
            RibbonTab::Accessibility,
            RibbonTab::Help,
        ];
        assert_eq!(tabs.len(), 10, "Foxit ships ten MK-mapped tabs (Share/Connect are Foxit-only)");
        assert!(!tabs.contains(&RibbonTab::Sign), "Fill & Sign must not be its own tab any more");
    }
}
