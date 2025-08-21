// 在 release 构建时，禁用 Windows 上的命令行窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::{egui, App, NativeOptions};
use egui::{Color32, FontData, FontDefinitions, FontFamily, Pos2, Rect, Rounding, Stroke, Vec2};
use rand::Rng;
use std::time::{Duration, Instant};

// --- 游戏逻辑部分 (与之前相同) ---

#[derive(Clone, Copy, PartialEq)]
enum Cell {
    Dead,
    Alive,
}

struct GameOfLife {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl GameOfLife {
    fn new(width: usize, height: usize) -> Self {
        // 确保宽度和高度至少为1，避免除零错误
        let width = width.max(1);
        let height = height.max(1);
        let cells = vec![Cell::Dead; width * height];
        Self { width, height, cells }
    }

    fn get_index(&self, row: usize, col: usize) -> usize {
        row * self.width + col
    }

    fn count_live_neighbors(&self, row: usize, col: usize) -> u8 {
        let mut count = 0;
        for i in [self.height - 1, 0, 1].iter().cloned() {
            for j in [self.width - 1, 0, 1].iter().cloned() {
                if i == 0 && j == 0 {
                    continue;
                }
                let neighbor_row = (row + i) % self.height;
                let neighbor_col = (col + j) % self.width;
                let idx = self.get_index(neighbor_row, neighbor_col);
                if self.cells[idx] == Cell::Alive {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn tick(&mut self) {
        let mut next_cells = self.cells.clone();
        for row in 0..self.height {
            for col in 0..self.width {
                let idx = self.get_index(row, col);
                let live_neighbors = self.count_live_neighbors(row, col);
                let next_state = match (self.cells[idx], live_neighbors) {
                    (Cell::Alive, 2) | (Cell::Alive, 3) => Cell::Alive,
                    (Cell::Dead, 3) => Cell::Alive,
                    _ => Cell::Dead,
                };
                next_cells[idx] = next_state;
            }
        }
        self.cells = next_cells;
    }

    pub fn randomize(&mut self) {
        let mut rng = rand::thread_rng();
        for cell in self.cells.iter_mut() {
            if rng.gen_bool(0.3) {
                *cell = Cell::Alive;
            } else {
                *cell = Cell::Dead;
            }
        }
    }

    pub fn clear(&mut self) {
        for cell in self.cells.iter_mut() {
            *cell = Cell::Dead;
        }
    }
}

// --- UI 应用部分 ---

struct MyApp {
    game: GameOfLife,
    paused: bool,
    cell_size: f32,
    time_step: Duration,
    last_update: Instant,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // --- 新增：字体设置 ---
        setup_custom_fonts(&cc.egui_ctx);
        
        // 初始化一个空的grid，它会在第一次update时被正确调整大小
        let game = GameOfLife::new(1, 1);

        Self {
            game,
            paused: true,
            cell_size: 10.0,
            time_step: Duration::from_millis(100),
            last_update: Instant::now(),
        }
    }
}

impl App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("control_panel").show(ctx, |ui| {
            ui.heading("控制面板");
            ui.separator();

            if ui.button(if self.paused { "▶ 播放" } else { "⏸ 暂停" }).clicked() {
                self.paused = !self.paused;
            }

            if ui.button("下一步").clicked() {
                self.game.tick();
            }
            
            ui.separator();

            if ui.button("随机生成").clicked() {
                self.game.randomize();
            }

            if ui.button("清空").clicked() {
                self.game.clear();
            }
            
            ui.separator();
            
            let mut speed_ms = self.time_step.as_millis() as u32;
            ui.label("演化速度 (ms/步):");
            if ui.add(egui::Slider::new(&mut speed_ms, 20..=1000).logarithmic(true)).changed() {
                self.time_step = Duration::from_millis(speed_ms as u64);
            }

            ui.label("细胞大小:");
            ui.add(egui::Slider::new(&mut self.cell_size, 2.0..=30.0));
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // --- 改进：动态调整网格大小 ---
            let panel_rect = ui.available_rect_before_wrap();
            let new_width = (panel_rect.width() / self.cell_size).max(1.0) as usize;
            let new_height = (panel_rect.height() / self.cell_size).max(1.0) as usize;

            if new_width != self.game.width || new_height != self.game.height {
                // 当窗口大小改变时，创建一个新的、空的grid。旧的图案会丢失。
                self.game = GameOfLife::new(new_width, new_height);
            }
            
            // 使用 interact 来感知拖拽和点击
            let response = ui.interact(panel_rect, ui.id().with("game_canvas"), egui::Sense::drag());
            let painter = ui.painter_at(panel_rect);

            // 绘制网格背景
            let grid_stroke = Stroke::new(1.0, Color32::from_gray(40));
            for i in 0..=self.game.height {
                let y = panel_rect.min.y + i as f32 * self.cell_size;
                painter.line_segment([Pos2::new(panel_rect.min.x, y), Pos2::new(panel_rect.max.x, y)], grid_stroke);
            }
            for i in 0..=self.game.width {
                let x = panel_rect.min.x + i as f32 * self.cell_size;
                painter.line_segment([Pos2::new(x, panel_rect.min.y), Pos2::new(x, panel_rect.max.y)], grid_stroke);
            }
            
            // 绘制存活的细胞
            for row in 0..self.game.height {
                for col in 0..self.game.width {
                    let idx = self.game.get_index(row, col);
                    if self.game.cells[idx] == Cell::Alive {
                        let cell_top_left = panel_rect.min + Vec2::new(col as f32 * self.cell_size, row as f32 * self.cell_size);
                        let cell_rect = Rect::from_min_size(cell_top_left, Vec2::splat(self.cell_size));
                        painter.rect_filled(cell_rect, Rounding::same(0.0), Color32::LIGHT_GREEN);
                    }
                }
            }

            // --- 改进：拖拽绘制/擦除逻辑 ---
            if response.is_pointer_button_down_on() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let col = ((pos.x - panel_rect.min.x) / self.cell_size) as usize;
                    let row = ((pos.y - panel_rect.min.y) / self.cell_size) as usize;
                    if row < self.game.height && col < self.game.width {
                        let idx = self.game.get_index(row, col);
                        // 左键拖拽 -> 绘制 (设为存活)
                        if ui.input(|i| i.pointer.primary_down()) {
                            self.game.cells[idx] = Cell::Alive;
                        } 
                        // 右键拖拽 -> 擦除 (设为死亡)
                        else if ui.input(|i| i.pointer.secondary_down()) {
                            self.game.cells[idx] = Cell::Dead;
                        }
                    }
                }
            }
        });

        if !self.paused && self.last_update.elapsed() >= self.time_step {
            self.game.tick();
            self.last_update = Instant::now();
        }
        
        ctx.request_repaint();
    }
}

// --- 新增：加载自定义字体的辅助函数 ---
fn setup_custom_fonts(ctx: &egui::Context) {
    // 从一个 .ttf 或 .otf 文件开始
    let mut fonts = FontDefinitions::default();

    // 安装我们的自定义字体。
    // `include_bytes!` 会在编译时将字体文件直接嵌入到你的可执行文件中。
    fonts.font_data.insert(
        "my_font".to_owned(),
        FontData::from_static(include_bytes!("../assets/SmileySans-Oblique.otf")),
    );

    // 将我们的字体设为 Proportional (常规) 和 Monospace (等宽) 字族的第一选择。
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "my_font".to_owned());

    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "my_font".to_owned());

    // 告诉 egui 使用我们新的字体定义
    ctx.set_fonts(fonts);
}


// --- 程序主入口 (与之前相同) ---

fn main() -> Result<(), eframe::Error> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "生命游戏 (Rust + egui)",
        options,
        Box::new(|cc| Box::new(MyApp::new(cc))),
    )
}