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

const COLORS: [[f32; 4]; 8] = [
    [1.0, 0.0, 0.0, 1.0],
    [0.0, 1.0, 0.0, 1.0],
    [0.0, 0.0, 1.0, 1.0],
    [1.0, 1.0, 0.0, 1.0],
    [1.0, 0.0, 1.0, 1.0],
    [0.0, 1.0, 1.0, 1.0],
    [0.5, 0.5, 0.5, 1.0],
    [1.0, 0.5, 0.5, 1.0],
];

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
    ) -> Rectangle {
        Rectangle {
            color: color,
            x: x,
            y: y,
            width: width,
            height: height,
            start_time: start_time,
            end_time: end_time,
        }
    }

    /// Draws the rectangle
    pub fn draw(
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
    event_times: Vec<u64>,
    thread_id: usize,
    children: Vec<TaskId>,
    /// Data needed for the drawing part
    width: f64,
}

impl TaskVisu {
    /// Takes a TaskLog object and copies its data into a TaskVisu structure
    /// Does not initialize time_colors (see the get_times method)
    pub fn new(task: &TaskLog, _tasks: &[TaskLog]) -> Self {
        TaskVisu {
            event_times: Vec::with_capacity(2),
            thread_id: task.thread_id,
            children: {
                let children = task.children.iter().map(|child| *child).collect();
                children
            },
            width: (task.end_time - task.start_time) as f64,
        }
    }

    /// Return start time.
    pub fn start_time(&self) -> u64 {
        *self.event_times.first().unwrap()
    }

    /// Return end time.
    pub fn end_time(&self) -> u64 {
        *self.event_times.last().unwrap()
    }

    /// Generate all the rectangle for a given task
    /// if the thread is actually in the task, then it will display color
    /// else, that part of the rectangle will stay black
    fn generate_rectangles(&self, x: f64, y: f64, rectangles: &mut Vec<Rectangle>) {
        rectangles.push(Rectangle::new(
            [0.0, 0.0, 0.0, 1.0],
            x,
            y,
            self.width,
            1.0,
            0,
            1,
        ));
        for times in self.event_times.chunks(2) {
            let start = times[0];
            let end = times[1];
            rectangles.push(Rectangle::new(
                COLORS[self.thread_id % COLORS.len()],
                x + (start - self.start_time()) as f64,
                y,
                (end - start) as f64,
                1.0,
                start,
                end,
            ));
        }
    }
}

