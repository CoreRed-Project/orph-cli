/// ORPH logo WIDE: ANSI/BBS-inspired brand lockup for ≥ 72 cols
pub const LOGO_WIDE: &str = r#"
      ╔════════════════════════════════════════════════════════╗
      ║   ____   ____   ____   __  __                         ║
      ║  / __ \ / __ \ / __ \ / / / /     S X N N Y S I D E   ║
      ║ / / / // /_/ // /_/ // /_/ /      LOCAL COMPANION OS   ║
      ║/ /_/ // _, _// ____// __  /       AUTONOMOUS SHELL     ║
      ║\____//_/ |_|/_/    /_/ /_/        SIGNAL KEPT LOCAL    ║
      ╚═══════════════[ ORPH ]═══════[ SMALL MACHINE SOUL ]════╝
"#;

/// ORPH logo COMPACT: Minimal retro design for smaller terminals
pub const LOGO_COMPACT: &str = r#"
  ____  ____  ____  __  __
 / __ \/ __ \/ __ \/ / / /
/ /_/ / /_/ / /_/ / /_/ /
\____/_/ |_/_/    /_/ /_/
 ORPH // LOCAL COMPANION OS
"#;

/// Animated pet ASCII - returns art based on mood and optional frame for animations
pub fn pet_ascii(mood: &str) -> &'static str {
    match mood {
        "happy" => PET_HAPPY,
        "hungry" => PET_HUNGRY,
        "sad" => PET_SAD,
        "critical" => PET_CRITICAL,
        _ => PET_CONTENT,
    }
}

/// HAPPY state: Digital companion rejoicing with electromagnetic energy
const PET_HAPPY: &str = r#"
             /\_____/\ 
            /  ^   ^  \
           /==   v   ==\
           \   \___/   /
            '._______.' 
             /|  _  |\
            /_| |_| |_\
           /  |  |  |  \
          /___|__|__|___\
             /_/   \_\
          __/ /     \ \__
         /___/  JOY  \___\
"#;

/// CONTENT state: Default peaceful state
const PET_CONTENT: &str = r#"
             /\_____/\ 
            /  o   o  \
           /==   v   ==\
           \    ___    /
            '._______.' 
             /|  _  |\
            /_| |_| |_\
           /  |  |  |  \
          /___|__|__|___\
             /_/   \_\
          __/ /     \ \__
         /___/ IDLE  \___\
"#;

/// HUNGRY state: Companion in need, seeking sustenance
const PET_HUNGRY: &str = r#"
             /\_____/\ 
            /  .   .  \
           /==   v   ==\
           \    ---    /
            '._______.' 
             /|  _  |\
            /_| |_| |_\
           /  |  |  |  \
          /___|__|__|___\
             /_/   \_\
          __/ /     \ \__
         /___/ FEED  \___\
"#;

/// SAD state: Lonely companion, isolation detected
const PET_SAD: &str = r#"
             /\_____/\ 
            /  -   -  \
           /==   v   ==\
           \    ___    /
            '._____\_.' 
             /|  _  |\
            /_| |_| |_\
           /  |  |  |  \
          /___|__|__|___\
             /_/   \_\
          __/ /     \ \__
         /___/ LOW   \___\
"#;

/// CRITICAL state: System overload, companion distress
const PET_CRITICAL: &str = r#"
             /\_____/\ 
            /  x   x  \
           /==   !   ==\
           \   _____   /
            '._______.' 
             /| !!! |\
            /_| !!! |_\
           /  | !!! |  \
          /___|__!__|___\
             /_/   \_\
          __/ /     \ \__
         /___/ CARE  \___\
"#;
