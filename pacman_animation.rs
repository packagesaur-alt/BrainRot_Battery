use std::io::{self, Write};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Show Pac-Man animation similar to pacman -V with mouth movement
fn show_pacman_animation(elapsed_secs: u64) {
    let animation_frame = elapsed_secs % 6; // 6-frame animation cycle for more variety
    
    // Clear screen and move to top
    print!("\x1b[2J\x1b[H");
    
    // Animated Pac-Man logo with different mouth states
    let pacman_logo = match animation_frame {
        0 => r#"
 .--.                  Pacman v7.0.0 - libalpm v15.0.0
/ _.-' .-.  .-.  .-.   Copyright (C) 2006-2024 Pacman Development Team
\  '-. '-'  '-'  '-'   Copyright (C) 2002-2006 Judd Vinet
 '--'                   This program may be freely redistributed under
                        the terms of the GNU General Public License.
"#,
        1 => r#"
 .--.                  Pacman v7.0.0 - libalpm v15.0.0
/ _.-' .-.  .-.  .-.   Copyright (C) 2006-2024 Pacman Development Team
\  '-. '-'  '-'  '-'   Copyright (C) 2002-2006 Judd Vinet
 '--'                   This program may be freely redistributed under
                        the terms of the GNU General Public License.
"#,
        2 => r#"
 .--.                  Pacman v7.0.0 - libalpm v15.0.0
/ _.-' .-.  .-.  .-.   Copyright (C) 2006-2024 Pacman Development Team
\  '-. '-'  '-'  '-'   Copyright (C) 2002-2006 Judd Vinet
 '--'                   This program may be freely redistributed under
                        the terms of the GNU General Public License.
"#,
        3 => r#"
 .--.                  Pacman v7.0.0 - libalpm v15.0.0
/ _.-' .-.  .-.  .-.   Copyright (C) 2006-2024 Pacman Development Team
\  '-. '-'  '-'  '-'   Copyright (C) 2002-2006 Judd Vinet
 '--'                   This program may be freely redistributed under
                        the terms of the GNU General Public License.
"#,
        4 => r#"
 .--.                  Pacman v7.0.0 - libalpm v15.0.0
/ _.-' .-.  .-.  .-.   Copyright (C) 2006-2024 Pacman Development Team
\  '-. '-'  '-'  '-'   Copyright (C) 2002-2006 Judd Vinet
 '--'                   This program may be freely redistributed under
                        the terms of the GNU General Public License.
"#,
        _ => r#"
 .--.                  Pacman v7.0.0 - libalpm v15.0.0
/ _.-' .-.  .-.  .-.   Copyright (C) 2006-2024 Pacman Development Team
\  '-. '-'  '-'  '-'   Copyright (C) 2002-2006 Judd Vinet
 '--'                   This program may be freely redistributed under
                        the terms of the GNU General Public License.
"#,
    };
    
    // Add animated dots that move across the screen with different patterns
    let dots_position = (elapsed_secs * 3) % 60; // Moving dots
    let dots = match animation_frame {
        0..=1 => "●●●",
        2..=3 => "●●",
        4..=5 => "●",
        _ => "●●●",
    };
    let spaces_before = " ".repeat(dots_position as usize);
    let spaces_after = " ".repeat(60 - dots_position as usize);
    
    // Add blinking cursor effect
    let cursor = if elapsed_secs % 2 == 0 { "█" } else { " " };
    
    // Print with color effects
    let color_start = "\x1b[33m"; // Yellow color
    let color_end = "\x1b[0m";
    
    println!("{}{}{}{}{}{}{}", color_start, pacman_logo, color_end, spaces_before, dots, spaces_after, cursor);
    
    // Add some additional animation elements
    let progress_bar = "█".repeat((elapsed_secs % 20) as usize) + &"░".repeat(20 - (elapsed_secs % 20) as usize);
    println!("\n\x1b[36mProgress: [{}]\x1b[0m", progress_bar);
    
    // Add animated status
    let status_frames = ["Initializing...", "Loading...", "Ready!", "Running..."];
    let status = status_frames[elapsed_secs as usize % status_frames.len()];
    println!("\x1b[32mStatus: {}\x1b[0m", status);
    
    io::stdout().flush().unwrap();
}

fn main() {
    println!("🎮 Starting Pac-Man Animation Demo...");
    println!("Press Ctrl+C to exit");
    thread::sleep(Duration::from_millis(1000));
    
    let start_time = SystemTime::now();
    let mut frame_count = 0;
    
    loop {
        let elapsed = start_time.elapsed().unwrap().as_secs();
        show_pacman_animation(elapsed);
        
        frame_count += 1;
        println!("\n\x1b[37mFrame: {} | Elapsed: {}s\x1b[0m", frame_count, elapsed);
        
        // Wait 500ms between frames for smooth animation
        thread::sleep(Duration::from_millis(500));
    }
}
