/// ORPH logo WIDE: ANSI/BBS-inspired brand lockup for ≥ 72 cols
pub const LOGO_WIDE: &str = r#"
      .               .
     / \             / \         ____  ____  ____  __  __
    (   )           (   )       / __ \/ __ \/ __ \/ / / /   🍓 ORPH HARNESS
     \_/             \_/       / / / / /_/ / /_/ / /_/ /    sxnnyside project
      │   .---.---.   │       / /_/ / _, _/ ____/ __  /     local-first harness
      └──/  o   o  \──┘       \____/_/ |_/_/   /_/ /_/      resilient & offline
"#;

/// ORPH logo COMPACT: Minimal retro design for smaller terminals
pub const LOGO_COMPACT: &str = r#"
   (o)   ORPH HARNESS // local-first
  (🍓)  sxnnyside project
   '-'
"#;

/// Animated pet ASCII - returns art based on mood and optional frame for animations
pub fn pet_ascii(mood: &str, frame: u64) -> String {
    // Determine eye state based on frame count (blink on every ~30 frames)
    let is_blinking = (frame / 2) % 15 == 0;

    // Determine breathing state (raise/lower ears)
    let breathing_lift = (frame / 4) % 2 == 0;

    let ears = if breathing_lift {
        "   \\│/"
    } else {
        "   /│\\"
    };

    let face = match mood {
        "hungry" => {
            if is_blinking {
                "│- -│"
            } else {
                "│. .│"
            }
        }
        "sad" => "│_ _│",
        "critical" => "│x x│",
        "playful" => {
            if is_blinking {
                "│~ ~│"
            } else {
                "│* *│"
            }
        }
        "sleepy" => "│z z│",
        "alert" => "│o o│",
        _ => {
            if is_blinking {
                "│~ ~│"
            } else {
                "│- -│"
            }
        }
    };

    format!("{}\n  ╭───╮\n  {}\n  ╰───╯", ears, face)
}
