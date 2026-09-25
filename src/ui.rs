use std::io::{self, IsTerminal, Write};

const RESET: &str = "\x1b[0m";
const BOLD_CYAN: &str = "\x1b[1;36m";
const GREEN: &str = "\x1b[32m";
const VIOLET: &str = "\x1b[35m";
const YELLOW: &str = "\x1b[33m";
const DIM: &str = "\x1b[2m";

fn sanitize_terminal(text: &str) -> String {
    text.chars()
        .filter(|character| {
            !character.is_control()
                && !matches!(
                    *character,
                    '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
                )
        })
        .collect()
}

fn stdout_style(style: &str, text: &str) -> String {
    let text = sanitize_terminal(text);
    if io::stdout().is_terminal() {
        format!("{style}{text}{RESET}")
    } else {
        text
    }
}

fn stderr_style(style: &str, text: &str) -> String {
    let text = sanitize_terminal(text);
    if io::stderr().is_terminal() {
        format!("{style}{text}{RESET}")
    } else {
        text
    }
}

fn line(text: &str) {
    let _ = writeln!(io::stdout().lock(), "{text}");
}

pub fn output_block(text: &str) {
    for output_line in text.lines() {
        line(output_line);
    }
}

pub fn plain(message: &str) {
    line(&stdout_style("", message));
}

pub fn banner(label: &str) {
    line("");
    line(&stdout_style(BOLD_CYAN, "╭─ ENVHOLE"));
    line(&format!("│ {}", stdout_style(DIM, label)));
    line("╰────────────────────────────────────────────");
}

pub fn payload_preview(names: &[String], size: usize) {
    let noun = if names.len() == 1 {
        "variable"
    } else {
        "variables"
    };
    line("╭─ Protected payload");
    line(&format!(
        "│ {} {noun}  ·  {}",
        names.len(),
        stdout_style(BOLD_CYAN, &format_bytes(size))
    ));
    line("│");
    for name in names {
        line(&format!(
            "│  {}={}",
            stdout_style(BOLD_CYAN, name),
            stdout_style(VIOLET, "••••••••")
        ));
    }
    line("│");
    line(&format!(
        "╰─ {}",
        stdout_style(DIM, "Values hidden · original bytes preserved")
    ));
}

pub fn step(message: &str) {
    line(&format!(
        "  {} {}",
        stdout_style(BOLD_CYAN, "◇"),
        stdout_style("", message)
    ));
}

pub fn success(message: &str) {
    line(&format!(
        "  {} {}",
        stdout_style(GREEN, "✓"),
        stdout_style("", message)
    ));
}

pub fn secure(message: &str) {
    line(&format!(
        "  {} {}",
        stdout_style(VIOLET, "◆"),
        stdout_style("", message)
    ));
}

pub fn receiver_command_box(command: &str) -> String {
    format!(
        "{}\n│ {}\n{}",
        stdout_style(BOLD_CYAN, "╭─ COPY ON THE RECEIVING MACHINE"),
        sanitize_terminal(command),
        stdout_style(BOLD_CYAN, "╰─")
    )
}

pub fn encrypted_payload_notice(size: usize) -> String {
    format!(
        "{}\n\
│  {}  {}  {}\n\
│  {}\n\
│  {}\n\
{}",
        stdout_style(VIOLET, "╭─ ENCRYPTED STREAM"),
        stdout_style(VIOLET, "█▓▒░"),
        stdout_style(BOLD_CYAN, &format_bytes(size)),
        stdout_style(VIOLET, "░▒▓█"),
        stdout_style(DIM, "End-to-end encrypted in transit"),
        stdout_style(
            DIM,
            "Ciphertext stays inside Magic Wormhole · saved file remains plaintext"
        ),
        stdout_style(VIOLET, "╰─")
    )
}

pub fn completion_card(title: &str, primary: &str, secondary: &str) -> String {
    let title = sanitize_terminal(title);
    let primary = sanitize_terminal(primary);
    let secondary = sanitize_terminal(secondary);
    let inner_width = 44usize
        .max(title.chars().count() + 3)
        .max(primary.chars().count().max(secondary.chars().count()) + 4);
    let top = format!(
        "╭─ {title} {}╮",
        "─".repeat(inner_width - title.chars().count() - 3)
    );
    let primary_padding = " ".repeat(inner_width - primary.chars().count() - 4);
    let secondary_padding = " ".repeat(inner_width - secondary.chars().count() - 4);
    let bottom = format!("╰{}╯", "─".repeat(inner_width));

    format!(
        "{}\n│ {} {}{} │\n│ {} {}{} │\n{}",
        stdout_style(BOLD_CYAN, &top),
        stdout_style(GREEN, "✓"),
        stdout_style("", &primary),
        primary_padding,
        stdout_style(VIOLET, "◆"),
        stdout_style(DIM, &secondary),
        secondary_padding,
        stdout_style(BOLD_CYAN, &bottom)
    )
}

pub fn complete(title: &str, primary: &str, secondary: &str) {
    output_block(&completion_card(title, primary, secondary));
}

