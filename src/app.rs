use std::collections::HashMap;

pub enum CurrentScreen {
    Chat,
    Battleship,
}

#[derive(Clone, Copy)]
pub enum Cell {
    Empty,
    Hit,
    Miss,
    Sunk,
    Carrier,
    Battleship,
    Destroyer,
    Submarine,
    Patrol,
}

pub struct App {
    pub current_screen: CurrentScreen, // the current screen the user is looking at, and will later determine what is rendered.
    pub selected_room: usize,
    pub your_board: [[Cell; 10]; 10],
    pub their_board: [[Cell; 10]; 10],
    pub your_turn: bool,
}

impl App {
    pub fn new() -> App {
        App {
            current_screen: CurrentScreen::Battleship,
            selected_room: 0,
            your_board: [[Cell::Hit; 10]; 10],
            their_board: [[Cell::Carrier; 10]; 10],
            your_turn: true,
        }
    }
}