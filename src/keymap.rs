pub fn key_name(code: u16) -> String {
    match code {
        29 | 97 => "Ctrl".to_string(),
        42 | 54 => "Shift".to_string(),
        56 | 100 => "Alt".to_string(),
        125 | 126 => "Super".to_string(),

        // Mouse Buttons
        272 => "Left Click".to_string(),
        273 => "Right Click".to_string(),
        274 => "Middle Click".to_string(),
        275 => "Side Click".to_string(),
        276 => "Extra Click".to_string(),

        // Standard Keys
        57 => "Space".to_string(),
        28 => "Enter".to_string(),
        1 => "Esc".to_string(),
        14 => "Backspace".to_string(),
        15 => "Tab".to_string(),
        183 => "F13".to_string(), 184 => "F14".to_string(), 185 => "F15".to_string(),
        186 => "F16".to_string(), 187 => "F17".to_string(), 188 => "F18".to_string(),
        189 => "F19".to_string(), 190 => "F20".to_string(), 191 => "F21".to_string(),
        192 => "F22".to_string(), 193 => "F23".to_string(), 194 => "F24".to_string(),
        102 => "Home".to_string(),
        107 => "End".to_string(),
        104 => "Page Up".to_string(),
        109 => "Page Down".to_string(),
        110 => "Insert".to_string(),
        111 => "Delete".to_string(),
        103 => "Up Arrow".to_string(),
        108 => "Down Arrow".to_string(),
        105 => "Left Arrow".to_string(),
        106 => "Right Arrow".to_string(),
        69 => "Num Lock".to_string(),
        70 => "Scroll Lock".to_string(),
        119 => "Pause".to_string(),
        73 => "Num Clear".to_string(), // Often mapped to Num Lock / Clear on Apple keyboards

        // Punctuation
        12 => "Minus".to_string(), 13 => "Equals".to_string(),
        26 => "Left Bracket".to_string(), 27 => "Right Bracket".to_string(),
        39 => "Semicolon".to_string(), 40 => "Apostrophe".to_string(),
        41 => "Grave".to_string(), 43 => "Backslash".to_string(),
        51 => "Comma".to_string(), 52 => "Period".to_string(), 53 => "Slash".to_string(),

        _ => {
            // Clean array lookup for Letters A-Z
            let letters = [
                (30, 'A'), (48, 'B'), (46, 'C'), (32, 'D'), (18, 'E'), (33, 'F'), (34, 'G'), (35, 'H'),
                (23, 'I'), (36, 'J'), (37, 'K'), (38, 'L'), (50, 'M'), (49, 'N'), (24, 'O'), (25, 'P'),
                (16, 'Q'), (19, 'R'), (31, 'S'), (20, 'T'), (22, 'U'), (47, 'V'), (17, 'W'), (45, 'X'),
                (21, 'Y'), (44, 'Z')
            ];
            if let Some(&(_, c)) = letters.iter().find(|&&(k, _)| k == code) {
                return c.to_string();
            }

            // Numbers 1-0
            if (2..=11).contains(&code) {
                let nums = ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'];
                return nums[(code - 2) as usize].to_string();
            }

            // F1-F12
            if (59..=68).contains(&code) { return format!("F{}", code - 58); }
            if code == 87 { return "F11".to_string(); }
            if code == 88 { return "F12".to_string(); }

            // Numpad
            if code == 78 { return "Numpad +".to_string(); }
            if code == 74 { return "Numpad -".to_string(); }
            if code == 55 { return "Numpad *".to_string(); }
            if code == 98 { return "Numpad /".to_string(); }
            if code == 96 { return "Numpad Enter".to_string(); }
            if code == 83 { return "Numpad .".to_string(); }
            if (71..=73).contains(&code) || (75..=77).contains(&code) || (79..=82).contains(&code) {
                let numpad_nums = ['7', '8', '9', '4', '5', '6', '1', '2', '3', '0'];
                let numpad_map = [71, 72, 73, 75, 76, 77, 79, 80, 81, 82];
                if let Some(idx) = numpad_map.iter().position(|&k| k == code) {
                    return format!("Numpad {}", numpad_nums[idx]);
                }
            }

            format!("Key {}", code)
        }
    }
}

pub fn combo_name(combo: &Vec<u16>) -> String {
    if combo.is_empty() { return "None".to_string(); }

    // Sort the combo so modifiers (Ctrl, Shift, Alt, Super) always appear first
    let mut sorted_combo = combo.clone();
    sorted_combo.sort_by_key(|&k| {
        match k {
            29 | 97 => 1,   // Ctrl
            42 | 54 => 2,   // Shift
            56 | 100 => 3,  // Alt
            125 | 126 => 4, // Super
            _ => 99,         // Everything else
        }
    });

    sorted_combo.iter().map(|c| key_name(*c)).collect::<Vec<String>>().join(" + ")
}
