use egui::{Color32, Painter, Pos2, Rect, Stroke, TextStyle, Ui, WidgetText, pos2, vec2};

use super::TimelineTf;

pub const CLIP_HEIGHT: f32 = 20.0;
pub const CLIP_RESIZE_ZONE: f32 = 4.0;
pub const CLIP_RENDER_EPSILON: f32 = 5.0;
pub const TRACK_LABEL_WIDTH: f32 = 100.0;
pub const TRACK_MARK_WIDTH: f32 = 10.0;

#[derive(Debug, Clone, Copy)]
pub enum ClipAction {
    Move,
    Resize { resize_left: bool },
}

pub struct Clip {
    pub id: u32,
    pub track_id: u32,
    pub label: WidgetText,
    pub pos: u32,
    pub len: u32,
}

impl Clip {
    pub fn rect(&self, track_y: u32, timeline_rect: Rect, tf: TimelineTf) -> Rect {
        let top = timeline_rect.top() + (track_y as f32) * CLIP_HEIGHT;
        let left = timeline_rect.left() + tf.tf_pos(self.pos as f32);
        let width = tf.tf_vector(self.len as f32);

        Rect::from_min_size(pos2(left, top), vec2(width, CLIP_HEIGHT))
    }

    pub fn paint(
        &self,
        ui: &Ui,
        painter: &Painter,
        timeline_rect: Rect,
        tf: TimelineTf,
        track_y: u32,
        color: Color32,
        selected: bool,
    ) {
        let dark_color = Color32::BLACK + color.additive().linear_multiply(0.2);
        let text_color = invert_color(color);
        let padding = ui.spacing().button_padding;
        let this_rect = self.rect(track_y, timeline_rect, tf);
        let left_resize_rect = Rect {
            max: pos2(this_rect.min.x + CLIP_RESIZE_ZONE, this_rect.max.y),
            ..this_rect
        };
        let right_resize_rect = Rect {
            min: pos2(this_rect.max.x - CLIP_RESIZE_ZONE, this_rect.min.y),
            ..this_rect
        };
        let move_rect = Rect {
            min: pos2(this_rect.min.x + CLIP_RESIZE_ZONE, this_rect.min.y),
            max: pos2(this_rect.max.x - CLIP_RESIZE_ZONE, this_rect.max.y),
        };

        if this_rect.width() > 2.0 * CLIP_RESIZE_ZONE + CLIP_RENDER_EPSILON {
            painter.rect_filled(left_resize_rect, 0.0, dark_color);
            painter.rect_filled(right_resize_rect, 0.0, dark_color);
            painter.rect_filled(move_rect, 0.0, color);
            if selected {
                painter.rect(
                    this_rect,
                    0.0,
                    Color32::TRANSPARENT,
                    ui.visuals().selection.stroke,
                    egui::StrokeKind::Inside,
                );
            }
        } else {
            let mini_rect_width = this_rect.width().max(CLIP_RENDER_EPSILON);
            let mini_rect =
                Rect::from_min_size(this_rect.min, vec2(mini_rect_width, this_rect.height()));
            painter.rect(
                mini_rect,
                0.0,
                color,
                Stroke::new(1.0, dark_color),
                egui::StrokeKind::Inside,
            );
            if selected {
                painter.rect(
                    mini_rect,
                    0.0,
                    Color32::TRANSPARENT,
                    ui.visuals().selection.stroke,
                    egui::StrokeKind::Inside,
                );
            }
        }

        if move_rect.width() > 2.0 * padding.x + CLIP_RENDER_EPSILON {
            let text_gal = self.label.clone().into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                move_rect.width() - 2.0 * padding.x,
                TextStyle::Button,
            );
            let text_pos = ui
                .layout()
                .align_size_within_rect(text_gal.size(), move_rect.shrink2(padding))
                .min;
            painter.galley(text_pos, text_gal, text_color);
        }

        let border_stroke = ui.visuals().widgets.inactive.bg_stroke;
        painter.rect(
            this_rect,
            0.0,
            Color32::TRANSPARENT,
            border_stroke,
            egui::StrokeKind::Inside,
        );
    }

    pub fn pointer_action(
        &self,
        timeline_rect: Rect,
        pointer: Pos2,
        tf: TimelineTf,
        track_y: u32,
    ) -> ClipAction {
        let this_rect = self.rect(track_y, timeline_rect, tf);
        let local_off = pointer.x - this_rect.left();
        let resize_left = local_off <= CLIP_RESIZE_ZONE;
        let resize_right = local_off >= this_rect.width() - CLIP_RESIZE_ZONE;

        if resize_left || resize_right {
            ClipAction::Resize { resize_left }
        } else {
            ClipAction::Move
        }
    }
}

#[derive(Clone)]
pub struct TrackInfo {
    pub id: u32,
    pub name: WidgetText,
    pub color: Color32,
}

pub struct Clips {
    next_track_id: u32,
    next_clip_id: u32,
    tracks: Vec<TrackInfo>,
    clips: Vec<Clip>,
}

impl Clips {
    pub fn new() -> Clips {
        Clips {
            next_track_id: 0,
            next_clip_id: 0,
            tracks: Vec::new(),
            clips: Vec::new(),
        }
    }

