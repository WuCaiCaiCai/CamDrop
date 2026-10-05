use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use eframe::egui;

use camdrop::detector;
use camdrop::organizer::{self, Event, Options, PreviewItem};

enum Msg {
    Cards(Vec<PathBuf>),
    Scanned {
        root: PathBuf,
        items: Vec<PreviewItem>,
    },
    Log(String),
    Progress(usize, usize),
    Done(String),
}

struct Card {
    path: PathBuf,
    selected: bool,
    items: Option<Vec<PreviewItem>>,
}

pub struct CamDropApp {
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    cards: Vec<Card>,
    target: String,
    copy_only: bool,
    dry_run: bool,
    scanning: bool,
    busy: bool,
    progress: (usize, usize),
    summary: Option<String>,
    log: Vec<String>,
}

impl CamDropApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_cjk_font(&cc.egui_ctx);

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
            let _ = tx0.send(Msg::Cards(cards.clone()));
            ctx.request_repaint();
            for root in cards {
                spawn_scan(tx0.clone(), root, ctx.clone());
            }
        });

        Self {
            tx,
            rx,
            cards: Vec::new(),
            target,
            copy_only: false,
            dry_run: false,
            scanning: true,
            busy: false,
            progress: (0, 0),
            summary: None,
            log: Vec::new(),
        }
    }

    fn rescan(&mut self, ctx: &egui::Context) {
        self.scanning = true;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let cards = detector::detect_cards();
            let _ = tx.send(Msg::Cards(cards.clone()));
            ctx.request_repaint();
            for root in cards {
                spawn_scan(tx.clone(), root, ctx.clone());
            }
        });
    }

    fn start(&mut self, ctx: &egui::Context) {
        let sources: Vec<PathBuf> = self
            .cards
            .iter()
            .filter(|c| c.selected)
            .map(|c| c.path.clone())
            .collect();
        if sources.is_empty() {
            self.log.push("请先选择至少一个源卡目录".to_owned());
            return;
        }

        let target = PathBuf::from(self.target.clone());
        let opts = Options {
            copy_only: self.copy_only,
            dry_run: self.dry_run,
        };
        let tx = self.tx.clone();
        let ctx = ctx.clone();

        self.busy = true;
        self.progress = (0, 0);
        self.summary = None;
        self.log.clear();
        thread::spawn(move || run_archive(sources, target, opts, tx, ctx));
    }

    fn drain_messages(&mut self, ctx: &egui::Context) {
        let mut refresh = false;
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Cards(cards) => {
                    self.cards = cards
                        .into_iter()
                        .map(|path| Card {
                            path,
                            selected: true,
                            items: None,
                        })
                        .collect();
                    self.scanning = false;
                    if self.cards.is_empty() {
                        self.log
                            .push("未检测到存储卡，点击「添加文件夹」手动选择".to_owned());
                    }
                }
                Msg::Scanned { root, items } => {
                    if let Some(card) = self.cards.iter_mut().find(|c| c.path == root) {
                        card.items = Some(items);
                    }
                }
                Msg::Log(line) => self.log.push(line),
                Msg::Progress(done, total) => self.progress = (done, total),
                Msg::Done(text) => {
                    self.busy = false;
                    self.summary = Some(text.clone());
                    self.log.push(text);
                    refresh = true;
                }
            }
        }

        if refresh {
            let roots: Vec<PathBuf> = self.cards.iter().map(|c| c.path.clone()).collect();
            for card in &mut self.cards {
                card.items = None;
            }
            for root in roots {
                spawn_scan(self.tx.clone(), root, ctx.clone());
            }
        }
    }
}

