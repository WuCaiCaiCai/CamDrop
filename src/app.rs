use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use chrono::Datelike;
use eframe::egui;

use camdrop::detector;
use camdrop::organizer::{self, DateFilter, Event, Options, PreviewItem};

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum FilterMode {
    All,
    Year,
    Month,
    Day,
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
    view: ViewMode,
    target: String,
    copy_only: bool,
    filter_mode: FilterMode,
    filter_year: i32,
    filter_month: u32,
    filter_day: u32,
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
            view: ViewMode::Files,
            target,
            copy_only: false,
            filter_mode: FilterMode::All,
            filter_year: 0,
            filter_month: 1,
            filter_day: 1,
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
        match self.filter_mode {
            FilterMode::All => DateFilter::All,
            FilterMode::Year => DateFilter::Year(self.filter_year),
            FilterMode::Month => DateFilter::Month(self.filter_year, self.filter_month),
            FilterMode::Day => {
                DateFilter::Day(self.filter_year, self.filter_month, self.filter_day)
            }
        }
    }

    /// Distinct years, months (within the selected year) and days (within the
    /// selected month) present in the scanned files.
    fn available_dates(&self) -> (Vec<i32>, Vec<u32>, Vec<u32>) {
        let mut years = BTreeSet::new();
        let mut months = BTreeSet::new();
        let mut days = BTreeSet::new();
        for item in &self.preview {
            let Some(date) = item.date else {
                continue;
            };
            years.insert(date.year());
            if date.year() == self.filter_year {
                months.insert(date.month());
                if date.month() == self.filter_month {
                    days.insert(date.day());
                }
            }
        }
        (
            years.into_iter().collect(),
            months.into_iter().collect(),
            days.into_iter().collect(),
        )
    }

    /// Keeps the selected year/month/day pointing at values that exist.
    fn sync_filter(&mut self) {
        let (years, _, _) = self.available_dates();
        if !years.is_empty() && !years.contains(&self.filter_year) {
            self.filter_year = years[0];
        }
        let (_, months, _) = self.available_dates();
        if !months.is_empty() && !months.contains(&self.filter_month) {
            self.filter_month = months[0];
        }
        let (_, _, days) = self.available_dates();
        if !days.is_empty() && !days.contains(&self.filter_day) {
            self.filter_day = days[0];
        }
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
            let items = organizer::preview(&sources, &Options::default(), |event| {
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
                    if self.cards.is_empty() {
                        self.log
                            .push("未检测到存储卡，点击「添加文件夹」手动选择".to_owned());
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

    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.interact_size.y = 28.0;
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

    let accent = egui::Color32::from_rgb(96, 165, 250);
    let surface = egui::Color32::from_rgb(30, 33, 41);
    let surface_hi = egui::Color32::from_rgb(41, 45, 56);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(22, 24, 30);
    visuals.window_fill = surface;
    visuals.extreme_bg_color = egui::Color32::from_rgb(16, 18, 22);
    visuals.faint_bg_color = surface;
    visuals.code_bg_color = surface;
    visuals.hyperlink_color = accent;
    visuals.selection.bg_fill = accent.gamma_multiply(0.35);
    visuals.selection.stroke = egui::Stroke::new(1.0, accent);
    visuals.window_corner_radius = egui::CornerRadius::same(10);
    visuals.menu_corner_radius = egui::CornerRadius::same(8);

    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::same(7);
    }
    visuals.widgets.inactive.weak_bg_fill = surface_hi;
    visuals.widgets.hovered.weak_bg_fill = surface_hi.gamma_multiply(1.25);
    visuals.widgets.active.weak_bg_fill = surface_hi.gamma_multiply(1.45);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, accent.gamma_multiply(0.7));

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
        self.sync_filter();

        egui::Panel::top("header").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("CamDrop");
                ui.add_space(8.0);
                ui.weak("相机存储卡按日期迁移");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.detecting || self.scanning || self.busy {
                        ui.spinner();
                    }
                    let (done, total) = self.progress;
                    if total > 0 {
                        let fraction = done as f32 / total as f32;
                        ui.add(egui::ProgressBar::new(fraction).desired_width(190.0));
                        ui.weak(format!("{done}/{total}"));
                    }
                    if (self.scanning || self.busy) && !self.progress_label.is_empty() {
                        ui.label(&self.progress_label);
                    }
                });
            });
            ui.add_space(6.0);
        });

        egui::Panel::bottom("log")
            .default_size(110.0)
            .min_size(48.0)
            .max_size(280.0)
            .resizable(true)
            .show(ui, |ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.strong("日志");
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
            .default_size(310.0)
            .min_size(260.0)
            .resizable(true)
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.strong("1 · 选择来源");
                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .id_salt("cards")
                    .max_height(140.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.cards.is_empty() && !self.detecting {
                            ui.weak("未检测到存储卡");
                        }
                        for card in &mut self.cards {
                            ui.checkbox(&mut card.selected, card.path.display().to_string());
                        }
                    });

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!self.busy, egui::Button::new("刷新"))
                        .clicked()
                    {
                        self.rescan_cards(&ctx);
                    }
                    if ui
                        .add_enabled(!self.busy, egui::Button::new("添加文件夹…"))
                        .clicked()
                        && let Some(dir) = rfd::FileDialog::new().pick_folder()
                    {
                        self.cards.push(Card {
                            path: dir,
                            selected: true,
                        });
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.strong("2 · 扫描");
                ui.add_space(4.0);
                let can_scan =
                    !self.busy && !self.scanning && !self.detecting && !self.cards.is_empty();
                if ui
                    .add_enabled(
                        can_scan,
                        egui::Button::new("扫描所选来源")
                            .min_size(egui::vec2(ui.available_width(), 30.0)),
                    )
                    .clicked()
                {
                    self.scan(&ctx);
                }

                ui.add_space(8.0);
                ui.separator();
                ui.strong("3 · 迁移设置");
                ui.add_space(4.0);
                ui.label("目标目录");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.target).desired_width(160.0));
                    if ui.button("浏览…").clicked()
                        && let Some(dir) = rfd::FileDialog::new().pick_folder()
                    {
                        self.target = dir.display().to_string();
                    }
                });

                ui.label("方式");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.copy_only, false, "移动（删除源文件）");
                    ui.radio_value(&mut self.copy_only, true, "复制（保留源文件）");
                });

                ui.label("时间筛选");
                let (years, months, days) = self.available_dates();
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("filter_mode")
                        .selected_text(match self.filter_mode {
                            FilterMode::All => "全部",
                            FilterMode::Year => "按年",
                            FilterMode::Month => "按月",
                            FilterMode::Day => "按天",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.filter_mode, FilterMode::All, "全部");
                            ui.selectable_value(&mut self.filter_mode, FilterMode::Year, "按年");
                            ui.selectable_value(&mut self.filter_mode, FilterMode::Month, "按月");
                            ui.selectable_value(&mut self.filter_mode, FilterMode::Day, "按天");
                        });
                    if matches!(
                        self.filter_mode,
                        FilterMode::Year | FilterMode::Month | FilterMode::Day
                    ) {
                        egui::ComboBox::from_id_salt("filter_year")
                            .width(70.0)
                            .selected_text(self.filter_year.to_string())
                            .show_ui(ui, |ui| {
                                for year in &years {
                                    ui.selectable_value(
                                        &mut self.filter_year,
                                        *year,
                                        year.to_string(),
                                    );
                                }
                            });
                    }
                    if matches!(self.filter_mode, FilterMode::Month | FilterMode::Day) {
                        egui::ComboBox::from_id_salt("filter_month")
                            .width(58.0)
                            .selected_text(format!("{:02}", self.filter_month))
                            .show_ui(ui, |ui| {
                                for month in &months {
                                    ui.selectable_value(
                                        &mut self.filter_month,
                                        *month,
                                        format!("{month:02}"),
                                    );
                                }
                            });
                    }
                    if matches!(self.filter_mode, FilterMode::Day) {
                        egui::ComboBox::from_id_salt("filter_day")
                            .width(58.0)
                            .selected_text(format!("{:02}", self.filter_day))
                            .show_ui(ui, |ui| {
                                for day in &days {
                                    ui.selectable_value(
                                        &mut self.filter_day,
                                        *day,
                                        format!("{day:02}"),
                                    );
                                }
                            });
                    }
                });

                ui.add_space(8.0);
                let filter = self.current_filter();
                let shown_count = self
                    .preview
                    .iter()
                    .filter(|item| filter.matches(item.date))
                    .count();
                let can_migrate =
                    !self.busy && !self.scanning && !self.cards.is_empty() && shown_count > 0;
                let start = ui
                    .add_enabled_ui(can_migrate, |ui| {
                        ui.add_sized(
                            [ui.available_width(), 36.0],
                            egui::Button::new(egui::RichText::new("开始迁移").size(16.0).strong()),
                        )
                    })
                    .inner;
                if start.clicked() {
                    self.migrate(&ctx);
                }
                if let Some(summary) = &self.summary {
                    ui.add_space(4.0);
                    ui.weak(summary);
                }
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

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.strong("预览");
                ui.add_space(10.0);
                ui.selectable_value(&mut self.view, ViewMode::Files, "文件");
                ui.selectable_value(&mut self.view, ViewMode::Tree, "迁移后目录");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.scanning {
                        ui.spinner();
                    }
                    ui.weak(format!("{shown_len} 个文件 · {}", human_size(total_size)));
                });
            });
            ui.separator();

            if shown_len == 0 {
                ui.add_space(20.0);
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
                                .spacing([18.0, 5.0])
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
                                        ui.label(target_path(item));
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
                                egui::CollapsingHeader::new(format!(
                                    "{folder}   ({} 个 · {})",
                                    items.len(),
                                    human_size(folder_size)
                                ))
                                .default_open(true)
                                .show(ui, |ui| {
                                    for item in items {
                                        ui.horizontal(|ui| {
                                            ui.label(file_name(item));
                                            ui.weak(human_size(item.size));
                                        });
                                    }
                                });
                            }
                        });
                }
            }
        });
    }
}
