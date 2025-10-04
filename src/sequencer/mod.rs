mod clips;

use egui::{epaint, vec2};
use egui::{Color32, Key, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget, pos2};

pub use clips::*;

pub const PIXELS_PER_UNIT: f32 = 18.0;

#[derive(Debug, Clone, Copy)]
pub struct TimelineTf {
    pub zoom: f32,
    pub pan: f32,
}

impl TimelineTf {
    pub fn tf_pos(&self, pos: f32) -> f32 {
        self.zoom * (pos + self.pan)
    }

    pub fn tf_vector(&self, vec: f32) -> f32 {
        self.zoom * vec
    }

    pub fn inv_tf_vector(&self, vec: f32) -> f32 {
        vec / self.zoom
    }

    pub fn inv_tf_pos(&self, pos: f32) -> f32 {
        pos / self.zoom - self.pan
    }
}

#[derive(Debug)]
pub enum SequencerState {
    Idle,
    MoveClip {
        clip_id: u32,
        start_pos_x: f32,
        start_pos_y: f32,
        total_drag_delta_x: f32,
        total_drag_delta_y: f32,
    },
    ResizeClip {
        clip_id: u32,
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

pub struct Sequencer<'a> {
    pub cursor_pos: &'a mut u32,
    pub selected_clip: &'a mut Option<u32>,
    pub selected_track: &'a mut Option<u32>,
    pub clips: &'a mut Clips,
    pub state: &'a mut SequencerState,
    pub tf: &'a mut TimelineTf,
    pub size: Vec2,
}

impl<'a> Sequencer<'a> {
    fn timeline_input(&mut self, ui: &mut Ui, response: &Response, timeline_rect: Rect) {
        let Some(pointer) = response.hover_pos() else {
            return;
        };

        if let Some(selected_clip) = self.selected_clip.clone() {
            if self.clips.get(selected_clip).is_none() {
                *self.selected_clip = None;
            }
        }

        match *self.state {
            SequencerState::Idle => self.timeline_input_idle(ui, response, timeline_rect, pointer),
            SequencerState::MoveClip {
                clip_id,
                start_pos_x,
                start_pos_y,
                total_drag_delta_x,
                total_drag_delta_y,
            } => self.timeline_input_moving_clip(
                ui,
                response,
                clip_id,
                start_pos_x,
                start_pos_y,
                total_drag_delta_x,
                total_drag_delta_y,
            ),
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
        self.timeline_input_idle_clips(ui, timeline_rect, pointer);
        self.timeline_input_idle_pan_and_zoom(ui);
        self.timeline_input_idle_cursor(response, timeline_rect);
    }

    fn timeline_input_idle_clips(&mut self, ui: &mut Ui, timeline_rect: Rect, pointer: Pos2) {
        let left_button_down = ui
            .ctx()
            .input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        let x_pos = self.tf.inv_tf_pos(pointer.x - timeline_rect.left()).round() as u32;
        let y_pos = ((pointer.y - timeline_rect.top()) / CLIP_HEIGHT) as u32;
        let clip = self.clips.clip_containing_pos(y_pos, x_pos);

        if left_button_down {
            *self.selected_clip = clip.map(|x| x.id);
        }

        if let Some(clip) = clip {
            let track_y = self.clips.track_y(clip.track_id).unwrap();
            let action = clip.pointer_action(timeline_rect, pointer, *self.tf, track_y);
            Self::clip_action_to_cursor(ui, action);
            if left_button_down {
                *self.state = Self::clip_action_to_new_state(track_y, clip, action);
            }
        }
    }

    fn clip_action_to_cursor(ui: &mut Ui, action: ClipAction) {
        match action {
            ClipAction::Move => ui.ctx().set_cursor_icon(egui::CursorIcon::Grab),
            ClipAction::Resize { .. } => {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal)
            }
        }
    }

    fn clip_action_to_new_state(track_y: u32, clip: &Clip, action: ClipAction) -> SequencerState {
        match action {
            ClipAction::Move => SequencerState::MoveClip {
                clip_id: clip.id,
                start_pos_x: clip.pos as f32,
                start_pos_y: track_y as f32 * CLIP_HEIGHT,
                total_drag_delta_x: 0.0f32,
                total_drag_delta_y: 0.0f32,
            },
            ClipAction::Resize { resize_left } => SequencerState::ResizeClip {
                clip_id: clip.id,
                resize_left,
                start_left: clip.pos as f32,
                start_right: (clip.pos + clip.len) as f32,
                total_drag_delta: 0.0f32,
            },
        }
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
            self.tf.zoom *= 1.3f32;
        } else if minus_pressed {
            self.tf.zoom /= 1.3f32;
        }

        if !middle_button_down && !space_down {
            return;
        }

        *self.state = SequencerState::Pan {
            start_pan: self.tf.pan,
            total_drag_delta: 0.0,
        };
    }

    fn timeline_input_idle_cursor(&mut self, response: &Response, timeline_rect: Rect) {
        let Some(pos) = response.interact_pointer_pos() else {
            return;
        };
        let y_pos = ((pos.y - timeline_rect.top()) / CLIP_HEIGHT) as u32;
        if response.clicked() {
            *self.cursor_pos = self.tf.inv_tf_pos(pos.x - timeline_rect.left()).round() as u32;
            *self.selected_track = self.clips.track_containing_pos(y_pos);
        }
    }

    fn timeline_input_moving_clip(
        &mut self,
        ui: &mut Ui,
        response: &Response,
        clip_id: u32,
        start_pos_x: f32,
        start_pos_y: f32,
        mut total_drag_delta_x: f32,
        mut total_drag_delta_y: f32,
    ) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        let left_button_down = ui
            .ctx()
            .input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        if !response.is_pointer_button_down_on() || !left_button_down {
            *self.state = SequencerState::Idle;
            return;
        }
        total_drag_delta_x += response.drag_delta().x;
        total_drag_delta_y += response.drag_delta().y;