fn spawn_scan(tx: Sender<Msg>, root: PathBuf, ctx: egui::Context) {
    thread::spawn(move || {
        let items = organizer::preview(std::slice::from_ref(&root), |_| {});
        let _ = tx.send(Msg::Scanned { root, items });
        ctx.request_repaint();
    });
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

fn run_archive(
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
        "完成：处理 {}，XMP {}，补迁 {}，错误 {}",
        summary.moved, summary.xmp, summary.orphans, summary.errors
    );
    let _ = done_tx.send(Msg::Done(text));
    ctx.request_repaint();
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
        self.drain_messages(&ctx);

        egui::Panel::top("header").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading("CamDrop");
                ui.weak("相机存储卡按日期归档");
            });
            ui.add_space(4.0);
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
            .default_size(300.0)
            .min_size(250.0)
            .resizable(true)
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.strong("源卡");
                    if ui
                        .add_enabled(!self.busy, egui::Button::new("刷新"))
                        .clicked()
                    {
                        self.rescan(&ctx);
                    }
                });

                egui::ScrollArea::vertical()
                    .id_salt("cards")
                    .max_height(150.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.cards.is_empty() && !self.scanning {
                            ui.weak("未检测到存储卡");
                        }
                        for card in &mut self.cards {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut card.selected, "");
                                ui.label(card.path.display().to_string());
                                match &card.items {
                                    Some(items) => {
                                        ui.weak(format!("{} 个", items.len()));
                                    }
                                    None => {
                                        ui.spinner();
                                    }
                                }
                            });
                        }
                    });

                if ui
                    .add_enabled(!self.busy, egui::Button::new("添加文件夹…"))
                    .clicked()
                    && let Some(dir) = rfd::FileDialog::new().pick_folder()
                {
                    self.cards.push(Card {
                        path: dir.clone(),
                        selected: true,
                        items: None,
                    });
                    spawn_scan(self.tx.clone(), dir, ctx.clone());
                }

                ui.separator();
                ui.label("目标目录");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.target).desired_width(160.0));
                    if ui.button("浏览…").clicked()
                        && let Some(dir) = rfd::FileDialog::new().pick_folder()
                    {
                        self.target = dir.display().to_string();
                    }
                });

                ui.separator();
                ui.checkbox(&mut self.copy_only, "仅复制（保留源文件）");
                ui.checkbox(&mut self.dry_run, "试运行（不修改文件）");

                ui.separator();

                let can_start = !self.busy && self.cards.iter().any(|c| c.selected);
                let start = ui
                    .add_enabled_ui(can_start, |ui| {
                        ui.add_sized(
                            [ui.available_width(), 34.0],
                            egui::Button::new(egui::RichText::new("开始归档").size(16.0)),
                        )
                    })
                    .inner;
                if start.clicked() {
                    self.start(&ctx);
                }

                let (done, total) = self.progress;
                let fraction = if total > 0 {
                    done as f32 / total as f32
                } else {
                    0.0
                };
                ui.add(egui::ProgressBar::new(fraction).show_percentage());
                if let Some(summary) = &self.summary {
                    ui.weak(summary);
                }
            });

        egui::CentralPanel::default().show(ui, |ui| {
            let mut count = 0usize;
            let mut total_size = 0u64;
            let mut pending = false;
            for card in self.cards.iter().filter(|c| c.selected) {
                match &card.items {
                    Some(items) => {
                        count += items.len();
                        total_size += items.iter().map(|i| i.size).sum::<u64>();
                    }
                    None => pending = true,
                }
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.strong("预览");
                ui.weak(format!("{count} 个文件 · {}", human_size(total_size)));
                if pending || self.busy {
                    ui.spinner();
                }
            });
            ui.separator();

            egui::ScrollArea::both()
                .id_salt("preview")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("preview_grid")
                        .num_columns(5)
                        .striped(true)
                        .spacing([16.0, 4.0])
                        .show(ui, |ui| {
                            ui.strong("文件名");
                            ui.strong("大小");
                            ui.strong("拍摄时间");
                            ui.strong("归档目录");
                            ui.strong("XMP");
                            ui.end_row();

                            for card in self.cards.iter().filter(|c| c.selected) {
                                let Some(items) = &card.items else {
                                    continue;
                                };
                                for item in items {
                                    let name = item
                                        .source
                                        .file_name()
                                        .map(|n| n.to_string_lossy().into_owned())
                                        .unwrap_or_default();
                                    ui.label(name);
                                    ui.label(human_size(item.size));
                                    ui.label(&item.captured);
                                    ui.label(&item.folder);
                                    ui.label(if item.has_xmp { "有" } else { "—" });
                                    ui.end_row();
                                }
                            }
                        });
                });
        });
    }
}