/// compute widths of all subtrees (and total height)
pub fn compute_dimensions(
    index: usize,
    tasks: &[TaskVisu],
    subtree_widths: &mut [f64],
) -> (f64, f64) {
    match tasks[index].children.len() {
        0 => (tasks[index].width, 1.0),
        _ => {
            // compute sizes of subtrees
            // we are interested in max of heights and sum of widths
            let (mut sub_width, mut sub_height) = tasks[index]
                .children
                .iter()
                .map(|&c| compute_dimensions(c, tasks, subtree_widths))
                .fold((0.0, 0.0_f64), |acc, (w, h)| (acc.0 + w, acc.1.max(h)));

            // now, final subtree width is max of our own and children's sum width
            subtree_widths[index] = sub_width.max(tasks[index].width);
            // height increases by one level
            sub_height += 1.0;

            (subtree_widths[index], sub_height)
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
pub fn create_rectangles(
    index: usize,
    tasks: &[TaskVisu],
    subtree_widths: &[f64],
    offset: f64,
    y: f64,
    rectangles: &mut Vec<Rectangle>,
) {
    let mut x = offset;
    let gap: f64 = (subtree_widths[index]
        - tasks[index]
            .children
            .iter()
            .map(|c| subtree_widths[*c])
            .sum::<f64>()) / (tasks[index].children.len() + 1) as f64;

    for child in &tasks[index].children {
        x += gap;
        create_rectangles(*child, tasks, subtree_widths, x, y + 1.2, rectangles);

        let pos_x = x + (subtree_widths[*child] - tasks[*child].width) / 2.0;
        let pos_y = y + 1.2;
        tasks[*child].generate_rectangles(pos_x, pos_y, rectangles);
        x += subtree_widths[*child];
    }
}

/// For each task we figure out in which time periods its thread is actively working on it (or idle).
/// This information is stored in TaskVisu's event_times field as time stamps for each activity
/// change.
fn compute_activity_periods(tasks_visu: &mut [TaskVisu], tasks: &[TaskLog]) {
    for thread in (0..8) {
        // we look at what each thread is doing
        let mut events: Vec<(TaskId, u64, bool)> = tasks
            .iter()
            .enumerate()
            .filter(|&(_, t)| t.thread_id == thread)
            .flat_map(|(id, t)| once((id, t.start_time, true)).chain(once((id, t.end_time, false))))
            .collect();

        // through time
        events.sort_unstable_by_key(|e| e.1);

        // now replay thread activity
        // we will need to remember what tasks are started
        let mut active_tasks: Vec<TaskId> = Vec::new();

        for event in &events {
            // add event to currently active task
            if let Some(task_id) = active_tasks.last() {
                tasks_visu[*task_id].event_times.push(event.1);
            }
            // change active task
            if event.2 {
                active_tasks.push(event.0);
            } else {
                active_tasks.pop();
            }
            // add event to new active task
            if let Some(task_id) = active_tasks.last() {
                tasks_visu[*task_id].event_times.push(event.1);
            }
        }
    }
}

/// Takes the vector of all the TaskLog and returns a vector of TaskVisu ready for the animation
fn create_taskvisu(
    tasks: &[TaskLog],
    begin_height: f64,
    rectangles: &mut Vec<Rectangle>,
    max_width: &mut f64,
) -> f64 {
    let mut tasks_visu: Vec<TaskVisu> = tasks.iter().map(|t| TaskVisu::new(t, tasks)).collect();
    compute_activity_periods(&mut tasks_visu, &tasks);
    let mut subtree_widths: Vec<f64> = tasks_visu.iter().map(|t| t.width).collect();
    let (width, height) = compute_dimensions(0, &tasks_visu, &mut subtree_widths);
    if width > *max_width {
        *max_width = width;
    }
    let x = (subtree_widths[0] - tasks_visu[0].width) / 2.0;
    let y = begin_height + 2.0;
    tasks_visu[0].generate_rectangles(x, y, rectangles);
    create_rectangles(0, &tasks_visu, &subtree_widths, 0.0, y, rectangles);
    y + height + 2.0
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

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("\nWrong argument(s)\nUsage: ./log_viewer path_to_log1.json path_to_log2.json");
    }
    let mut height_for_text = Vec::new();
    let mut current_height = 0.0;
    let mut max_width = 0.0;
    let mut vec_rectangle: Vec<Rectangle> = Vec::new();
    // For all the files passed in the command line
    for index in 1..args.len() {
        current_height = create_taskvisu(
            &json_into_vec(&args[index]).unwrap(),
            current_height,
            &mut vec_rectangle,
            &mut max_width,
        );
        height_for_text.push(current_height);
    }
    let max_height = current_height;
    // Create a window
    let mut window: PistonWindow = create_window("Rayon Log Viewer".to_string(), 800, 800);
    let mut glyphs = set_font(&window, "DejaVuSans.ttf".to_string());

    /*
    let mut file = File::create("out.svg").unwrap();
    // Header
    file.write_all(
        b"<?xml version=\"1.0\"?>
        <svg width=\"800\" height=\"800\" version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\">\n",
    ).unwrap();
    for rec in vec_rectangle.iter() {
        file.write_fmt(format_args!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"rgb({}, {}, {})\">\n
    \t<animate attributeType=\"XML\" attributeName=\"width\" from=\"{}\" to=\"{}\"
        dur=\"{}s\" begin=\"{}s\"/>\n
  </rect>\n",
            rec.x * 800.0 / max_width,
            rec.y * 800.0 / max_height,
            rec.width * 800.0 / max_width,
            rec.height * 800.0 / max_height,
            (rec.color[0] * 255.0) as u32,
            (rec.color[1] * 255.0) as u32,
            (rec.color[2] * 255.0) as u32,
            0,
            rec.width,
            (rec.end_time - rec.start_time) / 100,
            rec.start_time / 100
        )).unwrap();
    }
    file.write_all(b"</svg>").unwrap();
    */

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
        for index in 1..args.len() {
            draw_text(
                &mut window,
                &event,
                [0.0, 0.0, 0.0, 1.0], // Black
                0.0,
                height_for_text[index - 1] * window_height as f64 / height_for_text[args.len() - 2],
                20,
                (*args[index]).to_string(),
                &mut glyphs,
                &zoom,
                &trans_x,
                &trans_y,
            );
        }

        // We draw all the rectangles
        for rectangle in vec_rectangle.iter() {
            rectangle.draw(
                &mut window,
                &event,
                &(time * time_ratio),
                &zoom,
                &trans_x,
                &trans_y,
                &max_width,
                &height_for_text[args.len() - 2],
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
