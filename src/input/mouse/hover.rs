use crate::config::*;
use crate::editor::TabManager;
use crate::terminal::Terminal;
use crate::ui::layout::ViewportLayout;

pub fn compute_bottom_bars_y(tabs: &TabManager, layout: &ViewportLayout) -> (usize, usize) {
    let find_h = if tabs.find.is_open {
        if tabs.find.is_replace {
            66
        } else {
            36
        }
    } else {
        0
    };
    let qo_h = if tabs.quick_open.is_open { 36 } else { 0 };
    let base_y = layout.content_bottom + SCROLLBAR_THICKNESS;
    if tabs.find.is_open && tabs.quick_open.is_open {
        if tabs.quick_open_above_find {
            (base_y + qo_h, base_y)
        } else {
            (base_y, base_y + find_h)
        }
    } else {
        (base_y, base_y)
    }
}

pub fn update_terminal_tab_hover(
    terminal: &mut Terminal,
    mx: usize,
    my: usize,
    content_left: usize,
    screen_w: usize,
    screen_h: usize,
    char_w: usize,
) {
    terminal.hovered_new = false;
    terminal.hovered_tab = None;
    terminal.hovered_close_tab = None;

    if !terminal.is_open || mx < content_left || mx >= screen_w {
        return;
    }

    let term_y = screen_h.saturating_sub(terminal.height);
    let tabbar_y = term_y + 1;
    let tabbar_h = TERMINAL_TAB_BAR_HEIGHT;

    if my >= tabbar_y && my < tabbar_y + tabbar_h {
        let new_btn_w = "NEW".len() * char_w + 20;
        let strip_min_x = content_left + new_btn_w;
        let strip_max_x = screen_w;

        if mx < strip_min_x {
            terminal.hovered_new = true;
        } else {
            let mx_i32 = mx as i32;
            let mut cur_x = strip_min_x as i32 - terminal.tab_scroll_x as i32;
            for (idx, tab) in terminal.tabs.iter().enumerate() {
                let tw = tab.width(char_w) as i32;
                let tab_x0 = cur_x;
                let tab_x1 = cur_x + tw;
                cur_x += tw;

                if mx_i32 >= tab_x0 && mx_i32 < tab_x1 && mx >= strip_min_x && mx < strip_max_x {
                    terminal.hovered_tab = Some(idx);
                    if mx_i32 >= tab_x1 - 27 && mx_i32 <= tab_x1 - 7 {
                        terminal.hovered_close_tab = Some(idx);
                    }
                    break;
                }
            }
        }
    }
}

pub fn update_find_hover(
    tabs: &mut TabManager,
    layout: &ViewportLayout,
    screen_w: usize,
    char_w: usize,
    mx: usize,
    my: usize,
    bar_y: usize,
) {
    tabs.find.hovered_btn = None;
    if !tabs.find.is_open {
        return;
    }

    let bar_h = if tabs.find.is_replace { 66 } else { 36 };
    let bar_x = layout.content_left;
    let bar_w = screen_w.saturating_sub(bar_x);

    if my < bar_y || my >= bar_y + bar_h || mx < bar_x || mx >= bar_x + bar_w {
        return;
    }

    let cw = char_w.max(1);
    let input_h: usize = 24;
    let input_y = bar_y + 6;
    let bottom_row_y = if tabs.find.is_replace {
        bar_y + 36
    } else {
        bar_y + 6
    };

    let close_w = "Close".len() * cw + 16;
    let close_btn_x = (bar_x + bar_w).saturating_sub(close_w + 6);
    if mx >= close_btn_x
        && mx < close_btn_x + close_w
        && my >= bottom_row_y
        && my < bottom_row_y + input_h
    {
        tabs.find.hovered_btn = Some(16);
        return;
    }

    let strip_min_x = bar_x;
    let toggle_label = if tabs.find.is_replace { "[-]" } else { "[+]" };
    let toggle_w = (toggle_label.len() * cw + 6) as i32;
    let toggle_x = strip_min_x as i32 + 6;

    if my >= bottom_row_y && my < bottom_row_y + input_h {
        let mx_i = mx as i32;
        if mx_i >= toggle_x && mx_i < toggle_x + toggle_w {
            tabs.find.hovered_btn = Some(10);
            return;
        }
    }

    let scrollable_min_x = toggle_x as usize + toggle_w as usize + 6;
    let strip_max_x = close_btn_x.saturating_sub(6);
    let mut cur_x = scrollable_min_x as i32 - tabs.find.scroll_x as i32;
    let find_input_w: usize = 240;
    cur_x += find_input_w as i32 + 6;

    let mc_w = ("Match Case".len() * cw + 16) as i32;
    let ww_w = ("Whole Word".len() * cw + 16) as i32;
    let re_w = ("Regex".len() * cw + 16) as i32;
    let prev_w = ("Previous".len() * cw + 16) as i32;
    let next_w = ("Next".len() * cw + 16) as i32;
    let has_matches = !tabs.find.matches.is_empty();

    if my >= input_y && my < input_y + input_h {
        let mx_i = mx as i32;
        if mx_i >= cur_x && mx_i < cur_x + mc_w && mx >= scrollable_min_x && mx < strip_max_x {
            tabs.find.hovered_btn = Some(11);
            return;
        }
        cur_x += mc_w + 6;
        if mx_i >= cur_x && mx_i < cur_x + ww_w && mx >= scrollable_min_x && mx < strip_max_x {
            tabs.find.hovered_btn = Some(12);
            return;
        }
        cur_x += ww_w + 6;
        if mx_i >= cur_x && mx_i < cur_x + re_w && mx >= scrollable_min_x && mx < strip_max_x {
            tabs.find.hovered_btn = Some(13);
            return;
        }
        cur_x += re_w + 6;
        if mx_i >= cur_x && mx_i < cur_x + prev_w && mx >= scrollable_min_x && mx < strip_max_x {
            if has_matches {
                tabs.find.hovered_btn = Some(14);
            }
            return;
        }
        cur_x += prev_w + 6;
        if mx_i >= cur_x && mx_i < cur_x + next_w && mx >= scrollable_min_x && mx < strip_max_x {
            if has_matches {
                tabs.find.hovered_btn = Some(15);
            }
            return;
        }
    }

    if tabs.find.is_replace {
        let rep_input_y = bottom_row_y;
        if my >= rep_input_y && my < rep_input_y + input_h {
            let mut r_cur_x = scrollable_min_x as i32 - tabs.find.scroll_x as i32;
            let rep_input_w: usize = 240;
            r_cur_x += rep_input_w as i32 + 6;
            let rep_w = ("Replace".len() * cw + 16) as i32;
            let all_w = ("Replace All".len() * cw + 16) as i32;
            let mx_i = mx as i32;
            if mx_i >= r_cur_x
                && mx_i < r_cur_x + rep_w
                && mx >= scrollable_min_x
                && mx < strip_max_x
            {
                if has_matches {
                    tabs.find.hovered_btn = Some(20);
                }
                return;
            }
            r_cur_x += rep_w + 6;
            if mx_i >= r_cur_x
                && mx_i < r_cur_x + all_w
                && mx >= scrollable_min_x
                && mx < strip_max_x
            {
                if has_matches {
                    tabs.find.hovered_btn = Some(21);
                }
            }
        }
    }
}

