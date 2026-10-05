use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use chrono::{Datelike, NaiveDate};
use eframe::egui;

use camdrop::detector;
use camdrop::organizer::{self, DateFilter, Event, Options, PreviewItem};

const ACCENT: egui::Color32 = egui::Color32::from_rgb(54, 170, 208);
const ACCENT_STRONG: egui::Color32 = egui::Color32::from_rgb(34, 132, 168);
const ON_ACCENT: egui::Color32 = egui::Color32::from_rgb(8, 20, 27);
const BG: egui::Color32 = egui::Color32::from_rgb(17, 21, 27);
const SURFACE: egui::Color32 = egui::Color32::from_rgb(26, 32, 40);
const SURFACE_HI: egui::Color32 = egui::Color32::from_rgb(38, 47, 59);

enum Msg {
    Cards(Vec<PathBuf>),
    Scanned(Vec<PreviewItem>),
    Log(String),
    Progress(usize, usize),
    Done(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Files,
    Tree,
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
    target: String,
    copy_only: bool,
    detecting: bool,
    scanning: bool,
    busy: bool,
    progress: (usize, usize),
    progress_label: String,
    summary: Option<String>,
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
            target,
            copy_only: false,
            detecting: true,
            scanning: false,
            busy: false,
            progress: (0, 0),
            progress_label: String::new(),
            summary: None,
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

    fn migrate(&mut self, ctx: &egui::Context) {
        let sources = self.selected_sources();
        if sources.is_empty() {
            self.log.push("请先选择来源".to_owned());
            return;
        }

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
                    if self.cards.is_empty() {
                        self.log
                            .push("未检测到存储卡，点击「选择来源文件夹」手动选择".to_owned());
                    }
                }
                Msg::Scanned(items) => {
                    self.preview = items;
                    self.scanning = false;
                    self.progress = (0, 0);
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
    ctx.set_theme(egui::ThemePreference::Dark);
    let theme = egui::Theme::Dark;
    let mut style = (*ctx.style_of(theme)).clone();

    style.spacing.item_spacing = egui::vec2(9.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 7.0);
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
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
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

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = SURFACE;
    visuals.extreme_bg_color = egui::Color32::from_rgb(12, 16, 21);
    visuals.faint_bg_color = SURFACE;
    visuals.code_bg_color = SURFACE;
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke = egui::Stroke::new(1.0, ON_ACCENT);
    visuals.window_corner_radius = egui::CornerRadius::same(12);
    visuals.menu_corner_radius = egui::CornerRadius::same(9);

    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::same(8);
    }
    visuals.widgets.inactive.weak_bg_fill = SURFACE_HI;
    visuals.widgets.inactive.bg_fill = SURFACE_HI;
    visuals.widgets.hovered.weak_bg_fill = ACCENT_STRONG;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    visuals.widgets.active.weak_bg_fill = ACCENT_STRONG;

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

fn section(ui: &mut egui::Ui, text: &str) {
    ui.add_space(12.0);
    ui.label(egui::RichText::new(text).size(15.0).strong().color(ACCENT));
    ui.add_space(5.0);
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

        egui::Panel::top("header").show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("CamDrop")
                        .size(23.0)
                        .strong()
                        .color(ACCENT),
                );
                ui.add_space(8.0);
                ui.weak("相机存储卡按日期迁移");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.detecting || self.scanning || self.busy {
                        ui.spinner();
                    }
                    let (done, total) = self.progress;
                    if total > 0 {
                        let fraction = done as f32 / total as f32;
                        ui.add(
                            egui::ProgressBar::new(fraction)
                                .desired_width(200.0)
                                .fill(ACCENT),
                        );
                        ui.weak(format!("{done}/{total}"));
                    }
                    if (self.scanning || self.busy) && !self.progress_label.is_empty() {
                        ui.label(
                            egui::RichText::new(self.progress_label.as_str())
                                .strong()
                                .color(ACCENT),
                        );
                    }
                });
            });
            ui.add_space(8.0);
        });

        egui::Panel::bottom("log")
            .default_size(120.0)
            .min_size(48.0)
            .max_size(300.0)
            .resizable(true)
            .show(ui, |ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("日志").strong().color(ACCENT));
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
            .default_size(340.0)
            .min_size(300.0)
            .resizable(true)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("controls_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let full = ui.available_width();

                        section(ui, "1 · 选择来源");

                        let add =
                            egui::Button::new(egui::RichText::new("📁  选择来源文件夹").size(15.0))
                                .min_size(egui::vec2(full, 42.0))
                                .fill(SURFACE_HI);
                        if ui.add_enabled(!self.busy, add).clicked()
                            && let Some(dir) = rfd::FileDialog::new().pick_folder()
                        {
                            self.cards.push(Card {
                                path: dir,
                                selected: true,
                            });
                            self.preview.clear();
                            self.selected_days.clear();
                        }

                        ui.add_space(4.0);
                        egui::ScrollArea::vertical()
                            .id_salt("cards")
                            .max_height(110.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                if self.cards.is_empty() && !self.detecting {
                                    ui.weak("未检测到存储卡");
                                }
                                for card in &mut self.cards {
                                    ui.checkbox(
                                        &mut card.selected,
                                        card.path.display().to_string(),
                                    );
                                }
                            });

                        if ui
                            .add_enabled(!self.busy, egui::Button::new("🔄  重新检测存储卡"))
                            .clicked()
                        {
                            self.rescan_cards(&ctx);
                        }

                        section(ui, "2 · 扫描");
                        let can_scan = !self.busy
                            && !self.scanning
                            && !self.detecting
                            && !self.cards.is_empty();
                        let scan = egui::Button::new(
                            egui::RichText::new("🔍  扫描所选来源")
                                .size(15.0)
                                .color(ON_ACCENT),
                        )
                        .min_size(egui::vec2(full, 38.0))
                        .fill(ACCENT);
                        if ui.add_enabled(can_scan, scan).clicked() {
                            self.scan(&ctx);
                        }

                        section(ui, "3 · 按时间筛选");
                        if self.preview.is_empty() {
                            ui.weak("扫描后可在此按时间筛选");
                        } else {
                            let tree = self.available_dates();
                            let selected = self.selected_days.clone();
                            let toggles: RefCell<Vec<(Vec<NaiveDate>, bool)>> =
                                RefCell::new(Vec::new());

                            for (year, months) in &tree {
                                let year_days: Vec<NaiveDate> = months
                                    .iter()
                                    .flat_map(|(m, days)| {
                                        days.iter().map(move |d| {
                                            NaiveDate::from_ymd_opt(*year, *m, *d).unwrap()
                                        })
                                    })
                                    .collect();
                                let year_all = year_days.iter().all(|d| selected.contains(d));
                                let mut check = year_all;
                                if ui
                                    .checkbox(
                                        &mut check,
                                        egui::RichText::new(format!("{year} 年")).strong(),
                                    )
                                    .changed()
                                {
                                    toggles.borrow_mut().push((year_days, year_all));
                                }

                                for (month, days) in months {
                                    let month_days: Vec<NaiveDate> = days
                                        .iter()
                                        .map(|d| {
                                            NaiveDate::from_ymd_opt(*year, *month, *d).unwrap()
                                        })
                                        .collect();
                                    let month_all = month_days.iter().all(|d| selected.contains(d));
                                    ui.horizontal(|ui| {
                                        ui.add_space(18.0);
                                        let mut check = month_all;
                                        if ui
                                            .checkbox(&mut check, format!("{month:02} 月"))
                                            .changed()
                                        {
                                            toggles
                                                .borrow_mut()
                                                .push((month_days.clone(), month_all));
                                        }
                                    });

                                    for day in days {
                                        let date =
                                            NaiveDate::from_ymd_opt(*year, *month, *day).unwrap();
                                        let was_selected = selected.contains(&date);
                                        let mut check = was_selected;
                                        ui.horizontal(|ui| {
                                            ui.add_space(38.0);
                                            if ui
                                                .checkbox(&mut check, format!("{day:02} 日"))
                                                .changed()
                                            {
                                                toggles
                                                    .borrow_mut()
                                                    .push((vec![date], was_selected));
                                            }
                                        });
                                    }
                                }
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
                            if self.selected_days.is_empty() {
                                ui.weak("未筛选：迁移全部时间");
                            } else {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "已选 {} 天",
                                            self.selected_days.len()
                                        ))
                                        .color(ACCENT),
                                    );
                                    if ui.small_button("清除筛选").clicked() {
                                        self.selected_days.clear();
                                    }
                                });
                            }
                        }

                        section(ui, "4 · 迁移");
                        ui.label("目标目录");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.target)
                                    .desired_width(full - 74.0),
                            );
                            if ui.button("浏览…").clicked()
                                && let Some(dir) = rfd::FileDialog::new().pick_folder()
                            {
                                self.target = dir.display().to_string();
                            }
                        });

                        ui.add_space(2.0);
                        ui.label("方式");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.copy_only, false, "移动（删除源文件）");
                            ui.radio_value(&mut self.copy_only, true, "复制（保留源文件）");
                        });

                        ui.add_space(8.0);
                        let filter = self.current_filter();
                        let shown_count = self
                            .preview
                            .iter()
                            .filter(|item| filter.matches(item.date))
                            .count();
                        let can_migrate = !self.busy
                            && !self.scanning
                            && !self.cards.is_empty()
                            && shown_count > 0;

                        let migrate = egui::Button::new(
                            egui::RichText::new("▶  开始迁移")
                                .size(17.0)
                                .strong()
                                .color(ON_ACCENT),
                        )
                        .min_size(egui::vec2(full, 44.0))
                        .fill(ACCENT);
                        if ui.add_enabled(can_migrate, migrate).clicked() {
                            self.migrate(&ctx);
                        }
                        if let Some(summary) = &self.summary {
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(summary.as_str()).color(ACCENT));
                        }
                        ui.add_space(10.0);
                    });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            let filter = self.current_filter();
            let shown_len = self
                .preview
                .iter()
                .filter(|item| filter.matches(item.date))
                .count();
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
                        .color(ACCENT),
                );
                ui.add_space(12.0);
                ui.selectable_value(&mut self.view, ViewMode::Files, "文件");
                ui.selectable_value(&mut self.view, ViewMode::Tree, "迁移后目录");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.scanning {
                        ui.spinner();
                    }
                    ui.label(
                        egui::RichText::new(format!(
                            "{shown_len} 个文件 · {}",
                            human_size(total_size)
                        ))
                        .color(ACCENT),
                    );
                });
            });
            ui.add_space(4.0);
            ui.separator();

            if shown_len == 0 {
                ui.add_space(24.0);
                if self.scanning {
                    ui.weak("正在扫描…");
                } else if self.preview.is_empty() {
                    ui.weak("在左侧选择来源，然后点击「扫描所选来源」查看迁移计划。");
                } else {
                    ui.weak("当前筛选条件下没有文件。");
                }
                return;
            }

            let shown: Vec<&PreviewItem> = self
                .preview
                .iter()
                .filter(|item| filter.matches(item.date))
                .collect();

            match self.view {
                ViewMode::Files => {
                    egui::ScrollArea::both()
                        .id_salt("preview_files")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            egui::Grid::new("preview_grid")
                                .num_columns(5)
                                .striped(true)
                                .spacing([18.0, 6.0])
                                .show(ui, |ui| {
                                    ui.strong("文件名");
                                    ui.strong("大小");
                                    ui.strong("拍摄时间");
                                    ui.strong("迁移到");
                                    ui.strong("XMP");
                                    ui.end_row();

                                    for item in &shown {
                                        ui.label(file_name(item));
                                        ui.label(human_size(item.size));
                                        ui.label(&item.captured);
                                        ui.label(
                                            egui::RichText::new(target_path(item)).color(ACCENT),
                                        );
                                        ui.label(if item.has_xmp { "有" } else { "—" });
                                        ui.end_row();
                                    }
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
                                .color(ACCENT);
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
    }
}
