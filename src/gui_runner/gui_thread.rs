use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use super::{PartialWorld, RunMode};
use gui::GUI;

pub mod gui;

// GuiThread handles spawning a thread which will run the GUI

pub struct GuiThread {
    worker_to_gui_rx: Receiver<PartialWorld>,
    gui_to_game_tx: Sender<RunMode>,
}
impl GuiThread {
    pub fn new(worker_to_gui_rx: Receiver<PartialWorld>, gui_to_game_tx: Sender<RunMode>) -> Self {
        Self { worker_to_gui_rx, gui_to_game_tx }
    }
    pub fn start(self) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            // GUI is not Send :(
            let gui = GUI::new("Ragnarok", self.worker_to_gui_rx, self.gui_to_game_tx);
            gui.run();
        })
    }
}
