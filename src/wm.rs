use crate::config::{KEYBINDS, MODKEY};
use crate::cursor::{CoreCursor, CursorIndex};
use crate::desktop::Desktop;
use crate::event::{Event, MouseButton};
use crate::helper;
use crate::screen::Screen;
use crate::x::{XConn, XWindow};

#[derive(PartialEq)]
enum MouseMode {
    Ground,
    Resize,
    Move,
}

pub struct WM<'a> {
    // X connection
    pub conn: XConn<'a>,

    // Windows in workspaces stored
    pub desktop: Desktop,
    pub screen:  Screen,

    // Mouse mode from button press events
    mouse_mode: MouseMode,
    last_mouse_x: i32,
    last_mouse_y: i32,

    // Should we continue running?
    running: bool,
}

impl<'a> WM<'a> {
    pub fn register(conn: &'a xcb::Connection, screen_idx: i32) -> Self {
        // Create new XConn wrapping xcb::Connection
        let mut xconn = XConn::new(conn);

        // Get root window id for screen index
        let root_id = xconn.get_setup().roots().nth(screen_idx as usize).expect("Getting root window id for screen index").root();

        // Create new screen object
        let mut screen = Screen::new(screen_idx, root_id);

        // Try register the root window for necessary window management events
        xconn.change_window_attributes_checked(root_id, &helper::values_attributes_root());

        // For configured keybinds, register X to grab keys on the root window
        for (mask, keysym, _) in KEYBINDS {
            xconn.grab_key(root_id, *mask, *keysym, true);
        }

        // Register root window to grab necessary mouse button events
        xconn.grab_button(root_id, helper::ROOT_BUTTON_GRAB_MASK, xcb::BUTTON_INDEX_1, MODKEY, true);
        xconn.grab_button(root_id, helper::ROOT_BUTTON_GRAB_MASK, xcb::BUTTON_INDEX_3, MODKEY, true);

        // Create necessary core cursors
        xconn.create_core_cursor(CursorIndex::LeftPointer, CoreCursor::LeftPtr);

        // Now set the default starting cursor
        xconn.set_cursor(root_id, CursorIndex::LeftPointer);

        // Perform initial screen geometry fetch
        xconn.update_geometry(&mut screen);

        // Return new WM object
        return Self {
            conn: xconn,
            desktop: Desktop::default(),
            screen:  screen,
            mouse_mode: MouseMode::Ground,
            last_mouse_x: 0,
            last_mouse_y: 0,
            running: true,
        };
    }

    pub fn run(&mut self) {
        outlog::info!("Started running");

        while self.running {
            // Get next event
            let event = self.conn.next_event();

            // Handle event
            match event {
                Event::MapRequest(window_id) => {
                    if let Some((ws, _)) = self.desktop.contains_mut(window_id) {
                        // We already have this window, if in the current then focus!
                        if ws.is_active() {
                            ws.window_focus(&self.conn, &self.screen, window_id);
                        }
                    } else {
                        // Add to current workspace
                        self.desktop.current_mut().window_add(&self.conn, &self.screen, window_id);
                    }
                },

                Event::UnmapNotify(window_id) => {
                    // Remove window (if there!)
                    if let Some((ws, idx)) = self.desktop.contains_mut(window_id) {
                        ws.window_del(&self.conn, &self.screen, idx, window_id);
                    }
                },

                Event::DestroyNotify(window_id) => {
                    // Remove window (if there!)
                    if let Some((ws, idx)) = self.desktop.contains_mut(window_id) {
                        ws.window_del(&self.conn, &self.screen, idx, window_id);
                    }
                },

                Event::EnterNotify(window_id) => {
                    // Focus input to this window
                    self.conn.set_input_focus(window_id);
                },

                Event::MotionNotify => {
                    // If no tracked windows, nothing to do
                    if self.desktop.current().windows.is_empty() {
                        continue;
                    }

                    // Get current pointer location
                    let (px, py, _) = self.conn.query_pointer(self.screen.id());

                    // Calculate dx, dy
                    let dx = (px - self.last_mouse_x) as i32;
                    let dy = (py - self.last_mouse_y) as i32;

                    // Set new last mouse positions
                    self.last_mouse_x = px;
                    self.last_mouse_y = py;

                    // React depending on current MouseMode
                    match self.mouse_mode {
                        MouseMode::Move => {
                            // Move currently focused window
                            self.desktop.current_mut().windows.focused_mut().unwrap().do_move(&self.conn, &self.screen, dx, dy);
                        },

                        MouseMode::Resize => {
                            // Resize currently focused window
                            self.desktop.current_mut().windows.focused_mut().unwrap().do_resize(&self.conn, &self.screen, dx, dy);
                        },

                        MouseMode::Ground => panic!("MouseMode::Ground state registered in MotionNotify"),
                    }
                },

                Event::KeyPress((key_ev, window_id)) => {
                    // Try get function for keybind
                    for (mask, key, keyfn) in KEYBINDS {
                        if *mask == key_ev.mask &&
                           *key == key_ev.key {
                            // If window id isn't the focused window id, refocus
                            if !self.desktop.current_mut().windows.is_focused(window_id) {
                                self.desktop.current_mut().window_focus(&self.conn, &self.screen, window_id);
                            }

                            // Execute! And return
                            keyfn(self);
                            break;
                        }
                    }
                },

                Event::ButtonPress((but, window_id)) => {
                    // If no windows, nothing to do
                    if self.desktop.current().windows.is_empty() {
                        continue;
                    }

                    // Grab pointer input
                    self.conn.grab_pointer(self.screen.id(), helper::ROOT_POINTER_GRAB_MASK, false);

                    // Get current pointer position
                    let (px, py, _) = self.conn.query_pointer(self.screen.id());
                    self.last_mouse_x = px;
                    self.last_mouse_y = py;

                    // If window id different to focused, focus this one
                    if window_id != self.desktop.current().windows.focused().unwrap().id() {
                        self.desktop.current_mut().window_focus(&self.conn, &self.screen, window_id);
                    }

                    // Handle button press
                    match but {
                        MouseButton::LeftClick => {
                            // Enter move mode
                            self.mouse_mode = MouseMode::Move;
                        },

                        MouseButton::RightClick => {
                            // Enter resize mode
                            self.mouse_mode = MouseMode::Resize;
                        },
                    }
                },

                Event::ButtonRelease(_) => {
                    // Ungrab pointer input
                    self.conn.ungrab_pointer();

                    // Regardless of button, current state etc, we unset the mouse mode
                    self.mouse_mode = MouseMode::Ground;
                },
            }
        }

        outlog::info!("Finished running");
    }

    pub fn kill(&mut self) {
        outlog::info!("Killing");
        self.running = false;
    }
}
