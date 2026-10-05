use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use chrono::{Datelike, NaiveDate};
use eframe::egui;
use egui_extras::{Column, TableBuilder};

use camdrop::detector;
use camdrop::organizer::{self, DateFilter, Event, Options, PreviewItem};

const ACCENT: egui::Color32 = egui::Color32::from_rgb(42, 166, 208);
const ACCENT_DARK: egui::Color32 = egui::Color32::from_rgb(28, 128, 168);
const ON_ACCENT: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
const BG: egui::Color32 = egui::Color32::from_rgb(245, 247, 250);
const SURFACE: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
const SURFACE_HI: egui::Color32 = egui::Color32::from_rgb(233, 238, 244);
const TEXT: egui::Color32 = egui::Color32::from_rgb(31, 41, 51);
const TEXT_WEAK: egui::Color32 = egui::Color32::from_rgb(100, 116, 139);
const BORDER: egui::Color32 = egui::Color32::from_rgb(226, 232, 240);

enum Msg {
    Cards(Vec<PathBuf>),
    Scanned(Vec<PreviewItem>),
    Processed(PathBuf),
    Log(String),
    Progress(usize, usize),
    Done(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Files,
    Tree,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Source,
    Scan,
    Filter,
    Migrate,
}

struct Card {
    path: PathBuf,
    selected: bool,
}

pub struct CamDropApp {
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    cards: Vec<Card>,
    preview: Vec<PreviewItem>,
    selected_days: BTreeSet<NaiveDate>,
    view: ViewMode,
    step: Step,
    target: String,
    copy_only: bool,
    detecting: bool,
    scanning: bool,
    busy: bool,
    progress: (usize, usize),
    progress_label: String,
    summary: Option<String>,
    dialog: Option<Vec<String>>,
    log: Vec<String>,
}

impl CamDropApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_cjk_font(&cc.egui_ctx);
        apply_style(&cc.egui_ctx);

        let (tx, rx) = mpsc::channel();
        let target = std::env::current_dir()
            .unwrap_or_default()
            .join("RAW")
            .display()
            .to_string();

        let ctx = cc.egui_ctx.clone();
        let tx0 = tx.clone();
        thread::spawn(move || {
            let cards = detector::detect_cards();
            let _ = tx0.send(Msg::Cards(cards));
            ctx.request_repaint();
        });

