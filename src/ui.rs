use crate::audio::AudioData;
use crate::effects::{self, EffectsState};
use eframe::egui::{self, Align, Align2, CentralPanel, Color32, Context, Layout, Stroke, TopBottomPanel, Ui, Vec2, Visuals, Window};
use rfd::FileDialog;
use std::sync::{Arc, Mutex};

const MESSAGE_TIMER: f32 = 3.0;

pub struct UIState {
    selected_tab: Tab,
    error_message: Option<String>,
    success_message: Option<String>,
    current_message_timer: f32,
}

#[derive(PartialEq)]
pub enum Tab {
    FILE,
    EFFECTS,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            selected_tab: Tab::FILE,
            error_message: None,
            success_message: None,
            current_message_timer: 0.0,
        }
    }
}

pub fn render_ui(
    ctx: &Context,
    ui_state: &mut UIState,
    audio_data: &Arc<Mutex<AudioData>>,
    effects_state: &mut EffectsState,
) {
    ctx.set_visuals(Visuals::dark());

    // Handle message timers
    if ui_state.error_message.is_some() || ui_state.success_message.is_some() {
        ui_state.current_message_timer -= ctx.input(|is| is.unstable_dt);
        if ui_state.current_message_timer <= 0.0 {
            ui_state.error_message = None;
            ui_state.success_message = None;
        }
    }

    // Top panel with tabs
    TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut ui_state.selected_tab, Tab::FILE, "Fichier");
            ui.selectable_value(&mut ui_state.selected_tab, Tab::EFFECTS, "Effets");

            // Right-aligned status
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Ok(audio) = audio_data.lock() {
                    if audio.samples.is_empty() {
                        ui.label("Aucun audio chargé");
                    } else {
                        let duration_str = format!("{:.1}s", audio.duration_seconds);
                        ui.label(format!("{}Hz | {}ch | {}", audio.sample_rate, audio.channels, duration_str));

                        if audio.modified { ui.label("Modifié"); }
                    }
                }
            });
        });
    });

    // Central area
    CentralPanel::default().show(ctx, |ui| {
        match ui_state.selected_tab {
            Tab::FILE => render_file_tab(ui, audio_data, ui_state),
            Tab::EFFECTS => render_effects_tab(ui, audio_data, effects_state, ui_state),
        }
    });

    // Render modals
    render_modals(ctx, audio_data, effects_state, ui_state);

    // Show messages
    if let Some(error) = &ui_state.error_message {
        Window::new("Erreur")
            .anchor(Align2::CENTER_BOTTOM, [0.0, -10.0])
            .collapsible(false)
            .show(ctx, |ui| {
                ui.colored_label(Color32::RED, error);
            });
    }

    if let Some(success) = &ui_state.success_message {
        Window::new("Succès")
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -10.0])
            .collapsible(false)
            .show(ctx, |ui| {
                ui.colored_label(Color32::GREEN, success);
            });
    }
}

