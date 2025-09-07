use eframe::egui;
use egui::{
    Color32, Key, Painter, Pos2, Rect, Response, Sense, Stroke, TextStyle, Ui, Vec2, Vec2b, Widget,
    WidgetText, epaint, pos2, vec2,
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

pub const PIXELS_PER_UNIT: f32 = 20.0;
pub const CLIP_HEIGHT: f32 = 20.0;
pub const CLIP_RESIZE_ZONE: f32 = 8.0;
pub const CLIP_MIN_SIZE: f32 = 2.0 * CLIP_RESIZE_ZONE + 8.0;

pub struct Sequencer<'a> {
    pub clips: &'a mut Vec<Clip>,
    pub state: &'a mut SequencerState,
    pub zoom: &'a mut f32,
    pub pan: &'a mut f32,
    pub size: Vec2,
}

impl<'a> Sequencer<'a> {
    fn timeline_input(&mut self, ui: &mut Ui, response: &Response, timeline_rect: Rect) {
        let Some(pointer) = response.hover_pos() else {
            return;
        };

        match *self.state {
            SequencerState::Idle => self.timeline_input_idle(ui, response, timeline_rect, pointer),
            SequencerState::MoveClip {
                clip_id,
                start_pos,
                total_drag_delta,
            } => self.timeline_input_moving_clip(ui, response, clip_id, start_pos, total_drag_delta),
            SequencerState::ResizeClip {
                clip_id,
                start_left,
                start_right,
                total_drag_delta,
                resize_left,
            } => self.timeline_input_resizing_clip(
                ui,
                response,
                clip_id,
                start_left,
                start_right,
                total_drag_delta,
                resize_left,
            ),
            SequencerState::Pan {
                start_pan,
                total_drag_delta,
            } => self.timeline_input_pan(ui, start_pan, total_drag_delta),
        }
    }

    fn timeline_input_idle(
        &mut self,
        ui: &mut Ui,
        response: &Response,
        timeline_rect: Rect,
        pointer: Pos2,
    ) {
        self.timeline_input_idle_clips(ui, response, timeline_rect, pointer);
        self.timeline_input_idle_pan_and_zoom(ui);
    }

    fn timeline_input_idle_clips(
        &mut self,
        ui: &mut Ui,
        response: &Response,
        timeline_rect: Rect,
        pointer: Pos2,
    ) {
        // Find a clip that the user is hovering on
        let Some((clip_id, clip, cursor_mode)) =
            self.clips.iter().enumerate().find_map(|(idx, clip)| {
                clip.get_pointer_intent(timeline_rect, pointer, *self.zoom, *self.pan)
                    .map(|x| (idx, clip, x))
            })
        else {
            return;
        };

        match cursor_mode {
            ClipPointerIntent::Move => ui.ctx().set_cursor_icon(egui::CursorIcon::Grab),
            ClipPointerIntent::Resize { .. } => {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal)
            }
        }