        Self {
            tx,
            rx,
            cards: Vec::new(),
            preview: Vec::new(),
            selected_days: BTreeSet::new(),
            view: ViewMode::Files,
            step: Step::Source,
            target,
            copy_only: false,
            detecting: true,
            scanning: false,
            busy: false,
            progress: (0, 0),
            progress_label: String::new(),
            summary: None,
            dialog: None,
            log: Vec::new(),
        }
    }

    fn selected_sources(&self) -> Vec<PathBuf> {
        self.cards
            .iter()
            .filter(|c| c.selected)
            .map(|c| c.path.clone())
            .collect()
    }

    fn current_filter(&self) -> DateFilter {
        if self.selected_days.is_empty() {
            DateFilter::All
        } else {
            DateFilter::Days(self.selected_days.clone())
        }
    }

    fn filtered_count(&self) -> usize {
        let filter = self.current_filter();
        self.preview
            .iter()
            .filter(|item| filter.matches(item.date))
            .count()
    }

    fn scan_available(&self) -> bool {
        !self.cards.is_empty()
    }

    fn filter_available(&self) -> bool {
        !self.preview.is_empty()
    }

    fn migrate_available(&self) -> bool {
        self.busy || !self.preview.is_empty() || self.summary.is_some()
    }

    /// Years → months → days present in the scanned files.
    fn available_dates(&self) -> BTreeMap<i32, BTreeMap<u32, BTreeSet<u32>>> {
        let mut tree: BTreeMap<i32, BTreeMap<u32, BTreeSet<u32>>> = BTreeMap::new();
        for item in &self.preview {
            let Some(date) = item.date else {
                continue;
            };
            tree.entry(date.year())
                .or_default()
                .entry(date.month())
                .or_default()
                .insert(date.day());
        }
        tree
    }

    fn clamp_step(&mut self) {
        if self.busy || self.scanning {
            return;
        }
        if self.step == Step::Migrate && !self.migrate_available() {
            self.step = Step::Filter;
        }
        if self.step == Step::Filter && !self.filter_available() {
            self.step = if self.scan_available() {
                Step::Scan
            } else {
                Step::Source
            };
        }
        if self.step == Step::Scan && !self.scan_available() {
            self.step = Step::Source;
        }
    }

    fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.selected_sources().is_empty() {
            problems.push("还没有选择来源，请在「选择来源」里点选存储卡。".to_owned());
        }
        if self.preview.is_empty() {
            problems.push("还没有扫描，请先在「扫描」里点「扫描所选来源」。".to_owned());
        } else if self.filtered_count() == 0 {
            problems.push("当前时间筛选下没有文件，请调整筛选或「清除筛选」。".to_owned());
        }
        if self.target.trim().is_empty() {
            problems.push("还没有选择目标目录，请在「迁移设置」里选择。".to_owned());
        }
        problems
    }

    fn rescan_cards(&mut self, ctx: &egui::Context) {
        self.detecting = true;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let cards = detector::detect_cards();
            let _ = tx.send(Msg::Cards(cards));
            ctx.request_repaint();
        });
    }

    fn scan(&mut self, ctx: &egui::Context) {
        let sources = self.selected_sources();
        if sources.is_empty() {
            self.log.push("请先选择来源".to_owned());
            return;
        }
        self.scanning = true;
        self.progress = (0, 0);
        self.progress_label = "扫描中".to_owned();
        self.summary = None;

        let tx = self.tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let opts = Options::default();
            let items = organizer::preview(&sources, &opts, |event| {
                if let Event::Progress(done, total) = event {
                    let _ = tx.send(Msg::Progress(done, total));
                    ctx.request_repaint();
                }
            });
            let _ = tx.send(Msg::Scanned(items));
            ctx.request_repaint();
        });
    }

    fn try_migrate(&mut self, ctx: &egui::Context) {
        let problems = self.validate();
        if problems.is_empty() {
            self.migrate(ctx);
        } else {
            self.dialog = Some(problems);
        }
    }

    fn migrate(&mut self, ctx: &egui::Context) {
        let sources = self.selected_sources();
        let target = PathBuf::from(self.target.clone());
        let opts = Options {
            copy_only: self.copy_only,
            dry_run: false,
            filter: self.current_filter(),
        };
        let tx = self.tx.clone();
        let ctx = ctx.clone();

        self.busy = true;
        self.progress = (0, 0);
        self.progress_label = "迁移中".to_owned();
        self.summary = None;
        self.log.clear();
        thread::spawn(move || run_migrate(sources, target, opts, tx, ctx));
    }

    fn status_line(&self) -> String {
        if self.detecting {
            "正在检测存储卡…".to_owned()
        } else if self.scanning {
            "正在扫描，请稍候…".to_owned()
        } else if self.preview.is_empty() {
            if self.cards.is_empty() {
                "未检测到存储卡，请点「手动选择」选择文件夹。".to_owned()
            } else {
                format!(
                    "已选 {} 个来源，点「扫描所选来源」生成迁移计划。",
                    self.selected_sources().len()
                )
            }
        } else {
            format!(
                "扫描到 {} 个文件，筛选后待迁移 {} 个。",
                self.preview.len(),
                self.filtered_count()
            )
        }
    }

    fn drain_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Cards(cards) => {
                    self.cards = cards
                        .into_iter()
                        .map(|path| Card {
                            path,
                            selected: true,
                        })
                        .collect();
                    self.detecting = false;
                    self.preview.clear();
                    self.selected_days.clear();
                    if !self.cards.is_empty() && self.step == Step::Source {
                        self.step = Step::Scan;
                    }
                    if self.cards.is_empty() {
                        self.log
                            .push("未检测到存储卡，点击「手动选择」选择文件夹".to_owned());
                    }
                }
                Msg::Scanned(items) => {
                    self.preview = items;
                    self.selected_days.clear();
                    self.scanning = false;
                    self.progress = (0, 0);
                    self.step = Step::Filter;
                }
                Msg::Processed(path) => {
                    self.preview.retain(|item| item.source != path);
                }
                Msg::Log(line) => self.log.push(line),
                Msg::Progress(done, total) => self.progress = (done, total),
                Msg::Done(text) => {
                    self.busy = false;
                    self.progress = (0, 0);
                    self.summary = Some(text.clone());
                    self.log.push(text);
                }
            }
        }
    }
}

