extern crate serde;
extern crate serde_json;
// Piston
extern crate num;
extern crate piston;
extern crate piston_window;

#[macro_use]
extern crate serde_derive;

use piston_window::*;
// use std::cmp;
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;

/// /////////////////////////////////////////////////
/// PISTON
///

pub struct Rectangle {
    pub color: [f32; 4],
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub start_time: u64,
    pub end_time: u64,
    pub animate: bool,
}

impl Rectangle {
    /// Creates a new rectangle
    pub fn new(
        color: [f32; 4],
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        start_time: u64,
        end_time: u64,
        animate: bool,
    ) -> Rectangle {
        Rectangle {
            color: color,
            x: x,
            y: y,
            width: width,
            height: height,
            start_time: start_time,
            end_time: end_time,
            animate: animate,
        }
    }

    /// Draws the rectangle
    pub fn draw(
        &mut self,
        window: &mut PistonWindow,
        event: &Event,
        current_time: &u64,
        zoom: &f64,
        trans_x: &f64,
        trans_y: &f64,
    ) {
        if self.animate {
            if *current_time > self.start_time {
                let width = (self.width * ((current_time - self.start_time) as f64)
                    / ((self.end_time - self.start_time) as f64))
                    .min(self.width);

                window.draw_2d(event, |context, graphics| {
                    ();
                    rectangle(
                        self.color,
                        [self.x, self.y, width, self.height],
                        context.transform.zoom(*zoom).trans(*trans_x, *trans_y),
                        graphics,
                    );
                });
            }
        } else {
            window.draw_2d(event, |context, graphics| {
                ();
                rectangle(
                    self.color,
                    [self.x, self.y, self.width, self.height],
                    context.transform.zoom(*zoom).trans(*trans_x, *trans_y),
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
                x + *trans_x * (*zoom) - (x * (1.0 - *zoom)), // Little transformation so the text doesn't
                y + *trans_y * (*zoom) - (y * (1.0 - *zoom)), // get lost when zooming and moving around
            )
            .zoom(*zoom);
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

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskLog {
    start_time: TimeStamp,
    end_time: TimeStamp,
    thread_id: usize,
    children: Vec<TaskId>,
}

/// Returns the vector with the TaskLog from the json file
fn json_into_vec(path: &String) -> Result<Vec<TaskLog>, io::Error> {
    let file = File::open(path).unwrap();
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;
    let v: Vec<TaskLog> = serde_json::from_str(&contents.as_str())?;
    Ok(v)
}

/// //////////////////////////////////////////////////
/// TaskVisu
///

#[derive(Debug)]
/// A structure similar to TaskLog
/// but also with the time that the animation of the task should be in a certain color
pub struct TaskVisu {
    /// Same fields as in the TaskLog structure
    start_time: TimeStamp,
    end_time: TimeStamp,
    thread_id: usize,
    children: Vec<TaskId>,
    /// activity_periods : [(duration starting the task, color of the thread),
    /// (duration doing sub-tasks, black (=0)),
    /// (duration ending the task, color of the thread)]
    //activity_periods: [u64; 2],
    /// Data needed for the drawing part
    pos_x: f64,
    pos_y: f64,
    width: f64,
    gap: f64,
}

impl TaskVisu {
    /// Takes a TaskLog object and copies its data into a TaskVisu structure
    /// Does not initialize time_colors (see the get_times method)
    pub fn new(task: &TaskLog, _tasks: &[TaskLog]) -> Self {
        TaskVisu {
            start_time: task.start_time,
            end_time: task.end_time,
            thread_id: task.thread_id,
            children: {
                let children = task.children.iter().map(|child| *child).collect();
                children
            },
            //       activity_periods: match task.children {
            //           None => [task.end_time; 2],
            //           Some((l, r)) => [
            //               cmp::min(tasks[l].start_time, tasks[r].start_time),
            //               cmp::max(tasks[l].end_time, tasks[r].end_time),
            //           ],
            //       },
            pos_x: 0.0,
            pos_y: 0.0,
            width: (task.end_time - task.start_time) as f64,
            gap: 0.0,
        }
    }

    /// Returns a rectangle with the data from the task
    /// The color is chosent with the thread_id
    pub fn to_rectangle(&self) -> Rectangle {
        let color = match self.thread_id % 8 {
            0 => [1.0, 0.0, 0.0, 1.0],
            1 => [0.0, 1.0, 0.0, 1.0],
            2 => [0.0, 0.0, 1.0, 1.0],
            3 => [1.0, 1.0, 0.0, 1.0],
            4 => [1.0, 0.0, 1.0, 1.0],
            5 => [0.0, 1.0, 1.0, 1.0],
            6 => [0.5, 0.5, 0.5, 1.0],
            _ => [1.0, 0.5, 0.5, 1.0],
        };
        Rectangle::new(
            color,
            self.pos_x / 2000.0,
            self.pos_y * 2.0,
            self.width / 2000.0,
            20.0,
            self.start_time,
            self.end_time,
            true,
        )
    }

    ///  Returns a black rectangle with the data from the task
    pub fn to_black_rectangle(&self) -> Rectangle {
        Rectangle::new(
            [0.0, 0.0, 0.0, 1.0],
            self.pos_x / 2000.0,
            self.pos_y * 2.0,
            self.width / 2000.0,
            20.0,
            0,
            self.end_time,
            false,
        )
    }
}

/// Gets the width of the Tree
/// and compute the size of the gaps between the nodes
pub fn get_dimensions(
    index: &usize,
    mut tasks: &mut [TaskVisu],
    mut offset: &mut f64,
    start: &u64,
) -> (f64, f64) {
    match tasks[*index].children.len() {
        0 => {
            tasks[*index].start_time -= *start;
            tasks[*index].end_time -= *start;
            (*offset + tasks[*index].width, 0.0)
        }
        n => {
            tasks[*index].start_time -= *start;
            tasks[*index].end_time -= *start;

            let mut sub_height = 0.0;
            let mut sub_width = 0.0;
            let mut max_sub_width = 0.0;
            let children: Vec<usize> = tasks[*index].children.iter().map(|child| *child).collect();
            for child in children.iter() {
                let (child_width, child_height) =
                    get_dimensions(&child, &mut tasks, &mut offset, &start);
                *offset += child_width;
                sub_width += child_width;
                if child_width > max_sub_width {
                    max_sub_width = child_width;
                }
                if sub_height < child_height {
                    sub_height = child_height;
                }
            }
            if sub_width <= tasks[*index].width {
                // It is not possible for a node to have a single child
                // ie: a Task can not have a single sub-task after a join
                assert!(n != 1);
                tasks[*index].gap = (tasks[*index].width - sub_width) / (n as f64 - 1.0);
                return (tasks[*index].width, sub_height + 1.0);
            } else {
                tasks[*index].gap = 20000.0; // just for style purposes
                return (sub_width, sub_height + 1.0);
            }
        }
    }
}

/// Sets the positions of each node of the tree
/// We want something like this:
///
/// +-------------+
/// |    TASK 1   |
/// +-------------+
///
/// +---+  +------+
/// | 2 |  |   3  |
/// +---+  +------+
///
///        +--+ +-+
///        |4 | |5|
///        +--+ +-+
///
pub fn set_positions(index: &usize, mut tasks: &mut [TaskVisu], mut offset: &mut f64) -> f64 {
    match tasks[*index].children.len() {
        0 => {
            // If the node is a leaf, then its position (in x) is the offset
            tasks[*index].pos_x = *offset;
            // We return the new offset which is increased by the leaf width
            *offset + tasks[*index].width
        }
        n => {
            let children: Vec<usize> = tasks[*index].children.iter().map(|child| *child).collect();
            for child in children.iter() {
                // For each child
                // We set the y coordinate of the node
                tasks[*child].pos_y = tasks[*index].pos_y + 20.0;
                // And we call recursivly the function to set the positions
                // within the subtree
                *offset = set_positions(&child, &mut tasks, &mut offset) + tasks[*index].gap;
            }
            // We then set the position of the current node by placing it
            // in the middle of its children
            tasks[*index].pos_x = (tasks[children[0]].pos_x
                + tasks[children[n - 1]].pos_x
                + tasks[children[n - 1]].width) / 2.0
                - (tasks[*index].width / 2.0);
            // And we return the new offset
            *offset - tasks[*index].gap
        }
    }
}

/// Takes the vector of all the TaskLog and returns a vector of TaskVisu ready for the animation
fn create_taskvisu(tasks: &[TaskLog], begin_height: &f64) -> (Vec<TaskVisu>, f64) {
    let mut tasks_visu: Vec<TaskVisu> = tasks.iter().map(|t| TaskVisu::new(t, tasks)).collect();
    let (_, height) = get_dimensions(&mut 0, &mut tasks_visu, &mut 0.0, &tasks[0].start_time);
    tasks_visu[0].pos_y = begin_height * 20.0;
    let _ = set_positions(&0, &mut tasks_visu, &mut 0.0);
    (tasks_visu, height)
}

fn show_commands() {
    println!("############################ COMMANDS ###########################\n");
    println!("P           : Zoom In");
    println!("M           : Zoom Out");
    println!("Space       : Pauses the animation if playing, plays it if paused");
    println!("R           : Restart Animation");
    println!("B           : Go back a few moments in time");
    println!("Arrows Keys : Move the animation");
    println!("HJKL        : Move the animation (Vim Keys)\n");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("\nWrong argument(s)\nUsage: ./log_viewer path_to_log1.json path_to_log2.json");
    }
    let mut tasks_for_visu = Vec::new();
    let mut height_for_text = Vec::new();
    let mut current_height = 0.0;
    // For all the files passed in the command line
    for index in 1..args.len() {
        let (tasks_file, height) =
            create_taskvisu(&json_into_vec(&args[index]).unwrap(), &current_height);
        current_height += height + 2.0;
        tasks_for_visu.push(tasks_file);
        height_for_text.push(current_height);
    }
    // Create a window
    let mut window: PistonWindow = create_window("Rayon Log Viewer".to_string(), 800, 800);
    let mut glyphs = set_font(
        &window,
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_string(), // May not work if there is no such file ..
    );

    show_commands();
    let mut vec_rectangle: Vec<Rectangle> = Vec::new();
    let mut vec_black: Vec<Rectangle> = Vec::new();
    for file_tasks in tasks_for_visu.iter() {
        for task in file_tasks.iter() {
            let mut rec = task.to_rectangle();
            let mut rec_b = task.to_black_rectangle();

            vec_rectangle.push(rec);
            vec_black.push(rec_b);
        }
    }
    let mut time = 0;
    let mut zoom = 1.0;
    let mut trans_x = 0.0;
    let mut trans_y = 0.0;

    let mut paused = false;
    let time_ratio = 50; // 1 iteration = time_ratio * 1 nanosecond
    while let Some(event) = window.next() {
        // Actions by key
        if let Some(button) = event.press_args() {
            use piston_window::Button::Keyboard;
            use piston_window::Key;

            if button == Keyboard(Key::P) {
                zoom += 0.05; // Zoom In
            }
            if button == Keyboard(Key::M) {
                zoom -= 0.05; // Zoom Out
            }
            if button == Keyboard(Key::Space) {
                paused = !paused; // Pause if playing, play if paused
            }
            if button == Keyboard(Key::R) {
                time = 0; // Restart
            }
            if button == Keyboard(Key::B) {
                paused = true;
                if time > 5 * time_ratio {
                    time -= 5 * time_ratio;
                } else {
                    time = 0;
                }
            }
            // Keys for moving the rectangles (also works with Vim keys)
            if button == Keyboard(Key::Left) || button == Keyboard(Key::H) {
                trans_x -= 5.0;
            }
            if button == Keyboard(Key::Right) || button == Keyboard(Key::L) {
                trans_x += 5.0;
            }
            if button == Keyboard(Key::Up) || button == Keyboard(Key::K) {
                trans_y -= 5.0;
            }
            if button == Keyboard(Key::Down) || button == Keyboard(Key::J) {
                trans_y += 5.0;
            };
        }
        // Clear the screen in white
        clear_screen(&mut window, &event, [1.0; 4]);
        for index in 1..args.len() {
            draw_text(
                &mut window,
                &event,
                [0.0, 0.0, 0.0, 1.0], // Black
                0.0,
                (height_for_text[index - 1] - 1.0) * 40.0,
                20,
                (*args[index]).to_string(),
                &mut glyphs,
                &zoom,
                &trans_x,
                &trans_y,
            );
        }

        // We draw all the rectangles
        for rectangle in vec_black.iter_mut().chain(vec_rectangle.iter_mut()) {
            rectangle.draw(
                &mut window,
                &event,
                &(time * time_ratio),
                &zoom,
                &trans_x,
                &trans_y,
            );
        }
        if !paused {
            time += 1;
        }
    }
}