        let left_button_down = ui
            .ctx()
            .input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        // Switch state if the user actually started an interraction.
        if !response.is_pointer_button_down_on() || !left_button_down {
            return;
        }
        let new_state = match cursor_mode {
            ClipPointerIntent::Move => SequencerState::MoveClip {
                clip_id,
                start_pos: clip.pos,
                total_drag_delta: 0.0f32,
            },
            ClipPointerIntent::Resize { resize_left } => SequencerState::ResizeClip {
                resize_left,
                clip_id,
                start_left: clip.pos,
                start_right: clip.pos + clip.len,
                total_drag_delta: 0.0f32,
            },
        };
        *self.state = new_state;
    }

    fn timeline_input_idle_pan_and_zoom(&mut self, ui: &mut Ui) {
        let mut middle_button_down = false;
        let mut space_down = false;
        let mut plus_pressed = false;
        let mut minus_pressed = false;
        ui.ctx().input(|i| {
            middle_button_down = i.pointer.button_down(egui::PointerButton::Middle);
            space_down = i.key_down(Key::Space);
            plus_pressed = i.key_pressed(Key::Plus);
            minus_pressed = i.key_pressed(Key::Minus);
        });

        if plus_pressed {
            *self.zoom *= 1.3f32;
        } else if minus_pressed {
            *self.zoom /= 1.3f32;
        }

        if !middle_button_down && !space_down {
            return;
        }

        *self.state = SequencerState::Pan {
            start_pan: *self.pan,
            total_drag_delta: 0.0,
        };
    }

    fn timeline_input_moving_clip(
        &mut self,
        ui: &mut Ui,
        response: &Response,
        clip_id: usize,
        start_pos: f32,
        mut total_drag_delta: f32,
    ) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        let left_button_down = ui
            .ctx()
            .input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        if !response.is_pointer_button_down_on() || !left_button_down {
            *self.state = SequencerState::Idle;
            return;
        }
        total_drag_delta += response.drag_delta().x;

        let clip = &mut self.clips[clip_id];
        clip.pos = start_pos + total_drag_delta / *self.zoom;
        *self.state = SequencerState::MoveClip {
            clip_id,
            start_pos,
            total_drag_delta,
        }
    }

    fn timeline_input_resizing_clip(
        &mut self,
        ui: &mut Ui,
        response: &Response,
        clip_id: usize,
        start_left: f32,
        start_right: f32,
        mut total_drag_delta: f32,
        resize_left: bool,
    ) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        let left_button_down = ui
            .ctx()
            .input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        if !response.is_pointer_button_down_on() && !left_button_down {
            *self.state = SequencerState::Idle;
            return;
        }
        total_drag_delta += response.drag_delta().x;

        let (mut final_left, mut final_right) = (start_left, start_right);
        if resize_left {
            final_left += total_drag_delta / *self.zoom;
        } else {
            final_right += total_drag_delta / *self.zoom;
        };
        if final_right - final_left < CLIP_MIN_SIZE {
            return;
        }

        let clip = &mut self.clips[clip_id];
        clip.len = final_right - final_left;
        clip.pos = final_left;
        *self.state = SequencerState::ResizeClip {
            clip_id,
            start_left,
            start_right,
            resize_left,
            total_drag_delta,
        }
    }

    fn timeline_input_pan(&mut self, ui: &mut Ui, start_pan: f32, mut total_drag_delta: f32) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        let mut middle_button_down = false;
        let mut space_down = false;
        ui.ctx().input(|i| {
            middle_button_down = i.pointer.button_down(egui::PointerButton::Middle);
            space_down = i.key_down(Key::Space);
        });
        if !middle_button_down && !space_down {
            *self.state = SequencerState::Idle;
            return;
        }
        total_drag_delta += ui.ctx().input(|inp| inp.pointer.delta().x);
        *self.pan = start_pan + total_drag_delta / *self.zoom;
        *self.pan = self.pan.min(0.0);

        *self.state = SequencerState::Pan {
            start_pan,
            total_drag_delta,
        }
    }

    fn paint_timeline(&self, ui: &Ui, painter: &Painter, timeline_rect: Rect) {
        painter.rect(
            timeline_rect,
            0.0,
            ui.visuals().noninteractive().bg_fill,
            ui.visuals().noninteractive().fg_stroke,
            epaint::StrokeKind::Inside,
        );

        for section in 1..((timeline_rect.width() / PIXELS_PER_UNIT) as i32) {
            let mark_x =
                timeline_rect.left() + (*self.pan + section as f32 * PIXELS_PER_UNIT) * *self.zoom;
            let mark_points = [
                pos2(mark_x, timeline_rect.top()),
                pos2(mark_x, timeline_rect.bottom()),
            ];
            let color = ui.visuals().weak_text_color();
            painter.line_segment(mark_points, Stroke::new(1.0, color));
        }
    }

    fn paint_clips(&self, ui: &Ui, painter: &Painter, timeline_rect: Rect) {
        for clip in &*self.clips {
            clip.paint(ui, painter, timeline_rect, *self.zoom, *self.pan);
        }
    }

    fn paint_timeline_cursor(&self, response: &Response, painter: &Painter, timeline_rect: Rect) {
        let Some(hover) = response.hover_pos() else {
            return;
        };
        painter.line_segment(
            [
                pos2(hover.x, timeline_rect.top()),
                pos2(hover.x, timeline_rect.bottom()),
            ],
            Stroke::new(1.0, Color32::RED),
        );
    }
}

pub struct Clip {
    pub text: Option<WidgetText>,
    pub pos: f32,
    pub len: f32,
}

impl Clip {
    pub fn rect(&self, timeline_rect: Rect, zoom: f32, pan: f32) -> Rect {
        let top = timeline_rect.top();
        let left = timeline_rect.left() + pan * zoom;

        Rect::from_min_size(
            pos2(left + self.pos * zoom, top),
            vec2(self.len * zoom, CLIP_HEIGHT),
        )
    }

