use egui::{Color32, Painter, Pos2, Rect, Stroke, TextStyle, Ui, WidgetText, pos2, vec2};

use super::TimelineTf;

pub const CLIP_HEIGHT: f32 = 20.0;
pub const CLIP_RESIZE_ZONE: f32 = 4.0;
pub const CLIP_RENDER_EPSILON: f32 = 5.0;

#[derive(Debug, Clone, Copy)]
pub enum ClipAction {
    Move,
    Resize { resize_left: bool },
}

pub struct Clip {
    pub id: u32,
    pub label: WidgetText,
    pub pos: u32,
    pub len: u32,
}

impl Clip {
    pub fn rect(&self, timeline_rect: Rect, tf: TimelineTf) -> Rect {
        let top = timeline_rect.top();
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
        selected: bool,
    ) {
        let padding = ui.spacing().button_padding;
        let this_rect = self.rect(timeline_rect, tf);
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
            painter.rect_filled(left_resize_rect, 0.0, Color32::DARK_RED);
            painter.rect_filled(right_resize_rect, 0.0, Color32::DARK_RED);
            painter.rect_filled(move_rect, 0.0, Color32::RED);
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
                Color32::RED,
                Stroke::new(1.0, Color32::DARK_RED),
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
            painter.galley(text_pos, text_gal, Color32::WHITE);
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

    pub fn pointer_action(&self, timeline_rect: Rect, pointer: Pos2, tf: TimelineTf) -> ClipAction {
        let this_rect = self.rect(timeline_rect, tf);
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

pub struct Clips {
    next_id: u32,
    mem: Vec<Clip>,
}

impl Clips {
    pub fn new() -> Clips {
        Clips {
            next_id: 0,
            mem: Vec::new(),
        }
    }

    pub fn add_clip(&mut self, label: WidgetText, pos: u32, len: u32) -> bool {
        if self.clip_has_intersection(u32::MAX, pos, len) {
            return false;
        }

        self.mem.push(Clip {
            id: self.next_id,
            label,
            pos,
            len,
        });
        self.next_id += 1;
        true
    }

    pub fn delete_clip(&mut self, idx: u32) {
        self.mem.retain(|x| x.id != idx);
    }

    pub fn set_clip_pos_len(&mut self, idx: u32, new_pos: u32, new_len: u32) {
        if self.clip_has_intersection(idx, new_pos, new_len) {
            return;
        }

        let Some(clip) = self.mem.iter_mut().find(|x| x.id == idx) else {
            return;
        };

        clip.pos = new_pos;
        clip.len = new_len;
    }

    pub fn get(&self, idx: u32) -> Option<&Clip> {
        self.mem.iter().find(|x| x.id == idx)
    }

    pub fn paint(
        &self,
        ui: &mut Ui,
        painter: &Painter,
        timeline_rect: Rect,
        tf: TimelineTf,
        selected_clip: Option<u32>,
    ) {
        for clip in self.iter() {
            let selected = selected_clip.map(|x| x == clip.id).unwrap_or_default();
            clip.paint(ui, painter, timeline_rect, tf, selected);
        }
    }

    pub fn clip_containing_pos(&self, pos: u32) -> Option<&Clip> {
        self.mem
            .iter()
            .find(|x| x.pos <= pos && pos <= x.pos + x.len)
    }

    fn iter(&self) -> impl Iterator<Item = &Clip> {
        self.mem.iter()
    }

    fn clip_has_intersection(&self, skip: u32, pos: u32, len: u32) -> bool {
        for clip in self.mem.iter() {
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
