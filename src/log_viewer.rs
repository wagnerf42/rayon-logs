extern crate rayon_logs;
use rayon_logs::{visualization_rectangles, Rectangle, TaskLog};
extern crate serde;
extern crate serde_json;
// Piston
extern crate drag_controller;
extern crate num;
extern crate piston;
extern crate piston_window;

#[macro_use]
extern crate serde_derive;

use drag_controller::{Drag, DragController};
use piston_window::*;
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::iter::once;

trait PistonRectangle {
    fn draw(
        &self,
        window: &mut PistonWindow,
        event: &Event,
        current_time: &u64,
        zoom: &f64,
        trans_x: &f64,
        trans_y: &f64,
        max_width: &f64,
        max_height: &f64,
        window_width: &u32,
        window_height: &u32,
    );
}

impl PistonRectangle for Rectangle {
    /// Draws the rectangle
    fn draw(
        &self,
        window: &mut PistonWindow,
        event: &Event,
        current_time: &u64,
        zoom: &f64,
        trans_x: &f64,
        trans_y: &f64,
        max_width: &f64,
        max_height: &f64,
        window_width: &u32,
        window_height: &u32,
    ) {
        if *current_time > self.start_time {
            let width = (self.width * ((current_time - self.start_time) as f64)
                / ((self.end_time - self.start_time) as f64))
                .min(self.width);

            window.draw_2d(event, |context, graphics| {
                ();
                rectangle(
                    self.color,
                    [self.x, self.y, width, self.height],
                    context.transform.trans(*trans_x, *trans_y).scale(
                        *zoom * (*window_width as f64) / *max_width,
                        *zoom * (*window_height as f64) / *max_height,
                    ),
                    graphics,
                );
            });
        }
    }
}

/// Draw text in the window at the position (x, y)
/// in the color described in the RGBA format
/// and with the font contained in the glyphs
pub fn draw_text(
    window: &mut PistonWindow,
    event: &Event,
    color: [f32; 4],
    x: f64,
    y: f64,
    font_size: u32,
    text: String,
    glyphs: &mut Glyphs,
    zoom: &f64,
    trans_x: &f64,
    trans_y: &f64,
) {
    window.draw_2d(event, |context, graphics| {
        let transfo = context
            .transform
            .trans(
                x + *trans_x + x * (*zoom - 1.0),
                y + *trans_y + y * (*zoom - 1.0),
            )
            .scale(*zoom, *zoom);
        ();
        let _ = text::Text::new_color(color, font_size).draw(
            &text,
            glyphs,
            &context.draw_state,
            transfo,
            graphics,
        );
    });
}

/// Set the font for the text and return a Glyph with this font
/// path_to_font: path to the .ttf file for the chosen font
pub fn set_font(window: &PistonWindow, path_to_font: String) -> Glyphs {
    let factory = window.factory.clone();
    Glyphs::new(path_to_font, factory, TextureSettings::new()).unwrap()
}

/// Clear the screen with color
pub fn clear_screen(window: &mut PistonWindow, event: &Event, color: [f32; 4]) {
    window.draw_2d(event, |_context, graphics| {
        clear(color, graphics);
        ();
    });
}

/// Create a empty window with the size width * height and the title
pub fn create_window(title: String, width: u32, height: u32) -> PistonWindow {
    let opengl = OpenGL::V3_2;
    WindowSettings::new(title, [width, height])
        .exit_on_esc(true)
        .opengl(opengl)
        .build()
        .unwrap()
}
/// /////////////////////////////////////////////////
/// JSON
///
type TaskId = usize;
type TimeStamp = u64;

/// Load a rayon_logs log file and deserializes it into a vector of logged
/// tasks information.
fn load_log_file(path: &String) -> Result<Vec<TaskLog>, io::Error> {
    let file = File::open(path).unwrap();
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;
    let v: Vec<TaskLog> = serde_json::from_str(&contents.as_str())?;
    Ok(v)
}

/// Show the commands to control the animation
fn show_commands() {
    println!("############################ COMMANDS ###########################\n");
    println!("P           : Zoom In");
    println!("M           : Zoom Out");
    println!("Space       : Pauses the animation if playing, plays it if paused");
    println!("Mouse Right : Pauses the animation if playing, plays it if paused");
    println!("R           : Restart Animation");
    println!("B           : Go back a few moments in time");
    println!("S           : Slower Animation");
    println!("F           : Faster Animation");
    println!("Mouse Left  : Move the animation");
    println!("Arrows Keys : Move the animation");
    println!("HJKL        : Move the animation (Vim Keys)");
    println!("Esc         : Quit the animation\n");
}

