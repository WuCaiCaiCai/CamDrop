use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use eframe::egui;

use camdrop::detector;
use camdrop::organizer::{self, Event, Options};

enum Msg {
    Cards(Vec<PathBuf>),
    Log(String),
    Progress(usize, usize),
    Done(String),
}

struct Card {
    path: PathBuf,
    selected: bool,
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

        let scan_tx = tx.clone();
        let ctx = cc.egui_ctx.clone();
        thread::spawn(move || {
            let cards = detector::detect_cards();
            let _ = scan_tx.send(Msg::Cards(cards));
            ctx.request_repaint();
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
            log: Vec::new(),
        }
    }

    fn rescan(&mut self, ctx: &egui::Context) {
        self.scanning = true;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let cards = detector::detect_cards();
            let _ = tx.send(Msg::Cards(cards));
            ctx.request_repaint();
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
        self.log.clear();
        thread::spawn(move || run_archive(sources, target, opts, tx, ctx));
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
                    self.scanning = false;
                    if self.cards.is_empty() {
                        self.log
                            .push("未检测到存储卡，请用「添加文件夹」手动选择".to_owned());
                    }
                }
                Msg::Log(line) => self.log.push(line),
                Msg::Progress(done, total) => self.progress = (done, total),
                Msg::Done(text) => {
                    self.busy = false;
                    self.log.push(text);
                }
            }
        }
    }
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

impl eframe::App for CamDropApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain_messages();
        let ctx = ui.ctx().clone();

        egui::Panel::top("header").show(ui, |ui| {
            ui.add_space(4.0);
            ui.heading("CamDrop");
            ui.label("自动检测相机存储卡，按拍摄日期归档，并同步 .xmp 侧边栏文件");
            ui.add_space(4.0);
        });

        egui::Panel::bottom("footer").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("目标目录：");
                ui.add(egui::TextEdit::singleline(&mut self.target).desired_width(360.0));
                if ui.button("浏览…").clicked()
                    && let Some(dir) = rfd::FileDialog::new().pick_folder()
                {
                    self.target = dir.display().to_string();
                }
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.copy_only, "仅复制（保留源文件）");
                ui.checkbox(&mut self.dry_run, "试运行（不修改文件）");
                ui.add_space(8.0);
                let can_start = !self.busy && !self.scanning && !self.cards.is_empty();
                if ui
                    .add_enabled(can_start, egui::Button::new("开始归档"))
                    .clicked()
                {
                    self.start(&ctx);
                }
                if self.busy {
                    ui.spinner();
                }
            });
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("源卡");
                if ui.button("刷新").clicked() && !self.busy {
                    self.rescan(&ctx);
                }
                if ui.button("添加文件夹…").clicked()
                    && !self.busy
                    && let Some(dir) = rfd::FileDialog::new().pick_folder()
                {
                    self.cards.push(Card {
                        path: dir,
                        selected: true,
                    });
                }
                if self.scanning {
                    ui.spinner();
                    ui.label("正在扫描…");
                }
            });

            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .max_height(120.0)
                .id_salt("cards")
                .show(ui, |ui| {
                    if self.cards.is_empty() && !self.scanning {
                        ui.weak("（无）");
                    }
                    for card in &mut self.cards {
                        ui.checkbox(&mut card.selected, card.path.display().to_string());
                    }
                });

            ui.separator();

            let (done, total) = self.progress;
            let fraction = if total > 0 {
                done as f32 / total as f32
            } else {
                0.0
            };
            ui.add(
                egui::ProgressBar::new(fraction)
                    .show_percentage()
                    .text(format!("{done}/{total}")),
            );

            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .id_salt("log")
                .show(ui, |ui| {
                    for line in &self.log {
                        ui.monospace(line);
                    }
                });
        });
    }
}
