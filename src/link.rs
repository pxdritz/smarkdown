pub fn find_link_at(line: &str, target_char: usize) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' {
            if let Some(close_bracket) = chars[i + 1..].iter().position(|&c| c == ']') {
                let text_end = i + 1 + close_bracket;
                if chars.get(text_end + 1) == Some(&'(') {
                    if let Some(close_paren) = chars[text_end + 2..].iter().position(|&c| c == ')') {
                        let paren_end = text_end + 2 + close_paren;
                        if target_char >= i && target_char <= paren_end {
                            let url: String = chars[text_end + 2..paren_end].iter().collect();
                            return Some(url);
                        }
                        i = paren_end + 1;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    None
}

pub fn open_url(url: &str) {
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}
