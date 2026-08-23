use unicode_width::UnicodeWidthChar;

fn char_width(c: char) -> usize {
    c.width().unwrap_or(1)
}

pub fn wrap_segments(line: &str, width: u16) -> Vec<(usize, usize)> {
    let chars: Vec<char> = line.chars().collect();
    if width == 0 || chars.is_empty() {
        return vec![(0, chars.len())];
    }
    let width = width as usize;
    let mut segments = Vec::new();
    let mut seg_start = 0usize;
    let mut col = 0usize;
    let mut last_break: Option<usize> = None;

    let mut i = 0usize;
    while i < chars.len() {
        let w = char_width(chars[i]);
        if col + w > width {
            if let Some(b) = last_break.filter(|&b| b > seg_start) {
                segments.push((seg_start, b));
                seg_start = b;
            } else {
                let end = i.max(seg_start + 1);
                segments.push((seg_start, end));
                seg_start = end;
            }
            col = chars[seg_start..i.max(seg_start)]
                .iter()
                .map(|&c| char_width(c))
                .sum();
            last_break = None;
            continue;
        }
        if chars[i] == ' ' {
            last_break = Some(i + 1);
        }
        col += w;
        i += 1;
    }
    segments.push((seg_start, chars.len()));
    segments
}

pub fn ensure_visible_generic(scroll: &mut usize, cursor_idx: usize, height: usize) {
    if height == 0 {
        return;
    }
    if cursor_idx < *scroll {
        *scroll = cursor_idx;
    } else if cursor_idx >= *scroll + height {
        *scroll = cursor_idx + 1 - height;
    }
}