    pub fn add_track(&mut self, name: WidgetText, color: Color32) {
        self.tracks.push(TrackInfo { 
            id: self.next_track_id, 
            name, 
            color, 
        });
        self.next_track_id += 1;
    }

    pub fn delete_track(&mut self, track_id: u32) {
        self.tracks.retain(|x| x.id != track_id);
        self.clips.retain(|x| x.track_id != track_id);
    }

    pub fn add_clip(&mut self, track_id: u32, label: WidgetText, pos: u32, len: u32) -> bool {
        if !self.tracks.iter().any(|x| x.id == track_id) {
            return false;
        }

        if self.clip_has_intersection(track_id, u32::MAX, pos, len) {
            return false;
        }

        self.clips.push(Clip {
            track_id,
            id: self.next_clip_id,
            label,
            pos,
            len,
        });
        self.next_clip_id += 1;
        true
    }

    pub fn delete_clip(&mut self, idx: u32) {
        self.clips.retain(|x| x.id != idx);
    }

    pub fn set_clip_pos_len(&mut self, idx: u32, new_track_y: u32, new_pos: u32, new_len: u32) {
        let Some(new_track) = self.track_containing_pos(new_track_y) else {
            return;
        };

        if self.clip_has_intersection(new_track, idx, new_pos, new_len) {
            return;
        }

        let Some(clip) = self.get_mut(idx) else {
            return;
        };

        clip.track_id = new_track;
        clip.pos = new_pos;
        clip.len = new_len;
    }

    pub fn get(&self, idx: u32) -> Option<&Clip> {
        self.clips.iter().find(|x| x.id == idx)
    }

    fn get_mut(&mut self, idx: u32) -> Option<&mut Clip> {
        self.clips.iter_mut().find(|x| x.id == idx)
    }

    pub fn paint_track_labels(&self, ui: &mut Ui, painter: &Painter, widget_rect: Rect) {
        for (y, track) in self.tracks.iter().enumerate() {
            let top = widget_rect.top() + (y as f32) * CLIP_HEIGHT;
            let padding = ui.spacing().button_padding;

            let rect = Rect::from_min_size(
                pos2(widget_rect.left(), top),
                vec2(TRACK_MARK_WIDTH, CLIP_HEIGHT),
            );
            painter.rect_filled(rect, 0.0, track.color);

            let text_gal = track.name.clone().into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                TRACK_LABEL_WIDTH - TRACK_MARK_WIDTH - 2.0 * padding.x,
                TextStyle::Button,
            );

            let rect = Rect::from_min_size(
                pos2(widget_rect.left() + TRACK_MARK_WIDTH, top),
                vec2(TRACK_LABEL_WIDTH - TRACK_MARK_WIDTH, CLIP_HEIGHT),
            );
            let text_pos = ui
                .layout()
                .align_size_within_rect(text_gal.size(), rect.shrink2(padding))
                .min;
            painter.galley(text_pos, text_gal, Color32::WHITE);
        }
    }

    pub fn paint_clips(
        &self,
        ui: &mut Ui,
        painter: &Painter,
        timeline_rect: Rect,
        tf: TimelineTf,
        selected_clip: Option<u32>,
    ) {
        for clip in self.clips.iter() {
            let selected = selected_clip.map(|x| x == clip.id).unwrap_or_default();
            let (track_y, track) = self
                .tracks
                .iter()
                .enumerate()
                .find(|(_, x)| x.id == clip.track_id)
                .unwrap();
            clip.paint(
                ui,
                painter,
                timeline_rect,
                tf,
                track_y as u32,
                track.color,
                selected,
            );
        }
    }

    pub fn track_y(&self, track_id: u32) -> Option<u32> {
        self.tracks
            .iter()
            .enumerate()
            .find(|(_, x)| x.id == track_id)
            .map(|(id, _)| id as u32)
    }

    pub fn track_containing_pos(&self, track_y: u32) -> Option<u32> {
        self.tracks
            .iter()
            .enumerate()
            .find(|(idx, _)| *idx as u32 == track_y)
            .map(|(_, track)| track.id)
    }

    pub fn clip_containing_pos(&self, track_y: u32, pos: u32) -> Option<&Clip> {
        let track_id = self.track_containing_pos(track_y)?;
        self.clips
            .iter()
            .find(|x| x.track_id == track_id && x.pos <= pos && pos <= x.pos + x.len)
    }

    fn clip_has_intersection(&self, track_id: u32, skip: u32, pos: u32, len: u32) -> bool {
        for clip in self.clips.iter().filter(|x| x.track_id == track_id) {
            if clip.id == skip {
                continue;
            }
            if clip.pos <= pos && clip.pos + clip.len > pos {
                return true;
            }
            if pos <= clip.pos && pos + len > clip.pos {
                return true;
            }
        }
        false
    }
}

fn invert_color(col: Color32) -> Color32 {
    let arr = col.to_normalized_gamma_f32();
    let arr_inv = [1.0 - arr[0], 1.0 - arr[1], 1.0 - arr[2]];
    Color32::from_rgb(
        (arr_inv[0] * 255.0) as u8,
        (arr_inv[1] * 255.0) as u8,
        (arr_inv[2] * 255.0) as u8,
    )
}
