use eframe::{egui, App, Frame, NativeOptions};
use egui::Vec2;
use std::sync::Arc;
use std::sync::Mutex;
use crate::audio::AudioData;
use crate::ui::UIState;
use crate::effects::EffectsState;

mod audio;
mod effects;
mod ui;

struct AudioEditor {
    audio_data: Arc<Mutex<AudioData>>,
    ui_state: UIState,
    effects_state: EffectsState,
}

impl Default for AudioEditor {
    fn default() -> Self {
        Self {
            audio_data: Arc::new(Mutex::new(AudioData::default())),
            ui_state: UIState::default(),
            effects_state: EffectsState::default(),
        }
    }
}

impl App for AudioEditor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        ui::render_ui(ctx, &mut self.ui_state, &self.audio_data, &mut self.effects_state);
    }
}

fn main() {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(Vec2::new(1280.0, 720.0)), // Définit la taille de la fenêtre
        ..Default::default()
    };

    eframe::run_native(
        "Éditeur de son",
        options,
        Box::new(|_cc| Ok(Box::new(AudioEditor::default()))), // Ajout de Ok()
    ).unwrap();
}
