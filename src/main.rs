use eframe::egui;
use egui::{
    pos2, vec2, Color32, DragValue, Rect, Sense, Stroke, TextStyle, Ui, Vec2, Vec2b, Widget, WidgetText
};

/*
Sequencer (MVP):
1. Draw a "grid" with tracking:
    * Convert pointer coordinates to timeline index
2. Add elements
    * Should be able to move
    * Should be able to stretch
    * Everything should snap to the grid
3. Scroll can be achieved with scroll-area (hopefully)
4. Add tracks
*/

pub const TIMELINE_STEP: f32 = 20.0;
pub const ELEMENT_HEIGHT: f32 = 20.0;
pub const ELEMENT_STRETCH_ZONE: f32 = 15.0;

pub struct Sequencer<'a> {
    pub elements: &'a mut Vec<SequencerElement>,
    pub state: &'a mut SequencerState,
    pub size: Vec2,
}

pub struct SequencerElement {
    pub text: Option<WidgetText>,
    pub pos: f32,
    pub len: f32,
}

#[derive(Debug)]
pub enum SequencerState {
    None,
    MovingElement {
        element_id: usize,
        start_pos: f32,
        total_drag: f32,
    },
    StretchingElementLeft {
        element_id: usize,
        start_left: f32,
        start_right: f32,
        total_drag: f32,
    },
    StretchingElementRight {
        element_id: usize,
        start_left: f32,
        start_right: f32,
        total_drag: f32,
    },
}

impl<'a> Widget for Sequencer<'a> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let (reponse, painter) = ui.allocate_painter(self.size, Sense::click_and_drag());

        if !ui.is_rect_visible(reponse.rect) {
            return reponse;
        }

        let timeline_rect = reponse.rect;

        painter.rect_filled(timeline_rect, 0.0, Color32::WHITE);

        for section in 1..((timeline_rect.width() / TIMELINE_STEP) as i32) {
            painter.line_segment(
                [
                    pos2(
                        timeline_rect.left() + section as f32 * TIMELINE_STEP,
                        timeline_rect.top(),
                    ),
                    pos2(
                        timeline_rect.left() + section as f32 * TIMELINE_STEP,
                        timeline_rect.bottom(),
                    ),
                ],
                Stroke::new(1.0, Color32::GRAY),
            );
        }

        for (idx, element) in self.elements.iter_mut().enumerate() {
            let top = timeline_rect.top();
            let left = timeline_rect.left();
            let element_rect = Rect::from_min_size(
                pos2(left + element.pos, top),
                vec2(element.len, ELEMENT_HEIGHT),
            );
            painter.rect_filled(element_rect, 4.0, Color32::RED);
            if let Some(text) = &element.text {
                let text_gal = text.clone().into_galley(
                    ui,
                    Some(egui::TextWrapMode::Truncate),
                    element.len,
                    TextStyle::Button,
                );
                let text_pos = ui
                    .layout()
                    .align_size_within_rect(text_gal.size(), element_rect)
                    .min;
                painter.galley(text_pos, text_gal, Color32::WHITE);
            }

            let Some(pointer) = reponse.hover_pos() else {
                continue;
            };

            match self.state {
                SequencerState::None => {
                    if !element_rect.contains(pointer) {
                        continue;
                    }
                    let local_off = pointer.x - element_rect.left();
                    if local_off <= ELEMENT_STRETCH_ZONE {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        if reponse.is_pointer_button_down_on() {
                            *self.state = SequencerState::StretchingElementLeft { 
                                element_id: idx, 
                                start_left: element.pos,
                                start_right: element.pos + element.len,
                                total_drag: 0.0f32,
                            }
                        }
                    } else if local_off >= element_rect.width() - ELEMENT_STRETCH_ZONE {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        if reponse.is_pointer_button_down_on() {
                            *self.state = SequencerState::StretchingElementRight { 
                                element_id: idx, 
                                start_left: element.pos,
                                start_right: element.pos + element.len,
                                total_drag: 0.0f32,
                            }
                        }
                    } else {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                        if reponse.is_pointer_button_down_on() {
                            *self.state = SequencerState::MovingElement { 
                                element_id: idx, 
                                start_pos: element.pos,
                                total_drag: 0.0f32,
                            }
                        }
                    }
                },
                SequencerState::MovingElement { 
                    element_id, 
                    start_pos,
                    total_drag, 
                } if *element_id == idx => {
                    if reponse.drag_stopped() {
                        *self.state = SequencerState::None;
                        continue
                    }
                    *total_drag += reponse.drag_delta().x;

                    let final_pos = *start_pos + *total_drag;
                    element.pos = final_pos;
                },
                SequencerState::StretchingElementLeft { 
                    element_id, 
                    start_left, 
                    start_right,
                    total_drag, 
                } if *element_id == idx => {
                    if reponse.drag_stopped() {
                        *self.state = SequencerState::None;
                        continue
                    }
                    *total_drag += reponse.drag_delta().x;

                    let final_left = *start_left + *total_drag;
                    let final_right = *start_right;
                    element.len = final_right - final_left;
                    element.pos = final_left;
                },
                SequencerState::StretchingElementRight { 
                    element_id, 
                    start_left, 
                    start_right,
                    total_drag, 
                } if *element_id == idx => {
                    if reponse.drag_stopped() {
                        *self.state = SequencerState::None;
                        continue
                    }
                    *total_drag += reponse.drag_delta().x;

                    let final_left = *start_left;
                    let final_right = *start_right + *total_drag;
                    element.len = final_right - final_left;
                    element.pos = final_left;
                },
                _ => (),
            }
        }

        if let Some(hover) = reponse.hover_pos() {
            painter.line_segment(
                [
                    pos2(hover.x, timeline_rect.top()),
                    pos2(hover.x, timeline_rect.bottom()),
                ],
                Stroke::new(1.0, Color32::RED),
            );
        }

        reponse
    }
}

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
    elements: Vec<SequencerElement>,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self {
            sequencer_state: SequencerState::None,
            elements: vec![
                SequencerElement {
                    text: Some("lol".into()),
                    pos: 10.0,
                    len: 40.0,
                },
                SequencerElement {
                    text: Some("some event".into()),
                    pos: 60.0,
                    len: 60.0,
                },
            ],
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::Window::new("My Window")
            .resizable(Vec2b::new(true, true))
            .show(ctx, |ui| {
                ui.label("Hello world!");
                let _ = ui.button("lol");
                Sequencer {
                    state: &mut self.sequencer_state,
                    elements: &mut self.elements,
                    size: Vec2::new(500.0, 200.0),
                }
                .ui(ui);
                ui.label("Hello world!");
            });
    }
}