/// Change some of the animation parameters depending on which key was pressed
fn actions_keys(
    event: &Event,
    time: &mut u64,
    time_ratio: &mut u64,
    paused: &mut bool,
    zoom: &mut f64,
    trans_x: &mut f64,
    trans_y: &mut f64,
    ori_x: &mut f64,
    ori_y: &mut f64,
    trans_x_old: &mut f64,
    trans_y_old: &mut f64,
    drag: &mut DragController,
) {
    use piston::input::MouseButton;
    use piston_window::Button::Keyboard;
    use piston_window::Button::Mouse;
    use piston_window::Key;

    drag.event(event, |action| match action {
        Drag::Start(x, y) => {
            *ori_x = x;
            *ori_y = y;
            *trans_x_old = *trans_x;
            *trans_y_old = *trans_y;
            true
        }
        Drag::Move(x, y) => {
            *trans_x = (x - *ori_x) + *trans_x_old;
            *trans_y = (y - *ori_y) + *trans_y_old;
            true
        }
        Drag::End(x, y) => {
            *ori_x = x;
            *ori_y = y;
            false
        }
        Drag::Interrupt => true,
    });

    if let Some(button) = event.press_args() {
        match button {
            Keyboard(Key::P) => *zoom += 0.05,
            Keyboard(Key::M) => *zoom -= 0.05,
            Keyboard(Key::Space) => *paused = !*paused,
            Keyboard(Key::R) => *time = 1, // We can't say 0 because of the way the width of black rectangles is define (div by 0!)
            Keyboard(Key::B) => {
                *paused = true;
                if *time > *time_ratio {
                    *time -= *time_ratio;
                } else {
                    *time = 1;
                }
            }
            Keyboard(Key::Left) => *trans_x -= 5.0,
            Keyboard(Key::Right) => *trans_x += 5.0,
            Keyboard(Key::Up) => *trans_y -= 5.0,
            Keyboard(Key::Down) => *trans_y += 5.0,
            Keyboard(Key::F) => *time_ratio += 10,
            Keyboard(Key::S) => *time_ratio -= 10,
            Mouse(MouseButton::Right) => *paused = !*paused,
            _ => {}
        }
    };
    if let Some([_, y]) = event.mouse_scroll_args() {
        if y == 1.0 {
            *zoom += 0.05;
        } else {
            *zoom -= 0.05;
        }
    };
}

fn write_svg_file(vec_rectangle: &[Rectangle], max_height: &f64, max_width: &f64, path: String) {
    let mut file = File::create(path).unwrap();
    // Header
    file.write_all(
        b"<?xml version=\"1.0\"?>
        <svg width=\"800\" height=\"800\" version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\">\n",
    ).unwrap();
    for rec in vec_rectangle.iter() {
        file.write_fmt(format_args!(
           "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"rgba({},{},{},{})\">\n
            <animate attributeType=\"XML\" attributeName=\"width\" from=\"{}\" to=\"{}\" begin=\"{}s\" dur=\"{}s\" fill=\"freeze\"/>\n
            </rect>
            ",
            rec.x * 800.0 / max_width,                   // x
            rec.y * 800.0 / max_height,                  // y
            0,
            rec.height * 800.0 / max_height,             // height
            (rec.color[0] * 255.0) as u32,               // R
            (rec.color[1] * 255.0) as u32,               // G
            (rec.color[2] * 255.0) as u32,               // B
            rec.color[3],
            0,                                           // from
            rec.width * 800.0 / max_width,               // to
            rec.start_time / 100000,                   // begin
            (rec.end_time - rec.start_time)/ 10000  + 1,  // dur
        )).unwrap();
    }
    file.write_all(b"</svg>").unwrap();
}

fn main() {
    //let mut height_for_text = Vec::new();
    let mut current_height = 0.0;
    let mut max_width = 0.0;

    let filename = env::args().skip(1).next().expect("missing log file");
    let logs = load_log_file(&filename).expect("failed reading log file");
    let rectangles = visualization_rectangles(logs.as_slice(), 8);
    let width = rectangles
        .iter()
        .map(|r| r.width + r.x)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    let height = rectangles
        .iter()
        .map(|r| r.height + r.y)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    // Create a window
    let mut window: PistonWindow = create_window("Rayon Log Viewer".to_string(), 800, 800);
    let mut glyphs = set_font(&window, "DejaVuSans.ttf".to_string());

    show_commands();

    let mut time = 0;
    let mut zoom = 1.0;
    let mut trans_x = 0.0;
    let mut trans_y = 0.0;
    let mut trans_x_old = 0.0;
    let mut trans_y_old = 0.0;
    let mut pos_x = 0.0;
    let mut pos_y = 0.0;
    let mut drag = DragController::new();

    let mut paused = false;
    let mut time_ratio = 100; // 1 iteration = time_ratio * 1 nanosecond

    while let Some(event) = window.next() {
        let window_width = window.size().width;
        let window_height = window.size().height;
        // Actions by key: Apply modification to time or position depending on key pressed
        actions_keys(
            &event,
            &mut time,
            &mut time_ratio,
            &mut paused,
            &mut zoom,
            &mut trans_x,
            &mut trans_y,
            &mut pos_x,
            &mut pos_y,
            &mut trans_x_old,
            &mut trans_y_old,
            &mut drag,
        );
        // Clear the screen in white
        clear_screen(&mut window, &event, [1.0; 4]);
        // Display all the files names (the paths actually)
        //        for (index, file) in filenames.iter().enumerate() {
        //            draw_text(
        //                &mut window,
        //                &event,
        //                [0.0, 0.0, 0.0, 1.0], // Black
        //                0.0,
        //                height_for_text[index] * window_height as f64
        //                    / height_for_text[filenames.len() - 1],
        //                20,
        //                (file).to_string(),
        //                &mut glyphs,
        //                &zoom,
        //                &trans_x,
        //                &trans_y,
        //            );
        //        }

        // We draw all the rectangles
        for rectangle in &rectangles {
            rectangle.draw(
                &mut window,
                &event,
                &(time * time_ratio),
                &zoom,
                &trans_x,
                &trans_y,
                &width,
                &height,
                &window_width,
                &window_height,
            );
        }
        // We display the time
        draw_text(
            &mut window,
            &event,
            [0.0, 0.0, 0.0, 1.0], // Black
            0.0,
            20.0,
            20,
            format!("Time: {} ns", time * time_ratio).to_string(),
            &mut glyphs,
            &zoom,
            &trans_x,
            &trans_y,
        );

        if !paused {
            time += 1;
        }
    }
}