fn run_migrate(
    sources: Vec<PathBuf>,
    target: PathBuf,
    opts: Options,
    tx: Sender<Msg>,
    ctx: egui::Context,
) {
    let done_tx = tx.clone();
    let event_ctx = ctx.clone();
    let summary = organizer::organize(&sources, &target, &opts, move |event| {
        let msg = match event {
            Event::Log(line) => Msg::Log(line),
            Event::Progress(done, total) => Msg::Progress(done, total),
            Event::Processed(path) => Msg::Processed(path),
        };
        let _ = tx.send(msg);
        event_ctx.request_repaint();
    });

    let text = format!(
        "完成：迁移 {}，XMP {}，补迁 {}，错误 {}",
        summary.moved, summary.xmp, summary.orphans, summary.errors
    );
    let _ = done_tx.send(Msg::Done(text));
    ctx.request_repaint();
}

fn apply_style(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Light);
    let theme = egui::Theme::Light;
    let mut style = (*ctx.style_of(theme)).clone();

    style.spacing.item_spacing = egui::vec2(9.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.interact_size.y = 30.0;
    style.text_styles = [
        (
            egui::TextStyle::Heading,
            egui::FontId::new(21.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Body,
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Button,
            egui::FontId::new(14.5, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Small,
            egui::FontId::new(11.5, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Monospace,
            egui::FontId::new(13.0, egui::FontFamily::Monospace),
        ),
    ]
    .into();

    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = BG;
    visuals.window_fill = SURFACE;
    visuals.extreme_bg_color = egui::Color32::from_rgb(255, 255, 255);
    visuals.faint_bg_color = egui::Color32::from_rgb(240, 244, 248);
    visuals.code_bg_color = SURFACE_HI;
    visuals.hyperlink_color = ACCENT_DARK;
    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke = egui::Stroke::new(1.0, ON_ACCENT);
    visuals.window_corner_radius = egui::CornerRadius::same(12);
    visuals.menu_corner_radius = egui::CornerRadius::same(9);
    visuals.override_text_color = Some(TEXT);

    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::same(8);
        widget.bg_stroke = egui::Stroke::new(1.0, BORDER);
        widget.fg_stroke = egui::Stroke::new(1.0, TEXT);
    }
    visuals.widgets.inactive.weak_bg_fill = SURFACE_HI;
    visuals.widgets.inactive.bg_fill = SURFACE_HI;
    visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(214, 230, 240);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, TEXT);
    visuals.widgets.active.weak_bg_fill = ACCENT;
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, ON_ACCENT);
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, TEXT);

    style.visuals = visuals;
    ctx.set_style_of(theme, style);
}

fn install_cjk_font(ctx: &egui::Context) {
    const CANDIDATES: &[&str] = &[
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/msyh.ttf",
        "C:/Windows/Fonts/simhei.ttf",
        "C:/Windows/Fonts/simsun.ttc",
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJKsc-Regular.otf",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc",
        "/usr/share/fonts/wenquanyi/wqy-zenhei/wqy-zenhei.ttc",
    ];

    for path in CANDIDATES {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };

        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "cjk".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("cjk".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("cjk".to_owned());
        ctx.set_fonts(fonts);
        return;
    }
}