    pub fn paint(&self, ui: &Ui, painter: &Painter, timeline_rect: Rect, zoom: f32, pan: f32) {
        let this_rect = self.rect(timeline_rect, zoom, pan);
        let left_resize_rect = Rect {
            max: pos2(this_rect.min.x + CLIP_RESIZE_ZONE, this_rect.max.y),
            ..this_rect
        };
        let right_resize_rect = Rect {
            min: pos2(this_rect.max.x - CLIP_RESIZE_ZONE, this_rect.min.y),
            ..this_rect
        };
        let move_rect = Rect {
            min: pos2(this_rect.min.x + CLIP_RESIZE_ZONE * 0.5, this_rect.min.y),
            max: pos2(this_rect.max.x - CLIP_RESIZE_ZONE * 0.5, this_rect.max.y),
        };

        let border_stroke = ui.visuals().widgets.inactive.bg_stroke;
        painter.rect_filled(left_resize_rect, 0.0, Color32::DARK_RED);
        painter.rect_filled(right_resize_rect, 0.0, Color32::DARK_RED);
        painter.rect_filled(move_rect, 0.0, Color32::RED);
        painter.rect(
            this_rect,
            0.0,
            Color32::TRANSPARENT,
            border_stroke,
            egui::StrokeKind::Inside,
        );

        let padding = ui.spacing().button_padding;
        let Some(text) = &self.text else { return };
        let text_gal = text.clone().into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            this_rect.width() - 2.0 * padding.x,
            TextStyle::Button,
        );
        let text_pos = ui
            .layout()
            .align_size_within_rect(text_gal.size(), this_rect.shrink2(padding))
            .min;
        painter.galley(text_pos, text_gal, Color32::WHITE);
    }

    pub fn get_pointer_intent(
        &self,
        timeline_rect: Rect,
        pointer: Pos2,
        zoom: f32,
        pan: f32,
    ) -> Option<ClipPointerIntent> {
        let this_rect = self.rect(timeline_rect, zoom, pan);

        if !this_rect.contains(pointer) {
            return None;
        }
        let local_off = pointer.x - this_rect.left();
        let resize_left = local_off <= CLIP_RESIZE_ZONE;
        let resize_right = local_off >= this_rect.width() - CLIP_RESIZE_ZONE;

        if resize_left || resize_right {
            Some(ClipPointerIntent::Resize { resize_left })
        } else {
            Some(ClipPointerIntent::Move)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ClipPointerIntent {
    Move,
    Resize { resize_left: bool },
}

#[derive(Debug)]
pub enum SequencerState {
    Idle,
    MoveClip {
        clip_id: usize,
        start_pos: f32,
        total_drag_delta: f32,
    },
    ResizeClip {
        clip_id: usize,
        start_left: f32,
        start_right: f32,
        resize_left: bool,
        total_drag_delta: f32,
    },
    Pan {
        start_pan: f32,
        total_drag_delta: f32,
    },
}

impl<'a> Widget for Sequencer<'a> {
    fn ui(mut self, ui: &mut Ui) -> egui::Response {
        let (response, painter) = ui.allocate_painter(self.size, Sense::click_and_drag());
        let timeline_rect = response.rect;
        if !ui.is_rect_visible(timeline_rect) {
            return response;
        }

        self.timeline_input(ui, &response, timeline_rect);

        self.paint_timeline(ui, &painter, timeline_rect);
        self.paint_clips(ui, &painter, timeline_rect);
        self.paint_timeline_cursor(&response, &painter, timeline_rect);

        response
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
    zoom: f32,
    pan: f32,
    clips: Vec<Clip>,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self {
            sequencer_state: SequencerState::Idle,
            clips: vec![
                Clip {
                    text: Some("lol".into()),
                    pos: 10.0,
                    len: 40.0,
                },
                Clip {
                    text: Some("some event".into()),
                    pos: 60.0,
                    len: 60.0,
                },
            ],
            zoom: 1.0,
            pan: 0.0,
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
                    clips: &mut self.clips,
                    size: Vec2::new(500.0, 200.0),
                    zoom: &mut self.zoom,
                    pan: &mut self.pan,
                }
                .ui(ui);
                ui.label("Hello world!");
            });
    }
}
