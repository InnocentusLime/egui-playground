use eframe::egui;
use egui::{Button, Color32, Label, TextEdit, Vec2, Vec2b, Widget, vec2};

use crate::sequencer::{Clips, Sequencer, SequencerState, TimelineTf};

mod sequencer;

///////////////////////////////////////////////////////

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        native_options,
        Box::new(|cc| Ok(Box::new(MyEguiApp::new(cc)))),
    )
    .unwrap();
}

struct MyEguiApp {
    sequencer_state: SequencerState,
    tf: TimelineTf,
    clips: Clips,
    cursor_pos: u32,
    clip_label: String,
    selected_clip: Option<u32>,
    selected_track: Option<u32>,
}

impl MyEguiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        let mut clips = Clips::new();
        clips.add_track("red events".into(), Color32::RED);
        clips.add_track("yellow events".into(),Color32::YELLOW);
        clips.add_clip(0, "lol".into(), 10, 30);
        clips.add_clip(0, "some event".into(), 60, 60);
        clips.add_clip(1, "lol2".into(), 20, 40);

        Self {
            sequencer_state: SequencerState::Idle,
            clips,
            cursor_pos: 0,
            clip_label: String::new(),
            selected_clip: None,
            selected_track: None,
            tf: TimelineTf {
                zoom: 1.0,
                pan: 0.0,
            },
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::Window::new("My Window")
            .resizable(Vec2b::new(true, true))
            .show(ctx, |ui| {
                ui.group(|ui| {
                    ui.set_min_size(vec2(200.0, 150.0));
                    if let Some(clip) = self.selected_clip {
                        match self.clips.get(clip) {
                            None => self.selected_clip = None,
                            Some(clip) => {
                                ui.label(clip.label.clone());
                                ui.label(format!("Track: {}", clip.track_id));
                                ui.label(format!("Pos: {}", clip.pos));
                                ui.label(format!("Length: {}", clip.len));
                            }
                        }
                    } else {
                        ui.add_enabled(false, Label::new("No clip selected"));
                    }
                });

                ui.horizontal(|ui| {
                    TextEdit::singleline(&mut self.clip_label)
                        .desired_width(150.0)
                        .ui(ui);

                    let resp =
                        ui.add_enabled(self.selected_track.is_some(), Button::new("add clip"));
                    if let Some(track_id) = self.selected_track {
                        if resp.clicked() {
                            self.clips.add_clip(
                                track_id,
                                self.clip_label.as_str().into(),
                                self.cursor_pos,
                                30,
                            );
                        }
                    }

                    let resp =
                        ui.add_enabled(self.selected_clip.is_some(), Button::new("delete clip"));
                    if let Some(idx) = self.selected_clip {
                        if resp.clicked() {
                            self.clips.delete_clip(idx);
                        }
                    }
                });

                Sequencer {
                    state: &mut self.sequencer_state,
                    clips: &mut self.clips,
                    cursor_pos: &mut self.cursor_pos,
                    size: Vec2::new(500.0, 200.0),
                    tf: &mut self.tf,
                    selected_clip: &mut self.selected_clip,
                    selected_track: &mut self.selected_track,
                }
                .ui(ui);
            });
    }
}
