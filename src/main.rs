#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod github;

use eframe::egui::{self, Color32, FontId, RichText, Sense, Stroke, Vec2};
use github::{Repo, User};
use std::collections::{BTreeMap, HashMap};
use std::sync::mpsc::{channel, Receiver, Sender};

// ---- Renk paleti (özgün "gece vitrini" teması) ----
const BG: Color32 = Color32::from_rgb(11, 13, 20);
const PANEL: Color32 = Color32::from_rgb(18, 21, 31);
const CARD: Color32 = Color32::from_rgb(24, 28, 41);
const BORDER: Color32 = Color32::from_rgb(40, 46, 66);
const ACCENT: Color32 = Color32::from_rgb(124, 92, 255);
const ACCENT2: Color32 = Color32::from_rgb(34, 211, 238);
const TEXT: Color32 = Color32::from_rgb(232, 235, 245);
const MUTED: Color32 = Color32::from_rgb(130, 138, 165);

enum Msg {
    Loaded(Result<(User, Vec<Repo>), String>),
    Langs(String, BTreeMap<String, u64>),
}

#[derive(Clone, Copy, PartialEq)]
enum Sort {
    Updated,
    Stars,
    Name,
}

struct App {
    token: String,
    token_input: String,
    user: Option<User>,
    repos: Vec<Repo>,
    loading: bool,
    error: Option<String>,
    query: String,
    lang: Option<String>,
    sort: Sort,
    show_forks: bool,
    show_archived: bool,
    selected: Option<usize>,
    langs: HashMap<String, BTreeMap<String, u64>>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

fn token_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("vitrin").join("token"))
}

fn lang_color(l: &str) -> Color32 {
    match l {
        "Rust" => Color32::from_rgb(222, 165, 132),
        "Python" => Color32::from_rgb(55, 118, 171),
        "JavaScript" => Color32::from_rgb(241, 224, 90),
        "TypeScript" => Color32::from_rgb(49, 120, 198),
        "Go" => Color32::from_rgb(0, 173, 216),
        "C" => Color32::from_rgb(120, 120, 120),
        "C++" => Color32::from_rgb(243, 75, 125),
        "C#" => Color32::from_rgb(23, 134, 0),
        "Java" => Color32::from_rgb(176, 114, 25),
        "Kotlin" => Color32::from_rgb(169, 123, 255),
        "Swift" => Color32::from_rgb(240, 81, 56),
        "HTML" => Color32::from_rgb(227, 76, 38),
        "CSS" => Color32::from_rgb(86, 61, 124),
        "Shell" => Color32::from_rgb(137, 224, 81),
        _ => {
            // Bilinmeyen diller için addan türetilen kararlı renk
            let h = l.bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
            Color32::from_rgb(90 + (h % 140) as u8, 90 + ((h >> 8) % 140) as u8, 90 + ((h >> 16) % 140) as u8)
        }
    }
}

fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n).collect::<String>() + "…"
    }
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        let mut v = egui::Visuals::dark();
        v.panel_fill = PANEL;
        v.window_fill = PANEL;
        v.extreme_bg_color = BG;
        v.override_text_color = Some(TEXT);
        v.selection.bg_fill = ACCENT;
        v.widgets.inactive.bg_fill = CARD;
        v.widgets.inactive.weak_bg_fill = CARD;
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(38, 44, 66);
        v.widgets.active.weak_bg_fill = ACCENT;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
        cc.egui_ctx.set_visuals(v);

        let (tx, rx) = channel();
        let token = token_path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default().trim().to_string();
        let mut app = App {
            token_input: String::new(),
            token,
            user: None,
            repos: vec![],
            loading: false,
            error: None,
            query: String::new(),
            lang: None,
            sort: Sort::Updated,
            show_forks: true,
            show_archived: true,
            selected: None,
            langs: HashMap::new(),
            tx,
            rx,
        };
        if !app.token.is_empty() {
            app.load(&cc.egui_ctx);
        }
        app
    }

    fn load(&mut self, ctx: &egui::Context) {
        self.loading = true;
        self.error = None;
        let (tx, token, ctx) = (self.tx.clone(), self.token.clone(), ctx.clone());
        std::thread::spawn(move || {
            let r: Result<_, String> = (|| Ok((github::fetch_user(&token)?, github::fetch_repos(&token)?)))();
            let _ = tx.send(Msg::Loaded(r));
            ctx.request_repaint();
        });
    }

    fn select(&mut self, i: usize, ctx: &egui::Context) {
        self.selected = Some(i);
        let full = self.repos[i].full_name.clone();
        if self.langs.contains_key(&full) {
            return;
        }
        let (tx, token, ctx) = (self.tx.clone(), self.token.clone(), ctx.clone());
        std::thread::spawn(move || {
            if let Ok(l) = github::fetch_languages(&token, &full) {
                let _ = tx.send(Msg::Langs(full, l));
                ctx.request_repaint();
            }
        });
    }

    fn logout(&mut self) {
        if let Some(p) = token_path() {
            let _ = std::fs::remove_file(p);
        }
        self.token.clear();
        self.user = None;
        self.repos.clear();
        self.selected = None;
    }

    fn filtered(&self) -> Vec<usize> {
        let q = self.query.to_lowercase();
        let mut v: Vec<usize> = (0..self.repos.len())
            .filter(|&i| {
                let r = &self.repos[i];
                (self.show_forks || !r.fork)
                    && (self.show_archived || !r.archived)
                    && self.lang.as_ref().map_or(true, |l| r.language.as_ref() == Some(l))
                    && (q.is_empty()
                        || r.full_name.to_lowercase().contains(&q)
                        || r.description.as_deref().unwrap_or("").to_lowercase().contains(&q)
                        || r.topics.iter().any(|t| t.contains(&q)))
            })
            .collect();
        match self.sort {
            Sort::Updated => v.sort_by(|&a, &b| self.repos[b].updated_at.cmp(&self.repos[a].updated_at)),
            Sort::Stars => v.sort_by_key(|&i| std::cmp::Reverse(self.repos[i].stargazers_count)),
            Sort::Name => v.sort_by_key(|&i| self.repos[i].name.to_lowercase()),
        }
        v
    }

    fn login_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(egui::Frame::none().fill(BG)).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.28);
                ui.label(RichText::new("VİTRİN").font(FontId::proportional(54.0)).color(ACCENT).strong());
                ui.label(RichText::new("GitHub projelerinizin canlı vitrini").color(MUTED).size(16.0));
                ui.add_space(28.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.token_input)
                        .password(true)
                        .hint_text("GitHub Personal Access Token (repo okuma yetkisi)")
                        .desired_width(380.0)
                        .margin(Vec2::new(12.0, 10.0)),
                );
                ui.add_space(10.0);
                let btn = egui::Button::new(RichText::new("Bağlan").strong().size(15.0)).fill(ACCENT).min_size(Vec2::new(380.0, 38.0));
                if ui.add_enabled(!self.token_input.trim().is_empty() && !self.loading, btn).clicked() {
                    self.token = self.token_input.trim().to_string();
                    if let Some(p) = token_path() {
                        if let Some(d) = p.parent() {
                            let _ = std::fs::create_dir_all(d);
                        }
                        let _ = std::fs::write(p, &self.token);
                    }
                    self.token_input.clear();
                    self.load(ctx);
                }
                if self.loading {
                    ui.add_space(12.0);
                    ui.spinner();
                }
                if let Some(e) = &self.error {
                    ui.add_space(12.0);
                    ui.colored_label(Color32::from_rgb(255, 110, 120), e);
                }
                ui.add_space(18.0);
                ui.hyperlink_to(
                    RichText::new("Token oluştur → github.com/settings/tokens").color(ACCENT2),
                    "https://github.com/settings/tokens",
                );
            });
        });
    }

    fn card(&self, ui: &mut egui::Ui, i: usize) -> bool {
        let r = &self.repos[i];
        let sel = self.selected == Some(i);
        let out = egui::Frame::none()
            .fill(CARD)
            .rounding(12.0)
            .inner_margin(14.0)
            .stroke(Stroke::new(1.0, if sel { ACCENT } else { BORDER }))
            .show(ui, |ui| {
                ui.set_width(292.0);
                ui.set_min_height(128.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(clip(&r.name, 26)).size(17.0).strong());
                    if r.private {
                        ui.label(RichText::new("özel").small().color(ACCENT2));
                    }
                    if r.archived {
                        ui.label(RichText::new("arşiv").small().color(MUTED));
                    }
                    if r.fork {
                        ui.label(RichText::new("fork").small().color(MUTED));
                    }
                });
                ui.label(RichText::new(r.owner.login.as_str()).small().color(MUTED));
                ui.add_space(4.0);
                ui.label(RichText::new(clip(r.description.as_deref().unwrap_or("Açıklama yok"), 105)).color(Color32::from_rgb(190, 196, 215)));
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    for t in r.topics.iter().take(3) {
                        ui.label(RichText::new(format!("#{t}")).small().color(ACCENT));
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if let Some(l) = &r.language {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                        ui.painter().circle_filled(rect.center(), 5.0, lang_color(l));
                        ui.label(RichText::new(l).small());
                    }
                    ui.label(RichText::new(format!("★ {}", r.stargazers_count)).small().color(MUTED));
                    ui.label(RichText::new(format!("fork {}", r.forks_count)).small().color(MUTED));
                });
            });
        let resp = ui.interact(out.response.rect, ui.id().with(("card", i)), Sense::click());
        if resp.hovered() {
            ui.painter().rect_stroke(out.response.rect, 12.0, Stroke::new(1.5, ACCENT2));
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        resp.clicked()
    }

    fn details(&self, ctx: &egui::Context) -> bool {
        let mut close = false;
        let Some(i) = self.selected else { return false };
        let r = &self.repos[i];
        egui::SidePanel::right("detail").exact_width(340.0).frame(egui::Frame::none().fill(PANEL).inner_margin(18.0)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&r.name).size(22.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").clicked() {
                        close = true;
                    }
                });
            });
            ui.label(RichText::new(&r.full_name).color(MUTED));
            ui.add_space(10.0);
            ui.label(r.description.as_deref().unwrap_or("Açıklama yok"));
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                for (k, v) in [("Yıldız", r.stargazers_count), ("Fork", r.forks_count), ("Issue", r.open_issues_count)] {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(v.to_string()).size(22.0).color(ACCENT2).strong());
                        ui.label(RichText::new(k).small().color(MUTED));
                    });
                    ui.add_space(14.0);
                }
            });
            ui.add_space(14.0);
            ui.label(RichText::new("Diller").strong());
            match self.langs.get(&r.full_name) {
                Some(l) if !l.is_empty() => {
                    let total: u64 = l.values().sum();
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 10.0), Sense::hover());
                    let mut x = rect.left();
                    for (name, b) in l {
                        let w = rect.width() * (*b as f32 / total as f32);
                        ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(w, 10.0)), 0.0, lang_color(name));
                        x += w;
                    }
                    ui.add_space(6.0);
                    for (name, b) in l {
                        ui.label(RichText::new(format!("● {name}  {:.1}%", *b as f64 * 100.0 / total as f64)).color(lang_color(name)).small());
                    }
                }
                Some(_) => {
                    ui.label(RichText::new("Veri yok").color(MUTED));
                }
                None => {
                    ui.spinner();
                }
            }
            ui.add_space(14.0);
            ui.label(RichText::new(format!("Son güncelleme: {}", r.updated_at.get(..10).unwrap_or(""))).small().color(MUTED));
            ui.add_space(12.0);
            if ui.add(egui::Button::new(RichText::new("GitHub'da aç ↗").strong()).fill(ACCENT).min_size(Vec2::new(ui.available_width(), 36.0))).clicked() {
                ctx.open_url(egui::OpenUrl::new_tab(&r.html_url));
            }
        });
        close
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Loaded(Ok((u, r))) => {
                    self.user = Some(u);
                    self.repos = r;
                    self.loading = false;
                }
                Msg::Loaded(Err(e)) => {
                    self.error = Some(e);
                    self.loading = false;
                    self.token.clear();
                }
                Msg::Langs(k, l) => {
                    self.langs.insert(k, l);
                }
            }
        }
        if self.token.is_empty() || self.user.is_none() {
            return self.login_screen(ctx);
        }

        let mut refresh = false;
        egui::TopBottomPanel::top("top").frame(egui::Frame::none().fill(PANEL).inner_margin(12.0)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("VİTRİN").size(22.0).color(ACCENT).strong());
                ui.add_space(16.0);
                ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("🔍 Proje, açıklama veya etiket ara…").desired_width(320.0));
                egui::ComboBox::from_id_source("sort")
                    .selected_text(match self.sort { Sort::Updated => "Son güncellenen", Sort::Stars => "En çok yıldız", Sort::Name => "Ada göre" })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.sort, Sort::Updated, "Son güncellenen");
                        ui.selectable_value(&mut self.sort, Sort::Stars, "En çok yıldız");
                        ui.selectable_value(&mut self.sort, Sort::Name, "Ada göre");
                    });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Çıkış").clicked() {
                        self.logout();
                    }
                    if ui.add_enabled(!self.loading, egui::Button::new("⟳ Yenile")).clicked() {
                        refresh = true;
                    }
                    if self.loading {
                        ui.spinner();
                    }
                });
            });
        });
        if refresh {
            self.load(ctx);
        }

        // Dil sayıları
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for r in &self.repos {
            if let Some(l) = &r.language {
                *counts.entry(l.as_str()).or_default() += 1;
            }
        }
        let mut counts: Vec<_> = counts.into_iter().collect();
        counts.sort_by_key(|c| std::cmp::Reverse(c.1));

        let mut new_lang: Option<Option<String>> = None;
        egui::SidePanel::left("side").exact_width(220.0).frame(egui::Frame::none().fill(PANEL).inner_margin(14.0)).show(ctx, |ui| {
            if let Some(u) = &self.user {
                ui.label(RichText::new(u.name.as_deref().unwrap_or(&u.login)).size(18.0).strong());
                ui.label(RichText::new(format!("@{}", u.login)).color(MUTED));
                ui.label(RichText::new(format!("{} proje", self.repos.len())).color(ACCENT2));
            }
            ui.add_space(14.0);
            ui.label(RichText::new("DİLLER").small().color(MUTED));
            if ui.selectable_label(self.lang.is_none(), "Tümü").clicked() {
                new_lang = Some(None);
            }
            egui::ScrollArea::vertical().max_height((ui.available_height() - 90.0).max(60.0)).show(ui, |ui| {
                for (l, n) in &counts {
                    let on = self.lang.as_deref() == Some(*l);
                    if ui.selectable_label(on, RichText::new(format!("● {l}  ({n})")).color(lang_color(l))).clicked() {
                        new_lang = Some(if on { None } else { Some(l.to_string()) });
                    }
                }
            });
            ui.add_space(10.0);
            ui.checkbox(&mut self.show_forks, "Fork'ları göster");
            ui.checkbox(&mut self.show_archived, "Arşivlenenleri göster");
        });
        if let Some(l) = new_lang {
            self.lang = l;
        }

        if self.details(ctx) {
            self.selected = None;
        }

        let idx = self.filtered();
        let mut clicked = None;
        egui::CentralPanel::default().frame(egui::Frame::none().fill(BG).inner_margin(18.0)).show(ctx, |ui| {
            ui.label(RichText::new(format!("{} sonuç", idx.len())).color(MUTED));
            ui.add_space(8.0);
            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(14.0);
                    for &i in &idx {
                        if self.card(ui, i) {
                            clicked = Some(i);
                        }
                    }
                });
            });
        });
        if let Some(i) = clicked {
            self.select(i, ctx);
        }
    }
}

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]).with_min_inner_size([900.0, 560.0]).with_title("Vitrin — GitHub Projeleri"),
        ..Default::default()
    };
    eframe::run_native("Vitrin", opts, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