pub fn update_quick_open_hover(
    tabs: &mut TabManager,
    layout: &ViewportLayout,
    screen_w: usize,
    char_w: usize,
    mx: usize,
    my: usize,
    qo_y: usize,
) {
    tabs.quick_open.hovered_close = false;
    tabs.quick_open.hovered_match = None;
    if !tabs.quick_open.is_open {
        return;
    }

    let bar_x = layout.content_left;
    let bar_w = screen_w.saturating_sub(bar_x);
    let cw = char_w.max(1);
    let input_h: usize = 24;
    let input_y = qo_y + 6;
    let close_w = "Close".len() * cw + 16;
    let close_btn_x = (bar_x + bar_w).saturating_sub(close_w + 6);

    if mx >= close_btn_x && mx < close_btn_x + close_w && my >= input_y && my < input_y + input_h {
        tabs.quick_open.hovered_close = true;
        return;
    }

    let max_visible_items = 8;
    let item_count = tabs.quick_open.matches.len().min(max_visible_items);
    if item_count > 0 {
        let row_h = 28;
        let popup_h = item_count * row_h;
        let popup_y = qo_y.saturating_sub(popup_h);
        let popup_x = bar_x;
        let popup_w = bar_w;

        if mx >= popup_x && mx < popup_x + popup_w && my >= popup_y && my < popup_y + popup_h {
            let selected = tabs.quick_open.selected_match;
            let start_idx = if selected >= max_visible_items {
                selected - max_visible_items + 1
            } else {
                0
            };
            let row_idx = (my - popup_y) / row_h;
            let match_idx = start_idx + row_idx;
            if match_idx < tabs.quick_open.matches.len() {
                tabs.quick_open.hovered_match = Some(match_idx);
            }
        }
    }
}

pub fn update_tab_hover(
    tabs: &mut TabManager,
    layout: &ViewportLayout,
    char_w: usize,
    mx: usize,
    my: usize,
) {
    tabs.hovered_tab = None;
    tabs.hovered_close = None;
    if !tabs.tabs.is_empty() && my < TAB_BAR_HEIGHT && mx >= layout.content_left {
        let mx_i32 = mx as i32;
        let mut tx = layout.content_left as i32 - tabs.scroll_x as i32;
        for (idx, tab) in tabs.tabs.iter().enumerate() {
            let tw = tab.width(char_w) as i32;
            let tab_x0 = tx;
            let tab_x1 = tx + tw;
            if mx_i32 >= tab_x0 && mx_i32 < tab_x1 {
                tabs.hovered_tab = Some(idx);
                if mx_i32 >= tab_x1 - 27 && mx_i32 <= tab_x1 - 7 {
                    tabs.hovered_close = Some(idx);
                }
                break;
            }
            tx += tw;
        }
    }
}