fn render_file_tab(ui: &mut Ui, audio_data: &Arc<Mutex<AudioData>>, ui_state: &mut UIState) {
    ui.vertical_centered(|ui| {
        ui.add_space(20.0);

        if ui.button("Importer un fichier audio").clicked() {
            if let Some(path) = FileDialog::new()
                .add_filter("WAV Files", &["wav"])
                .pick_file() {

                let mut audio = audio_data.lock().unwrap();
                match audio.load_file(path) {
                    Ok(_) => {
                        ui_state.success_message = Some("Fichier chargé avec succès".to_string());
                        ui_state.current_message_timer = MESSAGE_TIMER;
                    },
                    Err(e) => {
                        ui_state.error_message = Some(format!("Erreur lors du chargement du fichier: {}", e));
                        ui_state.current_message_timer = MESSAGE_TIMER;
                    },
                }
            }
        }

        ui.add_space(10.0);

        if ui.button("Sauvegarder").clicked() {
            let audio = audio_data.lock().unwrap();
            if audio.file_path.is_none() {
                drop(audio);
                ui_state.error_message = Some("Aucun fichier chargé".to_string());
                ui_state.current_message_timer = MESSAGE_TIMER;
            } else {
                match audio.save_file(None) {
                    Ok(_) => {
                        ui_state.success_message = Some("File saved successfully".to_string());
                        ui_state.current_message_timer = 3.0;
                    },
                    Err(e) => {
                        ui_state.error_message = Some(format!("Error saving file: {}", e));
                        ui_state.current_message_timer = 3.0;
                    },
                }
            }
        }

        ui.add_space(10.0);

        if ui.button("Save As...").clicked() {
            let audio = audio_data.lock().unwrap();
            if audio.samples.is_empty() {
                drop(audio);
                ui_state.error_message = Some("No audio loaded".to_string());
                ui_state.current_message_timer = 3.0;
            } else {
                if let Some(path) = FileDialog::new()
                    .add_filter("WAV Files", &["wav"])
                    .save_file() {

                    match audio.save_file(Some(path)) {
                        Ok(_) => {
                            ui_state.success_message = Some("File saved successfully".to_string());
                            ui_state.current_message_timer = 3.0;
                        },
                        Err(e) => {
                            ui_state.error_message = Some(format!("Error saving file: {}", e));
                            ui_state.current_message_timer = 3.0;
                        },
                    }
                }
            }
        }
    });

    ui.add_space(20.0);

    // Audio waveform visualization and playback controls
    let mut audio = audio_data.lock().unwrap();
    if !audio.samples.is_empty() {
        // Waveform visualization
        let waveform_height = 200.0;

        ui.label("Audio Waveform:");
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), waveform_height), egui::Sense::click_and_drag());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);

            // Draw background
            painter.rect_filled(rect, 0.0, Color32::from_rgb(30, 30, 30));

            // Draw waveform
            let points = audio.visualization_data.len();
            let width_per_point = rect.width() / points as f32;

            for i in 0..points.saturating_sub(1) {
                let x1 = rect.left() + i as f32 * width_per_point;
                let x2 = rect.left() + (i + 1) as f32 * width_per_point;

                let y1 = rect.center().y - (audio.visualization_data[i] * waveform_height / 2.0);
                let y2 = rect.center().y - (audio.visualization_data[i + 1] * waveform_height / 2.0);

                painter.line_segment(
                    [egui::pos2(x1, y1), egui::pos2(x2, y2)],
                    Stroke::new(1.5, Color32::from_rgb(100, 200, 255)),
                );
            }

            // Draw playback position
            if audio.samples.len() > 0 {
                let position_ratio = audio.playback_position as f32 / audio.samples.len() as f32;
                let x_pos = rect.left() + rect.width() * position_ratio;

                painter.line_segment(
                    [egui::pos2(x_pos, rect.top()), egui::pos2(x_pos, rect.bottom())],
                    Stroke::new(2.0, Color32::RED),
                );
            }

            // Handle clicks on waveform to change playback position
            if rect.contains(ui.input(|i| i.pointer.interact_pos()).unwrap_or_default()) && ui.input(|i| i.pointer.primary_clicked()) {
                if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                    let click_x_ratio = (pos.x - rect.left()) / rect.width();
                    audio.playback_position = (click_x_ratio * audio.samples.len() as f32) as usize;
                }
            }
        }

        // Playback controls
        ui.horizontal(|ui| {
            if ui.button(if audio.playing { "⏸ Pause" } else { "▶ Play" }).clicked() {
                audio.playing = !audio.playing;
            }

            if ui.button("⏹ Stop").clicked() {
                audio.playing = false;
                audio.playback_position = 0;
            }

            // Playback position slider
            let mut position_seconds = audio.playback_position as f32 / (audio.sample_rate as f32 * audio.channels as f32);
            if ui.add(egui::Slider::new(&mut position_seconds, 0.0..=audio.duration_seconds)
                .text("Position")
                .show_value(true)
            ).changed() {
                audio.playback_position = (position_seconds * audio.sample_rate as f32 * audio.channels as f32) as usize;
            }
        });
    }
}

