use crate::audio::playback::PlaybackManager;
use crate::audio::AudioData;
use crate::effects::EffectsState;
use crate::ui::UIState;
use eframe::egui::{Context, ViewportBuilder};
use eframe::{egui, App, Frame, NativeOptions};
use egui::Vec2;
use std::sync::{Arc, Mutex};

mod audio;
mod effects;
mod ui;

struct AudioEditor {
    audio_data: Arc<Mutex<AudioData>>,
    ui_state: UIState,
    effects_state: EffectsState,
    playback_manager: PlaybackManager,
}

impl Default for AudioEditor {
    fn default() -> Self {
        Self {
            audio_data: Arc::new(Mutex::new(AudioData::default())),
            ui_state: UIState::default(),
            effects_state: EffectsState::default(),
            playback_manager: PlaybackManager::new(),
        }
    }
}

impl App for AudioEditor {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        // Update playback state in UI
        self.playback_manager.update_ui(&self.audio_data);

        ui::render_ui(
            ctx,
            &mut self.ui_state,
            &self.audio_data,
            &mut self.effects_state,
            &mut self.playback_manager,
        );
    }
}

fn main() {
    let options = NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size(Vec2::new(1280.0, 720.0)), // Définit la taille de la fenêtre
        ..Default::default()
    };

    eframe::run_native(
        "Éditeur de son",
        options,
        Box::new(|_cc| Ok(Box::new(AudioEditor::default()))), // Ajout de Ok()
    )
    .unwrap();
}