/// A hand-drawn full-width button with clear hover and press feedback.
fn action_button(
    ui: &mut egui::Ui,
    label: &str,
    fill: egui::Color32,
    text_color: egui::Color32,
    height: f32,
    enabled: bool,
) -> egui::Response {
    let width = ui.available_width();
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), sense);

    let hovering = enabled && response.hovered();
    let pressing = enabled && response.is_pointer_button_down_on();
    let (color, text) = if !enabled {
        (
            egui::Color32::from_rgb(236, 240, 244),
            egui::Color32::from_rgb(160, 168, 178),
        )
    } else if pressing {
        (fill.gamma_multiply(0.85), text_color)
    } else if hovering {
        (fill.gamma_multiply(0.94), text_color)
    } else {
        (fill, text_color)
    };

    let radius = egui::CornerRadius::same(10);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, color);
    if enabled && (hovering || pressing) {
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(1.5, ACCENT.gamma_multiply(0.7)),
            egui::StrokeKind::Inside,
        );
    }
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(16.0),
        text,
    );

    if hovering {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn step_header(ui: &mut egui::Ui, title: &str, active: bool, available: bool) -> egui::Response {
    let (fill, text) = if active {
        (ACCENT, ON_ACCENT)
    } else if available {
        (SURFACE_HI, TEXT)
    } else {
        (SURFACE, TEXT_WEAK)
    };
    let label = if active {
        format!("▾  {title}")
    } else {
        format!("▸  {title}")
    };
    action_button(ui, &label, fill, text, 38.0, available)
}

/// A full-width toggle button for a detected card, filled with the accent when
/// selected.
fn toggle_button(ui: &mut egui::Ui, label: &str, selected: bool, height: f32) -> egui::Response {
    let fill = if selected { ACCENT } else { SURFACE_HI };
    let text = if selected { ON_ACCENT } else { TEXT };
    action_button(ui, label, fill, text, height, true)
}

/// A small chip that highlights when selected.
fn chip_button(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    let fill = if selected { ACCENT } else { SURFACE_HI };
    let color = if selected { ON_ACCENT } else { TEXT };
    ui.add(
        egui::Button::new(egui::RichText::new(text).size(13.0).color(color))
            .fill(fill)
            .corner_radius(7.0)
            .small(),
    )
}

fn file_name(item: &PreviewItem) -> String {
    item.source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn target_path(item: &PreviewItem) -> String {
    format!("{}/{}", item.folder, file_name(item))
}

fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

impl eframe::App for CamDropApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.drain_messages();
        self.clamp_step();

        egui::Panel::top("header").show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("CamDrop")
                        .size(23.0)
                        .strong()
                        .color(ACCENT_DARK),
                );
                ui.add_space(8.0);
                ui.weak("相机存储卡按日期迁移");
            });
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(self.status_line())
                    .size(12.5)
                    .color(ACCENT_DARK),
            );
            ui.add_space(8.0);
        });

        // Bottom status bar: result + progress, pinned to the very bottom.
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let left = if self.busy || self.scanning {
                    self.progress_label.clone()
                } else if let Some(summary) = &self.summary {
                    summary.clone()
                } else {
                    String::new()
                };
                if left.is_empty() {
                    ui.weak("就绪");
                } else {
                    ui.label(egui::RichText::new(left).strong().color(ACCENT_DARK));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.detecting || self.scanning || self.busy {
                        ui.spinner();
                    }
                    let (done, total) = self.progress;
                    if total > 0 {
                        let fraction = done as f32 / total as f32;
                        ui.add(
                            egui::ProgressBar::new(fraction)
                                .desired_width(220.0)
                                .fill(ACCENT),
                        );
                        ui.weak(format!("{done}/{total}"));
                    }
                });
            });
            ui.add_space(4.0);
        });

        egui::Panel::bottom("log")
            .default_size(96.0)
            .min_size(40.0)
            .max_size(300.0)
            .resizable(true)
            .show(ui, |ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("日志").strong().color(ACCENT_DARK));
                    if !self.log.is_empty() && ui.small_button("清空").clicked() {
                        self.log.clear();
                    }
                });
                egui::ScrollArea::vertical()
                    .id_salt("log_scroll")
                    .stick_to_bottom(true)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for line in &self.log {
                            ui.monospace(line);
                        }
                    });
            });

        egui::Panel::left("controls")
            .default_size(360.0)
            .min_size(320.0)
            .resizable(true)
            .show(ui, |ui| {
                egui::Panel::bottom("migrate_bar").show(ui, |ui| {
                    ui.add_space(8.0);
                    if action_button(ui, "▶  开始迁移", ACCENT, ON_ACCENT, 52.0, !self.busy)
                        .clicked()
                    {
                        self.try_migrate(&ctx);
                    }
                    ui.add_space(8.0);
                });

                egui::ScrollArea::vertical()
                    .id_salt("controls_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(8.0);

                        // Step 1 · source
                        if step_header(ui, "1 · 选择来源", self.step == Step::Source, true)
                            .clicked()
                            && self.step != Step::Source
                        {
                            self.step = Step::Source;
                        }
                        if self.step == Step::Source {
                            ui.add_space(6.0);
                            egui::ScrollArea::vertical()
                                .id_salt("cards")
                                .max_height(168.0)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    if self.cards.is_empty() && !self.detecting {
                                        ui.weak("未检测到存储卡");
                                    }
                                    for card in &mut self.cards {
                                        let label = format!(
                                            "{}  {}",
                                            if card.selected { "●" } else { "○" },
                                            card.path.display()
                                        );
                                        if toggle_button(ui, &label, card.selected, 42.0).clicked()
                                        {
                                            card.selected = !card.selected;
                                        }
                                    }
                                });
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui
                                    .add_enabled(!self.busy, egui::Button::new("📁  手动选择…"))
                                    .clicked()
                                    && let Some(dir) = rfd::FileDialog::new().pick_folder()
                                {
                                    self.cards.push(Card {
                                        path: dir,
                                        selected: true,
                                    });
                                    self.preview.clear();
                                    self.selected_days.clear();
                                }
                                if ui
                                    .add_enabled(!self.busy, egui::Button::new("🔄  重新检测"))
                                    .clicked()
                                {
                                    self.rescan_cards(&ctx);
                                }
                            });
                            ui.add_space(8.0);
                            let can = !self.busy && !self.cards.is_empty();
                            if action_button(ui, "继续", SURFACE_HI, TEXT, 40.0, can).clicked() {
                                self.step = Step::Scan;
                            }
                            ui.add_space(8.0);
                        }

                        // Step 2 · scan
                        if step_header(
                            ui,
                            "2 · 扫描",
                            self.step == Step::Scan,
                            self.scan_available(),
                        )
                        .clicked()
                            && self.step != Step::Scan
                        {
                            self.step = Step::Scan;
                        }
                        if self.step == Step::Scan {
                            ui.add_space(6.0);
                            let can = !self.busy
                                && !self.scanning
                                && !self.detecting
                                && !self.cards.is_empty();
                            if action_button(ui, "🔍  扫描所选来源", ACCENT, ON_ACCENT, 46.0, can)
                                .clicked()
                            {
                                self.scan(&ctx);
                            }
                            ui.add_space(8.0);
                        }

                        // Step 3 · date filter
                        if step_header(
                            ui,
                            "3 · 时间筛选",
                            self.step == Step::Filter,
                            self.filter_available(),
                        )
                        .clicked()
                            && self.step != Step::Filter
                        {
                            self.step = Step::Filter;
                        }
                        if self.step == Step::Filter {
                            ui.add_space(6.0);
                            if self.preview.is_empty() {
                                ui.weak("扫描后可在此按时间筛选");
                            } else {
                                let tree = self.available_dates();
                                let selected = self.selected_days.clone();
                                let toggles: RefCell<Vec<(Vec<NaiveDate>, bool)>> =
                                    RefCell::new(Vec::new());

                                for (year, months) in &tree {
                                    egui::CollapsingHeader::new(
                                        egui::RichText::new(format!("{year} 年"))
                                            .strong()
                                            .color(ACCENT_DARK),
                                    )
                                    .id_salt(("year", *year))
                                    .default_open(true)
                                    .show(ui, |ui| {
                                        let year_days: Vec<NaiveDate> = months
                                            .iter()
                                            .flat_map(|(m, days)| {
                                                days.iter().map(move |d| {
                                                    NaiveDate::from_ymd_opt(*year, *m, *d).unwrap()
                                                })
                                            })
                                            .collect();
                                        let year_all =
                                            year_days.iter().all(|d| selected.contains(d));
                                        if chip_button(ui, "全年", year_all).clicked() {
                                            toggles
                                                .borrow_mut()
                                                .push((year_days.clone(), year_all));
                                        }
                                        for (month, days) in months {
                                            let month_days: Vec<NaiveDate> = days
                                                .iter()
                                                .map(|d| {
                                                    NaiveDate::from_ymd_opt(*year, *month, *d)
                                                        .unwrap()
                                                })
                                                .collect();
                                            let month_all =
                                                month_days.iter().all(|d| selected.contains(d));
                                            ui.horizontal_wrapped(|ui| {
                                                if chip_button(
                                                    ui,
                                                    &format!("{month:02} 月"),
                                                    month_all,
                                                )
                                                .clicked()
                                                {
                                                    toggles
                                                        .borrow_mut()
                                                        .push((month_days.clone(), month_all));
                                                }
                                                for day in days {
                                                    let date = NaiveDate::from_ymd_opt(
                                                        *year, *month, *day,
                                                    )
                                                    .unwrap();
                                                    let sel = selected.contains(&date);
                                                    if chip_button(ui, &format!("{day:02}"), sel)
                                                        .clicked()
                                                    {
                                                        toggles
                                                            .borrow_mut()
                                                            .push((vec![date], sel));
                                                    }
                                                }
                                            });
                                        }
                                    });
                                }

                                for (days, was_all) in toggles.into_inner() {
                                    if was_all {
                                        for day in &days {
                                            self.selected_days.remove(day);
                                        }
                                    } else {
                                        for day in &days {
                                            self.selected_days.insert(*day);
                                        }
                                    }
                                }

                                ui.add_space(4.0);
                                if !self.selected_days.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "已选 {} 天",
                                                self.selected_days.len()
                                            ))
                                            .color(ACCENT_DARK),
                                        );
                                        if ui.small_button("清除筛选").clicked() {
                                            self.selected_days.clear();
                                        }
                                    });
                                }
                            }
                            ui.add_space(8.0);
                            if action_button(ui, "继续", SURFACE_HI, TEXT, 40.0, true).clicked() {
                                self.step = Step::Migrate;
                            }
                            ui.add_space(8.0);
                        }

                        // Step 4 · migrate settings
                        if step_header(
                            ui,
                            "4 · 迁移设置",
                            self.step == Step::Migrate,
                            self.migrate_available(),
                        )
                        .clicked()
                            && self.step != Step::Migrate
                        {
                            self.step = Step::Migrate;
                        }
                        if self.step == Step::Migrate {
                            ui.add_space(6.0);
                            ui.label("方式");
                            ui.horizontal(|ui| {
                                ui.radio_value(&mut self.copy_only, false, "移动（删除源文件）");
                                ui.radio_value(&mut self.copy_only, true, "复制（保留源文件）");
                            });
                            ui.add_space(6.0);
                            ui.label("目标目录");
                            if action_button(
                                ui,
                                "📂  选择目标文件夹",
                                SURFACE_HI,
                                TEXT,
                                44.0,
                                !self.busy,
                            )
                            .clicked()
                                && let Some(dir) = rfd::FileDialog::new().pick_folder()
                            {
                                self.target = dir.display().to_string();
                            }
                            ui.label(egui::RichText::new(&self.target).size(11.5).weak());
                            ui.add_space(12.0);
                        }
                    });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            let filter = self.current_filter();
            let shown_len = self.filtered_count();
            let total_size: u64 = self
                .preview
                .iter()
                .filter(|item| filter.matches(item.date))
                .map(|i| i.size)
                .sum();

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("预览")
                        .size(17.0)
                        .strong()
                        .color(ACCENT_DARK),
                );
                ui.add_space(12.0);
                ui.selectable_value(&mut self.view, ViewMode::Files, "文件");
                ui.selectable_value(&mut self.view, ViewMode::Tree, "目录预览");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.scanning {
                        ui.spinner();
                    }
                    ui.label(
                        egui::RichText::new(format!(
                            "{shown_len} 个文件 · {}",
                            human_size(total_size)
                        ))
                        .color(ACCENT_DARK),
                    );
                });
            });
            ui.add_space(4.0);
            ui.separator();

            if shown_len == 0 {
                ui.add_space(28.0);
                ui.vertical_centered(|ui| {
                    if self.scanning {
                        ui.weak("正在扫描…");
                    } else if self.preview.is_empty() {
                        ui.weak("① 选择来源  →  ② 扫描  →  ③ 筛选  →  ④ 迁移");
                        ui.add_space(6.0);
                        ui.weak("在左侧选择来源，然后点击「扫描所选来源」。");
                    } else {
                        ui.weak("当前筛选条件下没有文件。");
                    }
                });
                return;
            }

            let shown: Vec<&PreviewItem> = self
                .preview
                .iter()
                .filter(|item| filter.matches(item.date))
                .collect();

            match self.view {
                ViewMode::Files => {
                    TableBuilder::new(ui)
                        .striped(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::initial(220.0).at_least(120.0).resizable(true))
                        .column(Column::initial(80.0).at_least(60.0))
                        .column(Column::initial(170.0).at_least(140.0))
                        .column(Column::remainder().at_least(180.0))
                        .column(Column::initial(52.0).at_least(44.0))
                        .header(26.0, |mut header| {
                            header.col(|ui| {
                                ui.strong("文件名");
                            });
                            header.col(|ui| {
                                ui.strong("大小");
                            });
                            header.col(|ui| {
                                ui.strong("拍摄时间");
                            });
                            header.col(|ui| {
                                ui.strong("迁移到");
                            });
                            header.col(|ui| {
                                ui.strong("XMP");
                            });
                        })
                        .body(|body| {
                            body.rows(23.0, shown.len(), |mut row| {
                                let item = shown[row.index()];
                                row.col(|ui| {
                                    ui.label(file_name(item));
                                });
                                row.col(|ui| {
                                    ui.label(human_size(item.size));
                                });
                                row.col(|ui| {
                                    ui.label(&item.captured);
                                });
                                row.col(|ui| {
                                    ui.label(
                                        egui::RichText::new(target_path(item)).color(ACCENT_DARK),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(if item.has_xmp { "有" } else { "—" });
                                });
                            });
                        });
                }
                ViewMode::Tree => {
                    let mut groups: BTreeMap<&str, Vec<&PreviewItem>> = BTreeMap::new();
                    for item in &shown {
                        groups.entry(item.folder.as_str()).or_default().push(item);
                    }
                    egui::ScrollArea::vertical()
                        .id_salt("preview_tree")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (folder, items) in groups {
                                let folder_size: u64 = items.iter().map(|i| i.size).sum();
                                let header = egui::RichText::new(format!(
                                    "{folder}   ({} 个 · {})",
                                    items.len(),
                                    human_size(folder_size)
                                ))
                                .strong()
                                .color(ACCENT_DARK);
                                egui::CollapsingHeader::new(header).default_open(true).show(
                                    ui,
                                    |ui| {
                                        for item in items {
                                            ui.horizontal(|ui| {
                                                ui.label(file_name(item));
                                                ui.weak(human_size(item.size));
                                            });
                                        }
                                    },
                                );
                            }
                        });
                }
            }
        });

        // Validation popup.
        if let Some(problems) = self.dialog.clone() {
            let mut close = false;
            let response = egui::Modal::new(egui::Id::new("validation")).show(&ctx, |ui| {
                ui.set_max_width(440.0);
                ui.label(
                    egui::RichText::new("还不能开始迁移")
                        .size(17.0)
                        .strong()
                        .color(ACCENT_DARK),
                );
                ui.add_space(8.0);
                for problem in &problems {
                    ui.label(format!("• {problem}"));
                }
                ui.add_space(12.0);
                if ui.button("知道了").clicked() {
                    close = true;
                }
            });
            if close || response.should_close() {
                self.dialog = None;
            }
        }
    }
}