        let Some(clip) = self.clips.get(clip_id) else {
            *self.state = SequencerState::Idle;
            return;
        };
        let new_track_y = ((start_pos_y + total_drag_delta_y).max(0.0) / CLIP_HEIGHT) as u32;
        let new_pos = (start_pos_x + self.tf.inv_tf_vector(total_drag_delta_x)) as u32;
        let new_len = clip.len;
        self.clips
            .set_clip_pos_len(clip_id, new_track_y, new_pos, new_len);
        *self.state = SequencerState::MoveClip {
            clip_id,
            start_pos_x,
            start_pos_y,
            total_drag_delta_x,
            total_drag_delta_y,
        }
    }

    fn timeline_input_resizing_clip(
        &mut self,
        ui: &mut Ui,
        response: &Response,
        clip_id: u32,
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

        let final_size_delta = self.tf.inv_tf_vector(total_drag_delta);
        let (mut final_left, mut final_right) = (start_left, start_right);
        if resize_left {
            final_left = f32::min(final_right - 1.0, final_left + final_size_delta);
        } else {
            final_right = f32::max(final_left + 1.0, final_right + final_size_delta);
        };

        let Some(clip) = self.clips.get(clip_id) else {
            *self.state = SequencerState::Idle;
            return;
        };
        // We want to keep clip.len + clip.pos the same so
        // the right doesn't jitter
        let new_len = if resize_left {
            (clip.pos + clip.len) as f32 - final_left.round()
        } else {
            (final_right - final_left).round()
        };
        let new_pos = final_left.round() as u32;
        let new_track_y = self.clips.track_y(clip.track_id).unwrap();
        self.clips
            .set_clip_pos_len(clip_id, new_track_y, new_pos, new_len as u32);
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
        self.tf.pan = start_pan + self.tf.inv_tf_vector(total_drag_delta);
        self.tf.pan = self.tf.pan.min(0.0);

        *self.state = SequencerState::Pan {
            start_pan,
            total_drag_delta,
        }
    }

    fn paint_timeline(&self, ui: &Ui, painter: &Painter, timeline_rect: Rect) {
        painter.rect_filled(timeline_rect, 0.0, ui.visuals().noninteractive().bg_fill);

        let track = self.selected_track.and_then(|idx| self.clips.get_track(idx));
        if let Some(track) = track {
            let dark_color = Color32::BLACK + track.color.additive().linear_multiply(0.05);
            // let dark_color = Color32::WHITE;
            let track_y = self.clips.track_y(track.id).unwrap();
            let track_selection = Rect::from_min_size(
                pos2(timeline_rect.left(), timeline_rect.top() + (track_y as f32) * CLIP_HEIGHT), 
                vec2(timeline_rect.width(), CLIP_HEIGHT),
            );
            painter.rect_filled(track_selection, 0.0, dark_color); 
        }

        let mut last_painted = None;
        let mut painted_count = 0;
        let contigious_paint = self.tf.tf_vector(1.0) >= PIXELS_PER_UNIT;
        for section in 1..300 {
            let local_pos = self.tf.tf_pos(section as f32);
            let too_close = last_painted
                .map(|x| local_pos - x < PIXELS_PER_UNIT)
                .unwrap_or(false);
            if too_close {
                continue;
            }

            let mark_x = timeline_rect.left() + local_pos;
            let mark_points = [
                pos2(mark_x, timeline_rect.top()),
                pos2(mark_x, timeline_rect.bottom()),
            ];
            let color = if painted_count % 5 == 0 || contigious_paint {
                ui.visuals().weak_text_color()
            } else {
                ui.visuals().extreme_bg_color
            };
            if local_pos >= 0.0 {
                painter.line_segment(mark_points, Stroke::new(1.0, color));
            }

            painted_count += 1;
            last_painted = Some(local_pos);
        }

        painter.rect(
            timeline_rect,
            0.0,
            Color32::TRANSPARENT,
            ui.visuals().noninteractive().fg_stroke,
            epaint::StrokeKind::Inside,
        );
    }

    fn paint_timeline_cursor(&self, painter: &Painter, timeline_rect: Rect) {
        let cur_x = self.tf.tf_pos(*self.cursor_pos as f32) + timeline_rect.left();
        painter.line_segment(
            [
                pos2(cur_x, timeline_rect.top()),
                pos2(cur_x, timeline_rect.bottom()),
            ],
            Stroke::new(1.0, Color32::RED),
        );
    }
}

impl<'a> Widget for Sequencer<'a> {
    fn ui(mut self, ui: &mut Ui) -> egui::Response {
        let (response, mut painter) = ui.allocate_painter(self.size, Sense::click_and_drag());
        let widget_rect = response.rect;
        if !ui.is_rect_visible(widget_rect) {
            return response;
        }

        self.clips.paint_track_labels(ui, &painter, widget_rect);

        let mut timeline_rect = widget_rect;
        timeline_rect.set_left(timeline_rect.left() + TRACK_LABEL_WIDTH);
        painter.set_clip_rect(timeline_rect);

        self.timeline_input(ui, &response, timeline_rect);
        self.paint_timeline(ui, &painter, timeline_rect);

        painter.set_clip_rect(timeline_rect.shrink(1.0));
        self.clips
            .paint_clips(ui, &painter, timeline_rect, *self.tf, *self.selected_clip);
        self.paint_timeline_cursor(&painter, timeline_rect);

        response
    }
}
