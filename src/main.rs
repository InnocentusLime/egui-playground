use eframe::egui;
use egui::{
    Color32, Painter, Pos2, Rect, Response, Sense, Stroke, TextStyle, Ui, Vec2, Vec2b, Widget,
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
pub const ELEMENT_HEIGHT: f32 = 20.0;
pub const CLIP_RESIZE_ZONE: f32 = 8.0;
pub const CLIP_MIN_SIZE: f32 = 2.0 * CLIP_RESIZE_ZONE + 8.0;

pub struct Sequencer<'a> {
    pub clips: &'a mut Vec<Clip>,
    pub state: &'a mut SequencerState,
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
                clip_id: element_id,
                start_pos,
                total_drag_delta: total_drag,
            } => self.timeline_input_moving_clip(ui, response, element_id, start_pos, total_drag),
            SequencerState::ResizeClip {
                clip_id: element_id,
                start_left,
                start_right,
                total_drag_delta: total_drag,
                resize_left,
            } => self.timeline_input_resizing_clip(
                ui,
                response,
                element_id,
                start_left,
                start_right,
                total_drag,
                resize_left,
            ),
        }
    }

    fn timeline_input_idle(
        &mut self,
        ui: &mut Ui,
        response: &Response,
        timeline_rect: Rect,
        pointer: Pos2,
    ) {
        // Find a clip that the user is hovering on
        let Some((clip_id, clip, cursor_mode)) =
            self.clips.iter().enumerate().find_map(|(idx, clip)| {
                clip.get_pointer_intent(timeline_rect, pointer)
                    .map(|x| (idx, clip, x))
            })
        else {
            return;
        };

        match cursor_mode {
            ClipPointerIntent::Move => ui.ctx().set_cursor_icon(egui::CursorIcon::Grab),
            ClipPointerIntent::Resize { .. } => ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal)
        }

        // Switch state if the user actually started an interraction.
        if !response.is_pointer_button_down_on() {
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

    fn timeline_input_moving_clip(
        &mut self,
        ui: &mut Ui,
        response: &Response,
        element_id: usize,
        start_pos: f32,
        mut total_drag_delta: f32,
    ) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        if !response.is_pointer_button_down_on() {
            *self.state = SequencerState::Idle;
            return;
        }
        total_drag_delta += response.drag_delta().x;

        let element = &mut self.clips[element_id];
        element.pos = start_pos + total_drag_delta;
        *self.state = SequencerState::MoveClip {
            clip_id: element_id,
            start_pos,
            total_drag_delta,
        }
    }

    fn timeline_input_resizing_clip(
        &mut self,
        ui: &mut Ui,
        response: &Response,
        element_id: usize,
        start_left: f32,
        start_right: f32,
        mut total_drag_delta: f32,
        resize_left: bool,
    ) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        if !response.is_pointer_button_down_on() {
            *self.state = SequencerState::Idle;
            return;
        }
        total_drag_delta += response.drag_delta().x;

        let (mut final_left, mut final_right) = (start_left, start_right);
        if resize_left {
            final_left += total_drag_delta;
        } else {
            final_right += total_drag_delta;
        };
        if final_right - final_left < CLIP_MIN_SIZE {
            return;
        }

        let element = &mut self.clips[element_id];
        element.len = final_right - final_left;
        element.pos = final_left;
        *self.state = SequencerState::ResizeClip {
            clip_id: element_id,
            start_left,
            start_right,
            resize_left,
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
            let mark_x = timeline_rect.left() + section as f32 * PIXELS_PER_UNIT;
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
            clip.paint(ui, painter, timeline_rect);
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
    pub fn rect(&self, timeline_rect: Rect) -> Rect {
        let top = timeline_rect.top();
        let left = timeline_rect.left();

        Rect::from_min_size(pos2(left + self.pos, top), vec2(self.len, ELEMENT_HEIGHT))
    }

    pub fn paint(&self, ui: &Ui, painter: &Painter, timeline_rect: Rect) {
        let this_rect = self.rect(timeline_rect);
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
            self.len - 2.0 * padding.x,
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
    ) -> Option<ClipPointerIntent> {
        let this_rect = self.rect(timeline_rect);

        if !this_rect.contains(pointer) {
            return None;
        }
        let local_off = pointer.x - this_rect.left();
        let resize_left = local_off <= CLIP_RESIZE_ZONE;
        let resize_right = local_off >= self.len - CLIP_RESIZE_ZONE;

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
    elements: Vec<Clip>,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self {
            sequencer_state: SequencerState::Idle,
            elements: vec![
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
                    clips: &mut self.elements,
                    size: Vec2::new(500.0, 200.0),
                }
                .ui(ui);
                ui.label("Hello world!");
            });
    }
}