fn render_effects_tab(ui: &mut Ui, audio_data: &Arc<Mutex<AudioData>>, effects_state: &mut EffectsState, ui_state: &mut UIState) {
    let audio_loaded = audio_data.lock().unwrap().samples.len() > 0;

    if !audio_loaded {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.label("No audio file loaded. Please import an audio file first.");
        });
        return;
    }

    ui.vertical(|ui| {
        ui.add_space(20.0);
        ui.heading("Audio Effects");
        ui.add_space(20.0);

        // Amplification
        ui.horizontal(|ui| {
            ui.label("Amplification: ");
            if ui.button("Configure").clicked() {
                effects_state.show_amplify_modal = true;
            }
            if ui.button("Apply").clicked() {
                let mut audio = audio_data.lock().unwrap();
                effects::amplify(&mut audio.samples, effects_state.amplify_gain);
                audio.modified = true;
                audio.generate_visualization_data();

                ui_state.success_message = Some(format!("Applied amplification with gain: {:.2}", effects_state.amplify_gain));
                ui_state.current_message_timer = 3.0;
            }
        });

        ui.add_space(10.0);

        // Anti-distortion
        ui.horizontal(|ui| {
            ui.label("Anti-distortion: ");
            if ui.button("Configure").clicked() {
                effects_state.show_anti_distortion_modal = true;
            }
            if ui.button("Apply").clicked() {
                let mut audio = audio_data.lock().unwrap();
                effects::anti_distortion(&mut audio.samples, effects_state.anti_distortion_threshold);
                audio.modified = true;
                audio.generate_visualization_data();

                ui_state.success_message = Some(format!("Applied anti-distortion with threshold: {:.2}", effects_state.anti_distortion_threshold));
                ui_state.current_message_timer = 3.0;
            }
        });

        ui.add_space(10.0);

        // Noise reduction
        ui.horizontal(|ui| {
            ui.label("Noise reduction: ");
            if ui.button("Configure").clicked() {
                effects_state.show_noise_reduction_modal = true;
            }
            if ui.button("Apply").clicked() {
                let mut audio = audio_data.lock().unwrap();
                effects::noise_reduction(&mut audio.samples, effects_state.noise_reduction_threshold);
                audio.modified = true;
                audio.generate_visualization_data();

                ui_state.success_message = Some(format!("Applied noise reduction with threshold: {:.2}", effects_state.noise_reduction_threshold));
                ui_state.current_message_timer = 3.0;
            }
        });

        ui.add_space(10.0);

        // Audio statistics
        ui.collapsing("Audio Statistics", |ui| {
            let audio = audio_data.lock().unwrap();
            let peak = effects::compute_peak(&audio.samples);
            let rms = effects::compute_rms(&audio.samples);

            ui.label(format!("Peak amplitude: {:.4}", peak));
            ui.label(format!("RMS level: {:.4}", rms));
            ui.label(format!("Crest factor: {:.2} dB", 20.0 * (peak / rms).log10()));
        });
    });
}

fn render_modals(
    ctx: &egui::Context,
    audio_data: &Arc<Mutex<AudioData>>,
    effects_state: &mut EffectsState,
    ui_state: &mut UIState,
) {
    // Amplification modal
    if effects_state.show_amplify_modal {
        egui::Window::new("Amplification Settings")
            .fixed_size([300.0, 150.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Adjust amplification gain:");
                ui.add(egui::Slider::new(&mut effects_state.amplify_gain, 0.1..=5.0).text("Gain"));

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        let mut audio = audio_data.lock().unwrap();
                        effects::amplify(&mut audio.samples, effects_state.amplify_gain);
                        audio.modified = true;
                        audio.generate_visualization_data();

                        effects_state.show_amplify_modal = false;
                        ui_state.success_message = Some(format!("Applied amplification with gain: {:.2}", effects_state.amplify_gain));
                        ui_state.current_message_timer = 3.0;
                    }

                    if ui.button("Cancel").clicked() {
                        effects_state.show_amplify_modal = false;
                    }
                });
            });
    }

    // Anti-distortion modal
    if effects_state.show_anti_distortion_modal {
        egui::Window::new("Anti-distortion Settings")
            .fixed_size([300.0, 150.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Set the threshold level for limiting:");
                ui.add(egui::Slider::new(&mut effects_state.anti_distortion_threshold, 0.1..=1.0).text("Threshold"));

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        let mut audio = audio_data.lock().unwrap();
                        effects::anti_distortion(&mut audio.samples, effects_state.anti_distortion_threshold);
                        audio.modified = true;
                        audio.generate_visualization_data();

                        effects_state.show_anti_distortion_modal = false;
                        ui_state.success_message = Some(format!("Applied anti-distortion with threshold: {:.2}", effects_state.anti_distortion_threshold));
                        ui_state.current_message_timer = 3.0;
                    }

                    if ui.button("Cancel").clicked() {
                        effects_state.show_anti_distortion_modal = false;
                    }
                });
            });
    }

    // Noise reduction modal
    if effects_state.show_noise_reduction_modal {
        Window::new("Noise Reduction Settings")
            .fixed_size([300.0, 150.0])
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Set noise threshold level:");
                ui.add(egui::Slider::new(&mut effects_state.noise_reduction_threshold, 0.001..=0.1).text("Threshold"));

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        let mut audio = audio_data.lock().unwrap();
                        effects::noise_reduction(&mut audio.samples, effects_state.noise_reduction_threshold);
                        audio.modified = true;
                        audio.generate_visualization_data();

                        effects_state.show_noise_reduction_modal = false;
                        ui_state.success_message = Some(format!("Applied noise reduction with threshold: {:.2}", effects_state.noise_reduction_threshold));
                        ui_state.current_message_timer = 3.0;
                    }

                    if ui.button("Cancel").clicked() {
                        effects_state.show_noise_reduction_modal = false;
                    }
                });
            });
    }
}
