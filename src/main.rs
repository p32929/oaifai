// oaifai — a full-screen TUI to scan and connect to WiFi networks.
//
// System mechanism (Ubuntu Server, no NetworkManager):
//   * scan    -> `iw dev <iface> scan`
//   * connect -> rewrite /etc/netplan/<file>.yaml + `netplan apply`
//
// Must be run as root (scanning and writing netplan both need it):
//   sudo oaifai                               # the TUI
//   sudo oaifai --list                        # plain list, no changes
//   sudo oaifai --connect "SSID" "PASSWORD"   # non-interactive
//   add --dry-run to print what would happen without applying.

use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};

const IFACE: &str = "wlp1s0";
const NETPLAN_FILE: &str = "/etc/netplan/00-installer-config.yaml";
const BACKUP_FILE: &str = "/etc/netplan/00-installer-config.yaml.oaifai.bak";
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Clone, Debug)]
struct Network {
    ssid: String,
    signal: f32, // dBm
}

// ---------- pure helpers (unit-tested) ----------

fn signal_quality(dbm: f32) -> u8 {
    let q = (dbm + 90.0) / 60.0 * 100.0;
    q.clamp(0.0, 100.0).round() as u8
}

fn signal_bars(dbm: f32) -> String {
    let n = match signal_quality(dbm) {
        0..=25 => 1,
        26..=50 => 2,
        51..=75 => 3,
        _ => 4,
    };
    let bars: String = "▂▄▆█".chars().take(n).collect();
    format!("{}{}", bars, "·".repeat(4 - n))
}

fn signal_color(dbm: f32) -> Color {
    match signal_quality(dbm) {
        0..=25 => Color::Red,
        26..=50 => Color::Yellow,
        51..=75 => Color::LightGreen,
        _ => Color::Green,
    }
}

fn truncate_ssid(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max - 1).collect();
        format!("{t}…")
    }
}

fn parse_scan(output: &str) -> Vec<Network> {
    let mut nets: Vec<Network> = Vec::new();
    let mut cur_signal: Option<f32> = None;
    for line in output.lines() {
        let t = line.trim();
        if t.starts_with("BSS ") {
            cur_signal = None;
        } else if let Some(rest) = t.strip_prefix("signal:") {
            cur_signal = rest.trim().split_whitespace().next().and_then(|v| v.parse::<f32>().ok());
        } else if let Some(rest) = t.strip_prefix("SSID:") {
            let ssid = rest.trim().to_string();
            if ssid.is_empty() || ssid.contains("\\x00") {
                continue;
            }
            let sig = cur_signal.unwrap_or(-100.0);
            if let Some(existing) = nets.iter_mut().find(|n| n.ssid == ssid) {
                if sig > existing.signal {
                    existing.signal = sig;
                }
            } else {
                nets.push(Network { ssid, signal: sig });
            }
        }
    }
    nets.sort_by(|a, b| b.signal.partial_cmp(&a.signal).unwrap_or(std::cmp::Ordering::Equal));
    nets
}

fn yaml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn generate_netplan(iface: &str, ssid: &str, password: &str) -> String {
    let ap = if password.is_empty() {
        format!("        \"{}\": {{}}\n", yaml_escape(ssid))
    } else {
        format!(
            "        \"{}\":\n          password: \"{}\"\n",
            yaml_escape(ssid),
            yaml_escape(password)
        )
    };
    format!(
        "network:\n  version: 2\n  renderer: networkd\n  wifis:\n    {iface}:\n      dhcp4: true\n      access-points:\n{ap}",
        iface = iface,
        ap = ap
    )
}

// ---------- system actions ----------

fn scan_networks() -> Result<Vec<Network>, String> {
    let run = || Command::new("iw").args(["dev", IFACE, "scan"]).output();
    let out = run().map_err(|e| format!("failed to run `iw`: {e}"))?;
    if out.status.success() {
        return Ok(parse_scan(&String::from_utf8_lossy(&out.stdout)));
    }
    let err = String::from_utf8_lossy(&out.stderr);
    if err.contains("Operation not permitted") || err.contains("Permission denied") {
        return Err("permission denied while scanning — run with sudo".into());
    }
    if err.to_lowercase().contains("busy") {
        thread::sleep(Duration::from_secs(2));
        if let Ok(out2) = run() {
            if out2.status.success() {
                return Ok(parse_scan(&String::from_utf8_lossy(&out2.stdout)));
            }
        }
    }
    Err(format!("scan failed: {}", err.trim()))
}

