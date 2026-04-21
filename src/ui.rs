use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::{App, Cell, CurrentScreen};

/// Big block ASCII/unicode title banner
const TITLE: &str = concat!(
    "╔═╗╦ ╦╔═╗╔╦╗\n",
    "║  ╠═╣╠═╣ ║ \n",
    "╚═╝╩ ╩╩ ╩ ╩ ",
);

pub fn ui(frame: &mut Frame, app: &App) {
    match app.current_screen {
        CurrentScreen::Chat => render_chat(frame, app),
        CurrentScreen::Battleship => render_battleship(frame, app),
    }
}

fn render_chat(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let [title_area, messages_area, input_area, room_selector_area] = Layout::vertical([
        Constraint::Length(5),
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(3),
    ])
    .areas(area);

    // Title banner
    frame.render_widget(
        Paragraph::new(TITLE).block(
            Block::bordered()
                .border_style(Style::default().fg(Color::Cyan)),
        ),
        title_area,
    );

    // Message history
    frame.render_widget(
        Block::bordered()
            .title(" messages ")
            .border_style(Style::default().fg(Color::DarkGray)),
        messages_area,
    );

    // Input box
    frame.render_widget(
        Block::bordered()
            .title(" type a message ")
            .border_style(Style::default().fg(Color::Yellow)),
        input_area,
    );

    // Room selector
    render_room_selector(frame, app, room_selector_area);
}

fn render_room_selector(frame: &mut Frame, app: &App, area: Rect) {
    let rooms = ["#general", "#battleship"];
    let constraints: Vec<Constraint> = rooms
        .iter()
        .map(|_| Constraint::Fill(1))
        .collect();

    let cells = Layout::horizontal(constraints).split(area);

    for (i, (room, cell)) in rooms.iter().zip(cells.iter()).enumerate() {
        let is_active = i == app.selected_room;
        let style = if is_active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        frame.render_widget(
            Paragraph::new(*room)
                .style(style)
                .block(Block::bordered()),
            *cell,
        );
    }
}

fn render_battleship(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let [grids_area, log_area, controls_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(10),
        Constraint::Length(6),
    ])
    .areas(area);

    let [your_stats_area, your_board_area, their_board_area, their_stats_area] =
        Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .areas(grids_area);

    // Your stats
    frame.render_widget(
        your_stats_widget(app)
            .block(Block::bordered()
                .title(" Your Stats ")
                .border_style(Style::default().fg(Color::Green))),
        your_stats_area,
    );

    // Your board
    let your_title = if app.your_turn {
        Line::from(vec![
            Span::styled("[TURN] ", Style::default().fg(Color::Yellow)),
            Span::raw("Your Board"),
        ])
    } else {
        Line::from(" Your Board ")
    };

    frame.render_widget(
        render_board(&app.your_board)
            .block(Block::bordered()
                .title(your_title)
                .border_style(Style::default().fg(Color::Green))),
        your_board_area,
    );

    // Opponent board
    let opp_title = if !app.your_turn {
        Line::from(vec![
            Span::styled("[TURN] ", Style::default().fg(Color::Yellow)),
            Span::raw("Opponent Board"),
        ])
    } else {
        Line::from(" Opponent Board ")
    };

    frame.render_widget(
        render_board(&app.their_board)
            .block(Block::bordered()
                .title(opp_title)
                .border_style(Style::default().fg(Color::Red))),
        their_board_area,
    );

    // Their stats
    frame.render_widget(
        their_stats_widget(app)
            .block(Block::bordered()
                .title(" Opp. Stats ")
                .border_style(Style::default().fg(Color::Red))),
        their_stats_area,
    );

    // Game log
    frame.render_widget(
        render_game_log(app)
            .block(Block::bordered()
                .title(" Game Log ")
                .border_style(Style::default().fg(Color::DarkGray))),
        log_area,
    );

    // Controls
    frame.render_widget(
        Paragraph::new(vec![
            Line::from("T: Toggle message input"),
            Line::from("Q: Quit"),
            Line::from("Arrow Keys: Change target position"),
            Line::from("Enter: Submit/Fire"),
        ])
        .block(Block::bordered()
            .title(" Controls ")
            .border_style(Style::default().fg(Color::DarkGray))),
        controls_area,
    );
}