pub fn prompt(message: &str) -> io::Result<()> {
    eprint!(
        "  {} {} [y/N] ",
        stderr_style(YELLOW, "?"),
        stderr_style("", message)
    );
    io::stderr().flush()
}

fn progress_bar(current: u64, total: u64, width: usize) -> String {
    let current = current.min(total);
    let (filled, percent) = if total == 0 {
        (width, 100)
    } else {
        (
            ((current as u128 * width as u128) / total as u128) as usize,
            ((current as u128 * 100) / total as u128) as usize,
        )
    };
    format!(
        "{}{} {percent:>3}%",
        "█".repeat(filled),
        "░".repeat(width - filled)
    )
}

fn progress_line(symbol: &str, label: &str, current: u64, total: u64) -> String {
    format!(
        "{} {:<10}{}   {} / {}",
        sanitize_terminal(symbol),
        sanitize_terminal(label),
        progress_bar(current, total, 20),
        format_bytes(current.min(total) as usize),
        format_bytes(total as usize)
    )
}

pub struct TransferProgress {
    symbol: &'static str,
    label: &'static str,
    active: bool,
}

impl TransferProgress {
    pub fn sending() -> Self {
        Self {
            symbol: "⇡",
            label: "Sending",
            active: false,
        }
    }

    pub fn receiving() -> Self {
        Self {
            symbol: "⇣",
            label: "Receiving",
            active: false,
        }
    }

    pub fn update(&mut self, current: u64, total: u64) {
        if !io::stdout().is_terminal() {
            return;
        }
        let completed = current >= total;
        let rendered = stdout_style(
            BOLD_CYAN,
            &progress_line(self.symbol, self.label, current, total),
        );
        let mut stdout = io::stdout().lock();
        let _ = write!(stdout, "\r\x1b[2K  {rendered}");
        if completed {
            let _ = writeln!(stdout);
        } else {
            let _ = stdout.flush();
        }
        self.active = !completed;
    }
}

impl Drop for TransferProgress {
    fn drop(&mut self) {
        if self.active && io::stdout().is_terminal() {
            let mut stdout = io::stdout().lock();
            let _ = write!(stdout, "\r\x1b[2K");
            let _ = stdout.flush();
        }
    }
}

pub fn format_bytes(bytes: usize) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1_048_576 {
        format!("{:.1} KiB", bytes as f64 / KIB)
    } else {
        format!("{:.1} MiB", bytes as f64 / MIB)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        completion_card, encrypted_payload_notice, format_bytes, progress_bar, progress_line,
        receiver_command_box, sanitize_terminal,
    };

    #[test]
    fn receiver_command_is_presented_as_a_copyable_block() {
        let rendered = receiver_command_box("envhole receive '7-code' --output .env.received");
        assert_eq!(
            rendered,
            "╭─ COPY ON THE RECEIVING MACHINE\n\
│ envhole receive '7-code' --output .env.received\n\
╰─"
        );
    }

    #[test]
    fn encryption_notice_is_honest_about_ciphertext_visibility() {
        let rendered = encrypted_payload_notice(1536);
        assert!(rendered.contains("1.5 KiB"));
        assert!(rendered.contains("End-to-end encrypted in transit"));
        assert!(rendered.contains("Ciphertext stays inside Magic Wormhole"));
        assert!(rendered.contains("saved file remains plaintext"));
    }

    #[test]
    fn terminal_controls_and_bidi_overrides_are_removed() {
        assert_eq!(
            sanitize_terminal("safe\x1b[31mred\n\u{202E}txt"),
            "safe[31mredtxt"
        );
    }

    #[test]
    fn progress_bar_is_bounded_and_reports_percentage() {
        assert_eq!(progress_bar(512, 1024, 10), "█████░░░░░  50%");
        assert_eq!(progress_bar(2048, 1024, 10), "██████████ 100%");
    }

    #[test]
    fn progress_line_is_compact_and_informative() {
        assert_eq!(
            progress_line("⇡", "Sending", 512, 1024),
            "⇡ Sending   ██████████░░░░░░░░░░  50%   512 B / 1.0 KiB"
        );
    }

    #[test]
    fn completion_card_reads_like_a_transfer_receipt() {
        let rendered = completion_card(
            "TRANSFER COMPLETE",
            "Sent 1.5 KiB",
            "Peer received the exact bytes",
        );
        let lines: Vec<_> = rendered.lines().collect();

        assert!(lines[0].starts_with("╭─ TRANSFER COMPLETE ") && lines[0].ends_with('╮'));
        assert!(lines[1].starts_with("│ ✓ Sent 1.5 KiB") && lines[1].ends_with('│'));
        assert!(lines[2].contains("◆ Peer received the exact bytes"));
        assert!(lines[3].starts_with('╰') && lines[3].ends_with('╯'));
        assert!(
            lines
                .windows(2)
                .all(|pair| { pair[0].chars().count() == pair[1].chars().count() })
        );
    }

    #[test]
    fn byte_sizes_are_human_readable() {
        assert_eq!(format_bytes(999), "999 B");
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_bytes(1_048_576), "1.0 MiB");
    }
}