fn current_ssid() -> Option<String> {
    let out = Command::new("iw").args(["dev", IFACE, "link"]).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    if s.contains("Not connected") {
        return None;
    }
    for l in s.lines() {
        if let Some(v) = l.trim().strip_prefix("SSID:") {
            let v = v.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn current_ip() -> Option<String> {
    let out = Command::new("ip").args(["-4", "-o", "addr", "show", IFACE]).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    for l in s.lines() {
        if let Some(idx) = l.find("inet ") {
            if let Some(ip) = l[idx + 5..].split('/').next() {
                let ip = ip.trim();
                if !ip.is_empty() {
                    return Some(ip.to_string());
                }
            }
        }
    }
    None
}

fn is_connected_to(ssid: &str) -> bool {
    current_ssid().as_deref() == Some(ssid)
}

/// Write netplan config, apply it, and verify connectivity. Returns the IP on success.
fn connect(ssid: &str, password: &str, dry_run: bool) -> Result<String, String> {
    let yaml = generate_netplan(IFACE, ssid, password);

    if dry_run {
        return Err(format!("dry-run — would write:\n{yaml}"));
    }

    if std::path::Path::new(NETPLAN_FILE).exists() {
        std::fs::copy(NETPLAN_FILE, BACKUP_FILE).map_err(|e| format!("backup failed: {e} (run with sudo?)"))?;
    }
    std::fs::write(NETPLAN_FILE, &yaml).map_err(|e| format!("writing netplan failed: {e} (run with sudo?)"))?;
    let _ = std::fs::set_permissions(NETPLAN_FILE, std::fs::Permissions::from_mode(0o600));

    let out = Command::new("netplan").arg("apply").output().map_err(|e| format!("`netplan apply` failed: {e}"))?;
    if !out.status.success() {
        restore_backup();
        return Err(format!("netplan apply error: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }

    for _ in 0..20 {
        thread::sleep(Duration::from_millis(1000));
        if is_connected_to(ssid) {
            if let Some(ip) = current_ip() {
                return Ok(ip);
            }
        }
    }

    restore_backup();
    Err("could not get connectivity — restored your previous network".into())
}

fn restore_backup() {
    if std::path::Path::new(BACKUP_FILE).exists() {
        let _ = std::fs::copy(BACKUP_FILE, NETPLAN_FILE);
        let _ = Command::new("netplan").arg("apply").output();
    }
}

// ---------- TUI ----------

enum Msg {
    Scanned { nets: Vec<Network>, ssid: Option<String>, ip: Option<String>, error: Option<String> },
    Connected(Result<String, String>),
}

#[derive(PartialEq)]
enum Screen {
    Loading,
    List,
    ManualSsid,
    Password,
    Connecting,
    Done,
}

struct App {
    screen: Screen,
    nets: Vec<Network>,
    selectable: Vec<bool>, // index 0 = manual entry, then one per net
    state: ListState,
    connected_ssid: Option<String>,
    connected_ip: Option<String>,
    scan_error: Option<String>,
    manual_ssid: String,
    password: String,
    show_password: bool,
    target_ssid: String,
    result: Option<Result<String, String>>,
    spinner: usize,
    dry_run: bool,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

impl App {
    fn new(dry_run: bool) -> Self {
        let mut app = App::bare(dry_run);
        app.start_scan();
        app
    }

    fn bare(dry_run: bool) -> Self {
        let (tx, rx) = mpsc::channel();
        App {
            screen: Screen::Loading,
            nets: Vec::new(),
            selectable: Vec::new(),
            state: ListState::default(),
            connected_ssid: None,
            connected_ip: None,
            scan_error: None,
            manual_ssid: String::new(),
            password: String::new(),
            show_password: false,
            target_ssid: String::new(),
            result: None,
            spinner: 0,
            dry_run,
            tx,
            rx,
        }
    }

    fn start_scan(&mut self) {
        self.screen = Screen::Loading;
        self.scan_error = None;
        let tx = self.tx.clone();
        thread::spawn(move || {
            let (nets, error) = match scan_networks() {
                Ok(n) => (n, None),
                Err(e) => (Vec::new(), Some(e)),
            };
            let _ = tx.send(Msg::Scanned { nets, ssid: current_ssid(), ip: current_ip(), error });
        });
    }

    fn start_connect(&mut self) {
        self.screen = Screen::Connecting;
        self.result = None;
        let tx = self.tx.clone();
        let ssid = self.target_ssid.clone();
        let pass = self.password.clone();
        let dry = self.dry_run;
        thread::spawn(move || {
            let _ = tx.send(Msg::Connected(connect(&ssid, &pass, dry)));
        });
    }

    fn handle_msg(&mut self, msg: Msg) {
        match msg {
            Msg::Scanned { nets, ssid, ip, error } => {
                self.connected_ssid = ssid;
                self.connected_ip = ip;
                self.scan_error = error;
                self.selectable = std::iter::once(true)
                    .chain(nets.iter().map(|n| self.connected_ssid.as_deref() != Some(n.ssid.as_str())))
                    .collect();
                self.nets = nets;
                // select the first selectable real network, else the manual entry
                let first = (1..self.selectable.len()).find(|&i| self.selectable[i]).unwrap_or(0);
                self.state.select(Some(first));
                self.screen = Screen::List;
            }
            Msg::Connected(res) => {
                self.result = Some(res);
                self.screen = Screen::Done;
            }
        }
    }

    fn move_selection(&mut self, delta: isize) {
        let n = self.selectable.len();
        if n == 0 {
            return;
        }
        let mut i = self.state.selected().unwrap_or(0) as isize;
        for _ in 0..n {
            i = (i + delta).rem_euclid(n as isize);
            if self.selectable[i as usize] {
                break;
            }
        }
        self.state.select(Some(i as usize));
    }

    /// Returns true if the app should quit.
    fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> bool {
        match self.screen {
            Screen::Loading | Screen::Connecting => {
                if self.screen == Screen::Loading && matches!(code, KeyCode::Char('q')) {
                    return true;
                }
            }
            Screen::List => match code {
                KeyCode::Char('q') | KeyCode::Esc => return true,
                KeyCode::Char('r') => self.start_scan(),
                KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
                KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
                KeyCode::Enter => {
                    match self.state.selected() {
                        Some(0) => {
                            self.manual_ssid.clear();
                            self.screen = Screen::ManualSsid;
                        }
                        Some(i) => {
                            self.target_ssid = self.nets[i - 1].ssid.clone();
                            self.password.clear();
                            self.show_password = false;
                            self.screen = Screen::Password;
                        }
                        None => {}
                    }
                }
                _ => {}
            },
            Screen::ManualSsid => match code {
                KeyCode::Esc => self.screen = Screen::List,
                KeyCode::Enter => {
                    if !self.manual_ssid.trim().is_empty() {
                        self.target_ssid = self.manual_ssid.trim().to_string();
                        self.password.clear();
                        self.show_password = false;
                        self.screen = Screen::Password;
                    }
                }
                KeyCode::Backspace => {
                    self.manual_ssid.pop();
                }
                KeyCode::Char('u') if mods.contains(KeyModifiers::CONTROL) => self.manual_ssid.clear(),
                KeyCode::Char(c) => self.manual_ssid.push(c),
                _ => {}
            },
            Screen::Password => match code {
                KeyCode::Esc => self.screen = Screen::List,
                KeyCode::Tab => self.show_password = !self.show_password,
                KeyCode::Enter => self.start_connect(),
                KeyCode::Backspace => {
                    self.password.pop();
                }
                KeyCode::Char('u') if mods.contains(KeyModifiers::CONTROL) => self.password.clear(),
                KeyCode::Char(c) => self.password.push(c),
                _ => {}
            },
            Screen::Done => match code {
                KeyCode::Char('q') | KeyCode::Esc => return true,
                _ => self.start_scan(),
            },
        }
        false
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        loop {
            while let Ok(msg) = self.rx.try_recv() {
                self.handle_msg(msg);
            }
            terminal.draw(|f| self.render(f))?;
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(k) = event::read()? {
                    if k.kind == KeyEventKind::Press && self.handle_key(k.code, k.modifiers) {
                        return Ok(());
                    }
                }
            }
            self.spinner = self.spinner.wrapping_add(1);
        }
    }

    fn render(&mut self, f: &mut Frame) {
        let area = f.area();
        let title = "  oaifai  ";
        let outer = Block::bordered()
            .title(title)
            .title_alignment(Alignment::Center)
            .border_style(Style::new().cyan());
        let inner = outer.inner(area);
        f.render_widget(outer, area);

        match self.screen {
            Screen::Loading => self.render_center(f, inner, vec![
                Line::from(Span::styled(format!("{}  Scanning for networks…", SPINNER[self.spinner % SPINNER.len()]), Style::new().cyan())),
            ], "q quit"),
            Screen::List => self.render_list(f, inner),
            Screen::ManualSsid => self.render_input(f, inner, "Network name (SSID)", &self.manual_ssid, false),
            Screen::Password => self.render_input(f, inner, &format!("Password for \"{}\"", self.target_ssid), &self.password, true),
            Screen::Connecting => self.render_center(f, inner, vec![
                Line::from(Span::styled(
                    format!("{}  Connecting to \"{}\" …", SPINNER[self.spinner % SPINNER.len()], self.target_ssid),
                    Style::new().cyan(),
                )),
            ], "please wait"),
            Screen::Done => self.render_done(f, inner),
        }
    }

    fn render_list(&mut self, f: &mut Frame, area: Rect) {
        let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

        if self.nets.is_empty() {
            if let Some(err) = self.scan_error.clone() {
                self.render_center(f, rows[0], vec![
                    Line::from(Span::styled(format!("✗  {err}"), Style::new().red())),
                ], "");
            } else {
                self.render_center(f, rows[0], vec![Line::raw("No networks found.")], "");
            }
            let footer = Line::from(Span::styled("  r rescan    q quit", Style::new().dark_gray()));
            f.render_widget(Paragraph::new(footer), rows[1]);
            return;
        }

        let mut items: Vec<ListItem> = Vec::with_capacity(self.nets.len() + 1);
        items.push(ListItem::new(Line::from(Span::styled(
            "+  Enter network name manually",
            Style::new().fg(Color::Cyan),
        ))));

        for n in &self.nets {
            let connected = self.connected_ssid.as_deref() == Some(n.ssid.as_str());
            let bars = signal_bars(n.signal);
            let name = format!("{:<22}", truncate_ssid(&n.ssid, 22));
            let line = if connected {
                let ip = self.connected_ip.clone().unwrap_or_else(|| "no IP".into());
                Line::from(vec![
                    Span::styled(format!("{bars}  "), Style::new().green()),
                    Span::styled(name, Style::new().green().add_modifier(Modifier::BOLD)),
                    Span::styled(format!("● connected · {ip}"), Style::new().green()),
                ])
            } else {
                Line::from(vec![
                    Span::styled(format!("{bars}  "), Style::new().fg(signal_color(n.signal))),
                    Span::styled(name, Style::new().white()),
                    Span::styled(format!("{} dBm", n.signal as i32), Style::new().dark_gray()),
                ])
            };
            items.push(ListItem::new(line));
        }

        let list = List::new(items)
            .highlight_symbol(" ▶ ")
            .highlight_style(Style::new().add_modifier(Modifier::BOLD).bg(Color::Rgb(40, 42, 66)));
        // pad each row's gutter so non-selected rows align with the highlight symbol
        let list = list.block(Block::default());
        f.render_stateful_widget(list, rows[0].inner(ratatui::layout::Margin::new(1, 0)), &mut self.state);

        let footer = Line::from(Span::styled(
            "  ↑/↓ move    ⏎ select    r rescan    q quit",
            Style::new().dark_gray(),
        ));
        f.render_widget(Paragraph::new(footer), rows[1]);
    }

    fn render_input(&self, f: &mut Frame, area: Rect, label: &str, value: &str, is_password: bool) {
        let shown = if is_password && !self.show_password {
            "●".repeat(value.chars().count())
        } else {
            value.to_string()
        };
        let hint = if is_password {
            "empty = open network     Tab show/hide     ⏎ connect     Esc back"
        } else {
            "⏎ continue     Esc back"
        };
        let lines = vec![
            Line::raw(""),
            Line::from(Span::styled(format!("  {label}"), Style::new().cyan().add_modifier(Modifier::BOLD))),
            Line::raw(""),
            Line::from(vec![
                Span::raw("    "),
                Span::styled(format!("{shown}▏"), Style::new().yellow()),
            ]),
            Line::raw(""),
            Line::from(Span::styled(format!("  {hint}"), Style::new().dark_gray())),
        ];
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }

    fn render_done(&mut self, f: &mut Frame, area: Rect) {
        let lines = match &self.result {
            Some(Ok(ip)) => vec![
                Line::from(Span::styled(format!("✓  Connected to \"{}\"", self.target_ssid), Style::new().green().add_modifier(Modifier::BOLD))),
                Line::raw(""),
                Line::from(Span::styled(format!("IP {ip}"), Style::new().green())),
            ],
            Some(Err(e)) => vec![
                Line::from(Span::styled(format!("✗  Failed to connect to \"{}\"", self.target_ssid), Style::new().red().add_modifier(Modifier::BOLD))),
                Line::raw(""),
                Line::from(Span::styled(e.clone(), Style::new().red())),
            ],
            None => vec![Line::raw("…")],
        };
        self.render_center(f, area, lines, "⏎ back to list    q quit");
    }

    fn render_center(&self, f: &mut Frame, area: Rect, mut lines: Vec<Line>, footer: &str) {
        let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
        let pad = (rows[0].height.saturating_sub(lines.len() as u16)) / 2;
        let mut content: Vec<Line> = (0..pad).map(|_| Line::raw("")).collect();
        content.append(&mut lines);
        f.render_widget(
            Paragraph::new(content).alignment(Alignment::Center).wrap(Wrap { trim: false }),
            rows[0],
        );
        if !footer.is_empty() {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(format!("  {footer}"), Style::new().dark_gray()))),
                rows[1],
            );
        }
    }
}

/// Render one frame of a given screen to a TestBackend and dump it as plain text.
fn snapshot(which: &str) {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut app = App::bare(false);
    let nets = scan_networks().unwrap_or_default();
    app.handle_msg(Msg::Scanned { nets, ssid: current_ssid(), ip: current_ip(), error: None });
    match which {
        "password" => {
            app.target_ssid = app.connected_ssid.clone().unwrap_or_else(|| "S1".into());
            app.password = "hunter2".into();
            app.screen = Screen::Password;
        }
        "done" => {
            app.target_ssid = app.connected_ssid.clone().unwrap_or_else(|| "S1".into());
            app.result = Some(Ok(app.connected_ip.clone().unwrap_or_else(|| "192.168.1.30".into())));
            app.screen = Screen::Done;
        }
        _ => {}
    }

    let mut terminal = Terminal::new(TestBackend::new(74, 18)).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
    let buf = terminal.backend().buffer().clone();
    let area = buf.area;
    for y in 0..area.height {
        let mut line = String::new();
        for x in 0..area.width {
            line.push_str(buf[(x, y)].symbol());
        }
        println!("{}", line.trim_end());
    }
}

fn interactive(dry_run: bool) {
    let mut terminal = ratatui::init();
    let mut app = App::new(dry_run);
    let res = app.run(&mut terminal);
    ratatui::restore();
    if let Err(e) = res {
        eprintln!("error: {e}");
    }
}

// ---------- CLI helpers / entry ----------

fn run_connect_cli(ssid: &str, pass: &str, dry_run: bool) {
    println!("Connecting to \"{ssid}\"…");
    match connect(ssid, pass, dry_run) {
        Ok(ip) => println!("✓ Connected to \"{ssid}\" — IP {ip}"),
        Err(e) => println!("✗ {e}"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dry_run = args.iter().any(|a| a == "--dry-run");

    if let Some(pos) = args.iter().position(|a| a == "--connect") {
        let ssid = args.get(pos + 1).cloned().unwrap_or_default();
        let pass = args.get(pos + 2).cloned().unwrap_or_default();
        if ssid.is_empty() {
            eprintln!("usage: oaifai --connect \"SSID\" \"PASSWORD\"");
            std::process::exit(2);
        }
        run_connect_cli(&ssid, &pass, dry_run);
        return;
    }

    if let Some(pos) = args.iter().position(|a| a == "--snapshot") {
        snapshot(args.get(pos + 1).map(|s| s.as_str()).unwrap_or("list"));
        return;
    }

    if args.iter().any(|a| a == "--list") {
        match current_ssid() {
            Some(ssid) => {
                let ip = current_ip().unwrap_or_else(|| "no IP".into());
                println!("Connected: \"{ssid}\"  —  IP {ip}\n");
            }
            None => println!("Not connected\n"),
        }
        match scan_networks() {
            Ok(nets) => {
                if nets.is_empty() {
                    println!("(no networks found)");
                }
                let active = current_ssid();
                for n in nets {
                    let mark = if active.as_deref() == Some(n.ssid.as_str()) { "  ✓" } else { "" };
                    println!("{}  {:>4} dBm  {}{}", signal_bars(n.signal), n.signal as i32, n.ssid, mark);
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    interactive(dry_run);
}

// ---------- tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_dedupes_scan() {
        let sample = "\
BSS 20:23:51:4b:1a:07(on wlp1s0)
\tsignal: -45.00 dBm
\tSSID: ZZZ
BSS aa:bb:cc:dd:ee:ff(on wlp1s0)
\tsignal: -70.00 dBm
\tSSID: Neighbor
BSS 11:22:33:44:55:66(on wlp1s0)
\tsignal: -80.00 dBm
\tSSID: ZZZ
BSS 99:88:77:66:55:44(on wlp1s0)
\tsignal: -60.00 dBm
\tSSID: ";
        let nets = parse_scan(sample);
        assert_eq!(nets.len(), 2);
        assert_eq!(nets[0].ssid, "ZZZ");
        assert_eq!(nets[0].signal, -45.0);
        assert_eq!(nets[1].ssid, "Neighbor");
    }

    #[test]
    fn quality_is_bounded() {
        assert_eq!(signal_quality(-20.0), 100);
        assert_eq!(signal_quality(-100.0), 0);
        assert!(signal_quality(-60.0) > 0 && signal_quality(-60.0) < 100);
    }

    #[test]
    fn bars_are_four_wide() {
        for dbm in [-95.0, -75.0, -55.0, -35.0] {
            assert_eq!(signal_bars(dbm).chars().count(), 4);
        }
    }

    #[test]
    fn ssid_truncation() {
        assert_eq!(truncate_ssid("short", 22), "short");
        assert_eq!(truncate_ssid("a-very-long-network-name-here", 10).chars().count(), 10);
    }

    #[test]
    fn netplan_with_password() {
        let y = generate_netplan("wlp1s0", "MyNet", "secret123");
        assert!(y.contains("renderer: networkd"));
        assert!(y.contains("\"MyNet\":"));
        assert!(y.contains("password: \"secret123\""));
        assert!(y.contains("dhcp4: true"));
    }

    #[test]
    fn netplan_open_network() {
        let y = generate_netplan("wlp1s0", "OpenNet", "");
        assert!(y.contains("\"OpenNet\": {}"));
        assert!(!y.contains("password:"));
    }

    #[test]
    fn netplan_escapes_quotes() {
        let y = generate_netplan("wlp1s0", "we\"ird", "pa\\ss\"");
        assert!(y.contains("we\\\"ird"));
        assert!(y.contains("pa\\\\ss\\\""));
    }
}