fn your_stats_widget(app: &App) -> Paragraph<'static> {
    Paragraph::new(vec![
        Line::from(Span::styled("Pieces", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("Carrier:     ", Style::default().fg(Color::DarkGray)),
            Span::styled("5/5", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Battleship:  ", Style::default().fg(Color::DarkGray)),
            Span::styled("4/4", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Destroyer:   ", Style::default().fg(Color::DarkGray)),
            Span::styled("3/3", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Submarine:   ", Style::default().fg(Color::DarkGray)),
            Span::styled("3/3", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Patrol Boat: ", Style::default().fg(Color::DarkGray)),
            Span::styled("2/2", Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Total", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("HP: ", Style::default().fg(Color::DarkGray)),
            Span::styled("17/17", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Shots: ", Style::default().fg(Color::DarkGray)),
            Span::styled("0", Style::default().fg(Color::Blue)),
        ]),
        Line::from(vec![
            Span::styled("Hits: ", Style::default().fg(Color::DarkGray)),
            Span::styled("0", Style::default().fg(Color::Yellow)),
        ]),
    ])
}

fn their_stats_widget(app: &App) -> Paragraph<'static> {
    Paragraph::new(vec![
        Line::from(Span::styled("Pieces", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("???", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled("???", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled("???", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled("???", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled("???", Style::default().fg(Color::DarkGray))),
        Line::from(""),
        Line::from(Span::styled("Total", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("HP: ", Style::default().fg(Color::DarkGray)),
            Span::styled("??/17", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("Shots: ", Style::default().fg(Color::DarkGray)),
            Span::styled("0", Style::default().fg(Color::Blue)),
        ]),
        Line::from(vec![
            Span::styled("Hits: ", Style::default().fg(Color::DarkGray)),
            Span::styled("0", Style::default().fg(Color::Yellow)),
        ]),
    ])
}

fn render_game_log(app: &App) -> Paragraph<'static> {
    Paragraph::new(vec![
        Line::from(vec![
            Span::styled("9:41", Style::default().fg(Color::DarkGray)),
            Span::styled(" [Game]: ", Style::default().fg(Color::Yellow)),
            Span::raw("Welcome to Battleship! Welcome to Battleship! Welcome to Battleship! Welcome to Battleship! Welcome to Battleship! Welcome to Battleship!"),
        ]),

        Line::from(vec![
            Span::styled("9:42", Style::default().fg(Color::DarkGray)),
            Span::styled(" <You>: ", Style::default().fg(Color::Green)),
            Span::raw("Why hello there"),
        ]),

        Line::from(vec![
            Span::styled("9:42", Style::default().fg(Color::DarkGray)),
            Span::styled(" <Dark Passenger>: ", Style::default().fg(Color::Red)),
            Span::raw("Whats up"),
        ]),
    ])
}

fn render_board(board: &[[Cell; 10]; 10]) -> Paragraph<'static> {
    let mut lines: Vec<Line> = Vec::new();

    // Header row: "   A B C D E F G H I J"
    let header = Line::from(
Span::styled("   A B C D E F G H I J", Style::default().fg(Color::DarkGray)),
);
    lines.push(header);

    // Separator
    let sep = Line::from(Span::styled("  +-+-+-+-+-+-+-+-+-+-+", Style::default().fg(Color::DarkGray)));

    lines.push(sep.clone());

    for row in 0..10 {
        // Row number — use 0 for the 10th row to keep single digit
        let row_label = if row == 9 { "0".to_string() } else { (row + 1).to_string() };

        let mut spans = vec![
            Span::styled(format!("{} ", row_label), Style::default().fg(Color::DarkGray)),
            Span::styled("|", Style::default().fg(Color::DarkGray)),
        ];

        for col in 0..10 {
            let (symbol, fg) = match board[row][col] {
                Cell::Empty      => (" ", Color::Reset),
                Cell::Hit        => ("x", Color::Yellow),
                Cell::Miss       => ("~", Color::Blue),
                Cell::Sunk       => ("x", Color::Red),
                Cell::Carrier    => ("C", Color::Green),
                Cell::Battleship => ("B", Color::Green),
                Cell::Destroyer  => ("D", Color::Green),
                Cell::Submarine  => ("S", Color::Green),
                Cell::Patrol     => ("P", Color::Green),
            };

            spans.push(Span::styled(symbol, Style::default().fg(fg)));
            spans.push(Span::styled("|", Style::default().fg(Color::DarkGray)));
        }

        lines.push(Line::from(spans));
        lines.push(sep.clone());
    }

    Paragraph::new(lines)
}
