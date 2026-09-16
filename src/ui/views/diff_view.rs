use crate::config::*;
use crate::editor::tabs::Tab;
use crate::git::diff::DiffLineKind;
use crate::syntax::{self, Language};
use crate::ui::canvas::draw_solid_rect;
use crate::ui::font::FontManager;
use crate::ui::layout::{calc_thumb, ViewportLayout};

pub fn render_diff_view(
    frame: &mut [u32],
    fonts: &mut FontManager,
    tab: &Tab,
    layout: &ViewportLayout,
    screen_w: usize,
    screen_h: usize,
) {
    let Some(ref diff) = tab.diff else {
        return;
    };

    let cw = fonts.char_width;
    let lh = fonts.line_height;
    let total_lines = diff.lines.len();

    let digits = if total_lines == 0 {
        1
    } else {
        (total_lines.ilog10() + 1) as usize
    }
    .max(3);

    let gutter_w = GUTTER_PADDING * 2 + (digits * 2 + 2) * cw;
    let diff_code_x = layout.content_left + gutter_w + CODE_LEFT_MARGIN;
    let gutter_x = layout.content_left;
    let gutter_h = layout.content_bottom.saturating_sub(TAB_BAR_HEIGHT) + SCROLLBAR_THICKNESS;

    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        gutter_x,
        TAB_BAR_HEIGHT,
        gutter_w,
        gutter_h,
        COLOR_GUTTER_BACKGROUND,
    );
    draw_solid_rect(
        frame,
        screen_w,
        screen_h,
        gutter_x + gutter_w,
        TAB_BAR_HEIGHT,
        1,
        gutter_h,
        COLOR_GUTTER_SEPARATOR,
    );

    let language = Language::from_path(tab.buffer.file_path.as_deref());

    let mut in_comment_state = false;
    if !matches!(
        language,
        Language::PlainText | Language::Bash | Language::Json
    ) {
        let limit = tab.buffer.scroll_line.min(total_lines);
        let mut pre_chars = Vec::with_capacity(128);
        for i in 0..limit {
            pre_chars.clear();
            pre_chars.extend(
                diff.lines[i]
                    .text
                    .chars()
                    .take_while(|&c| c != '\n' && c != '\r'),
            );
            let (_, next_state) =
                syntax::highlight_line(&pre_chars, language, in_comment_state, COLOR_TEXT_DEFAULT);
            in_comment_state = next_state;
        }
    }

    let mut line_chars = Vec::with_capacity(128);

    for row in 0..=layout.visible_lines {
        let line_idx = tab.buffer.scroll_line + row;
        if line_idx >= total_lines {
            break;
        }
        let y = TAB_BAR_HEIGHT + TOP_PADDING + row * lh;
        if y + lh > layout.content_bottom {
            break;
        }

        let diff_line = &diff.lines[line_idx];

        let (row_bg, row_border, text_color, sign_char) = match diff_line.kind {
            DiffLineKind::Added => (
                Some(COLOR_GIT_DIFF_ADDED_BG),
                Some(COLOR_GIT_DIFF_ADDED_BORDER),
                COLOR_GIT_TEXT_ADDED,
                '+',
            ),
            DiffLineKind::Deleted => (
                Some(COLOR_GIT_DIFF_DELETED_BG),
                Some(COLOR_GIT_DIFF_DELETED_BORDER),
                COLOR_GIT_TEXT_DELETED,
                '-',
            ),
            DiffLineKind::Context => (None, None, COLOR_TEXT_DEFAULT, ' '),
        };

        if let Some(bg) = row_bg {
            let row_w = layout.content_right.saturating_sub(layout.content_left);
            draw_solid_rect(
                frame,
                screen_w,
                screen_h,
                layout.content_left,
                y,
                row_w,
                lh,
                bg,
            );
        }
        if let Some(border) = row_border {
            draw_solid_rect(
                frame,
                screen_w,
                screen_h,
                layout.content_left + gutter_w - 3,
                y,
                3,
                lh,
                border,
            );
        }

        let mut nx = gutter_x + GUTTER_PADDING;
        let old_str = match diff_line.old_lineno {
            Some(num) => format!("{:>width$}", num, width = digits),
            None => " ".repeat(digits),
        };
        for ch in old_str.chars() {
            fonts.draw_char(
                frame,
                ch,
                nx as i32,
                y as i32,
                screen_w,
                screen_h,
                COLOR_LINE_NUMBER_MUTED,
            );
            nx += cw;
        }

        nx += cw;

        let new_str = match diff_line.new_lineno {
            Some(num) => format!("{:>width$}", num, width = digits),
            None => " ".repeat(digits),
        };
        for ch in new_str.chars() {
            fonts.draw_char(
                frame,
                ch,
                nx as i32,
                y as i32,
                screen_w,
                screen_h,
                COLOR_LINE_NUMBER_MUTED,
            );
            nx += cw;
        }

        nx += cw / 2;
        let sign_color = match diff_line.kind {
            DiffLineKind::Added => COLOR_GIT_ADDED,
            DiffLineKind::Deleted => COLOR_GIT_DELETED,
            DiffLineKind::Context => COLOR_LINE_NUMBER_MUTED,
        };
        fonts.draw_char(
            frame, sign_char, nx as i32, y as i32, screen_w, screen_h, sign_color,
        );

        line_chars.clear();
        line_chars.extend(
            diff_line
                .text
                .chars()
                .take_while(|&c| c != '\n' && c != '\r'),
        );
        let (syntax_colors, next_comment_state) =
            syntax::highlight_line(&line_chars, language, in_comment_state, text_color);
        in_comment_state = next_comment_state;

        let mut current_vcol = 0;
        for (char_idx_in_line, &ch) in line_chars.iter().enumerate() {
            let char_w_cols = if ch == '\t' {
                TAB_WIDTH - (current_vcol % TAB_WIDTH)
            } else {
                1
            };
            let start_vcol = current_vcol;
            let end_vcol = current_vcol + char_w_cols;
            current_vcol = end_vcol;

            if end_vcol <= tab.buffer.scroll_col {
                continue;
            }
            let text_x =
                diff_code_x as i32 + (start_vcol as i32 - tab.buffer.scroll_col as i32) * cw as i32;
            if text_x >= layout.content_right as i32 {
                break;
            }

            let char_color = syntax_colors
                .get(char_idx_in_line)
                .copied()
                .unwrap_or(text_color);

            if ch != '\t' && !ch.is_control() && !ch.is_whitespace() {
                fonts.draw_char(frame, ch, text_x, y as i32, screen_w, screen_h, char_color);
            }
        }
    }

    let usable_track_h = layout.content_bottom.saturating_sub(TAB_BAR_HEIGHT);
    let virtual_total_lines = total_lines + layout.visible_lines.saturating_sub(1);
    let vert_thumb = calc_thumb(
        virtual_total_lines,
        layout.visible_lines,
        tab.buffer.scroll_line,
        usable_track_h,
    );

    if let Some((ty, th)) = vert_thumb {
        let thumb_y = TAB_BAR_HEIGHT + ty;
        draw_solid_rect(
            frame,
            screen_w,
            screen_h,
            layout.content_right,
            TAB_BAR_HEIGHT,
            SCROLLBAR_THICKNESS,
            usable_track_h,
            COLOR_BACKGROUND,
        );
        draw_solid_rect(
            frame,
            screen_w,
            screen_h,
            layout.content_right,
            thumb_y,
            SCROLLBAR_THICKNESS,
            th,
            COLOR_SCROLLBAR_THUMB,
        );
    }
}
