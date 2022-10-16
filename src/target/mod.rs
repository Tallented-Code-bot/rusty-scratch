#![allow(
    unused_imports,
    unused_mut,
    unused_variables,
    dead_code,
    non_snake_case
)]

extern crate rand;
use genawaiter::rc::{gen, Co};
use genawaiter::{yield_, GeneratorState};
use rand::Rng;
use std::pin::Pin;
use std::{
    boxed::Box,
    collections::HashMap,
    f32::consts::PI,
    future::{Future, IntoFuture},
};

#[derive(Clone)]
pub enum RotationStyle {
    AllAround,
    LeftRight,
    DontRotate,
}

impl RotationStyle {
    pub fn from_str(input: &str) -> Result<RotationStyle, ()> {
        match input {
            "all around" => Ok(RotationStyle::AllAround),
            "left-right" => Ok(RotationStyle::LeftRight),
            "don't rotate" => Ok(RotationStyle::DontRotate),
            _ => Err(()),
        }
    }
    pub fn to_str(&self) -> &str {
        match self {
            RotationStyle::AllAround => "RotationStyle::AllAround",
            RotationStyle::LeftRight => "RotationStyle::LeftRight",
            RotationStyle::DontRotate => "RotationStyle::DontRotate",
        }
    }
}

#[derive(Clone)]
pub enum VideoState {
    On,
    Off,
    OnFlipped,
}

impl VideoState {
    pub fn from_str(input: &str) -> Result<VideoState, ()> {
        match input {
            "on" => Ok(VideoState::On),
            "off" => Ok(VideoState::Off),
            "on-flipped" => Ok(VideoState::OnFlipped),
            _ => Err(()),
        }
    }
    pub fn to_str(&self) -> &str {
        match self {
            VideoState::Off => "VideoState::Off",
            VideoState::On => "VideoState::On",
            VideoState::OnFlipped => "VideoState::OnFlipped",
        }
    }
}

/// A value, for a variable, list, or something else.
///
/// This can represent either a number or a string.
#[derive(Clone)]
enum Value {
    Num(f32),
    String(String),
}

impl Default for Value {
    fn default() -> Self {
        return Self::Num(0.0);
    }
}

impl From<f32> for Value {
    fn from(item: f32) -> Self {
        Value::Num(item)
    }
}

impl From<String> for Value {
    fn from(item: String) -> Self {
        Value::String(item)
    }
}

/// The target trait.
///
/// A target is anything that can run scratch code, meaning
/// either a sprite or the stage.
trait OldTarget {
    // fn new(&self) -> Self;
    fn set_x(&mut self, x: f32);
    fn set_y(&mut self, y: f32);
    fn get_x(&self) -> f32;
    fn get_y(&self) -> f32;
    fn go_to(&mut self, x: f32, y: f32) {
        self.set_x(x);
        self.set_y(y);
    }
    fn change_x_by(&mut self, x: f32) {
        self.set_x(self.get_x() + x);
    }
    fn change_y_by(&mut self, y: f32) {
        self.set_y(self.get_y() + y);
    }
    fn say(&self, text: String) {
        println!("{}", text);
    }
    fn direction(&self) -> f32;
    fn point_direction(&mut self, direction: f32);
    fn set_rotation_style(&mut self, style: RotationStyle);
    fn move_steps(&mut self, steps: f32) {
        let radians = self.direction() * PI / 180.0;
        let dx = steps * radians.cos();
        let dy = steps * radians.sin();
        self.go_to(self.get_x() + dx, self.get_y() + dy);
    }
    fn turn_left(&mut self, degrees: f32) {
        self.point_direction(self.direction() - degrees);
    }
    fn turn_right(&mut self, degrees: f32) {
        self.point_direction(self.direction() + degrees);
    }
    fn get_variable(&self, id: String) -> Option<&Value>;
    fn set_variable(&mut self, id: String, value: &mut Value);
    fn change_variable(&mut self, id: String, amount: f32) {
        // self.set_variable(id, self.get_variable(id).unwrap() + amount);
    }
    fn generate_random(&self, from: i32, to: i32) -> i32 {
        let mut rng = rand::thread_rng();
        return rng.gen_range(from..to);
    }
}

/// The stage.
///
/// Since the stage cannot run some blocks, some
/// of these definitions are empty.
// struct Stage {
//     /// The tempo, in BPM.
//     tempo: i32,
//     /// Determines if video is on or off.
//     videoState: VideoState,
//     /// The video transparency  Defaults to 50.  This has no effect if `videoState`
//     /// is off or the project does not use an exptension with video input.
//     videoTransparency: i32,
//     /// The text to speech language.  Defaults to the editor language.
//     textToSpeechLanguage: String,
//     variables: HashMap<String, Value>,
// }

// impl Target for Stage {
//     // fn new(&self) -> Self {
//     //     return Stage {
//     //         tempo: 60,
//     //         videoState: VideoState::Off,
//     //         videoTransparency: 50,
//     //         textToSpeechLangauge: "en".to_string(),
//     //     };
//     // }
//     fn set_x(&mut self, _x: f32) {}
//     fn set_y(&mut self, _y: f32) {}
//     /// Returns 0
//     fn get_x(&self) -> f32 {
//         return 0.0;
//     }
//     /// Returns 0
//     fn get_y(&self) -> f32 {
//         return 0.0;
//     }
//     fn direction(&self) -> f32 {
//         0.0
//     }
//     /// Does nothing; the stage cannot point in a direction.
//     fn point_direction(&mut self, direction: f32) {}
//     /// Does nothing; the stage cannot "say" anything.
//     fn say(&self, text: String) {}
//     fn set_rotation_style(&mut self, style: RotationStyle) {}
//     // fn set_variable(&mut self, id: String, value: &mut Value);
//     fn get_variable(&self, id: String) -> Option<&Value> {
//         return self.variables.get(&id);
//     }
//     fn set_variable(&mut self, id: String, value: &mut Value) {
//         if self.variables.contains_key(&id) {
//             let mut variable = self.variables.entry(id).or_default();
//             variable = value;
//         }
//     }
// }

// impl Target for Sprite {
//     // fn new(&self) -> Self {
//     //     return Sprite {
//     //         name: String::from("Sprite1"),
//     //         visible: true,
//     //         x: 0,
//     //         y: 0,
//     //         size: 100,
//     //         direction: 90,
//     //         draggable: false,
//     //         rotation_style: RotationStyle::AllAround,
//     //         blocks: Vec::new(),
//     //     };
//     // }
//     fn set_x(&mut self, x: f32) {
//         self.x = x;
//     }
//     fn set_y(&mut self, y: f32) {
//         self.y = y;
//     }
//     fn get_x(&self) -> f32 {
//         return self.x;
//     }
//     fn get_y(&self) -> f32 {
//         return self.y;
//     }
//     fn direction(&self) -> f32 {
//         return self.direction;
//     }
//     fn point_direction(&mut self, direction: f32) {
//         self.direction = direction;
//     }
//     fn set_rotation_style(&mut self, style: RotationStyle) {
//         self.rotation_style = style;
//     }
//     /// Get a variable
//     fn get_variable(&self, id: String) -> Option<&Value> {
//         let variable = self.variables.get(&id);

//         match variable {
//             Some(variable) => return Some(variable),
//             None => {}
//         }

//         let variable = self.stage.get_variable(id);

//         match variable {
//             Some(variable) => return Some(variable),
//             None => return None,
//         }
//     }
//     /// Set a variable
//     fn set_variable(&mut self, id: String, value: &mut Value) {
//         if self.variables.contains_key(&id) {
//             let mut variable = self.variables.entry(id).or_default();
//             variable = value;
//         }
//     }
// }

#[derive(Clone)]
struct Sprite {
    /// Whether the sprite is visibile.  Defaults to true.
    visible: bool,
    /// The x-coordinate.  Defaults to 0.
    x: f32,
    /// The y-coordinate.  Defaults to 0.
    y: f32,
    /// The sprite's size, as a percentage.  Defaults to 100.
    size: f32,
    /// The direction of the sprite in degrees clockwise from up.  Defaults to 90.
    direction: f32,
    /// Whether the sprite is draggable.  Defaults to false.
    draggable: bool,
    /// The rotation style.
    rotation_style: RotationStyle,
    /// The name of the sprite.
    name: String,
    /// The blocks in the sprite.
    /// This is currently only 1 stack of blocks,
    /// but this should change soon.
    // blocks: Vec<Thread<'a>>,
    /// A list of variables for the sprite
    variables: HashMap<String, Value>,
    costume: usize,
}

fn move_steps(object: Option<&mut Sprite>, steps: f32) {
    //unwrap option
    if let Some(uo) = object {
        let radians = uo.direction * PI / 180.0;
        uo.x += steps * radians.cos();
        uo.y += steps * radians.sin();
    }
}

fn set_costume(object: Option<&mut Sprite>, globalCostume: &mut usize, costume: usize) {
    match object {
        Some(sprite) => sprite.costume = costume,
        None => *globalCostume = costume,
    }
}

/// Set the costume of a target.
fn set_costume_better(object: &mut Target, costume: usize) {
    match &mut object.sprite {
        Some(x) => x.costume = costume,
        None => object.stage.current_costume = costume,
    }
}

fn say(speech: String) {
    println!("{}", speech);
}

// impl Sprite {}

// /// A stack of blocks
// struct Stack {
//     stack_type: StackType,
//     reference: String,
// }

// enum StackType {
//     FlagClicked,
// }

/// An emty type that implements future.
struct EmptyFuture {}

/// A future that always returns `()` immediately.
impl Future for EmptyFuture {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::task::Poll::Ready(())
    }
}

struct SpriteFuture {}

/// A thread object.
struct Thread<T: Future> {
    // function: fn(object: Option<&mut Sprite>),
    // function: genawaiter::rc::Gen<(), Option<Sprite>, Pin<Box<dyn Future<Output = ()>>>>,
    function: genawaiter::rc::Gen<Option<Target>, Target, T>,
    // function: genawaiter::rc::Gen<Option<Sprite>, Option<Sprite>, EmptyFuture>,
    // object: &mut Sprite,
    /// The object that this thread works on. The number represents the index of
    /// the object in the program vector. If this is None, it represents the
    /// stage.
    obj_index: Option<usize>,
    /// Whether or not the thread is complete.  If this is true, the thread
    /// is ok to be deleted.
    complete: bool,
}

/// The main project class.  This is in charge of running threads and
/// redrawing the screen.
struct Program<T: Future> {
    threads: Vec<Thread<T>>,
    objects: Vec<Sprite>,
}

impl<T: Future> Program<T> {
    /// Run 1 tick
    fn tick(&mut self, stage: &mut Stage) {
        for (i, thread) in &mut self.threads.iter_mut().enumerate() {
            let returned = match thread.obj_index {
                Some(obj_index_unwrapped) => thread.function.resume_with(Target::new(
                    self.objects[obj_index_unwrapped].clone(),
                    stage.clone(),
                )),

                None => thread
                    .function
                    .resume_with(Target::new_stage(stage.clone())),
            };

            match returned {
                GeneratorState::Yielded(yielded_sprite) => {
                    if let Some(zz) = yielded_sprite {
                        match zz.sprite {
                            Some(y) => {
                                if let Some(obj_index_unwrapped) = thread.obj_index {
                                    self.objects[obj_index_unwrapped] = y;
                                    *stage = zz.stage;
                                }
                            }
                            None => *stage = zz.stage,
                        }
                    }
                }
                GeneratorState::Complete(x) => thread.complete = true,
            }
        }

        // remove all threads that are complete.
        self.threads.retain(|x| !x.complete);
    }

    fn new() -> Self {
        return Program {
            threads: Vec::new(),
            objects: Vec::new(),
        };
    }

    /// Add threads from a sprite to the program.
    /// This _moves_ the threads out of the sprite.
    fn add_threads(&mut self, mut threads: Vec<Thread<T>>) {
        // self.threads.append(&mut sprite.blocks);
        self.threads.append(&mut threads)
    }

    fn add_object(&mut self, object: Sprite) {
        self.objects.push(object);
    }

    /// Add a thread.
    fn add_thread(&mut self, thread: Thread<T>) {
        self.threads.push(thread);
    }
}

/// This is the stage object.
#[derive(Clone)]
struct Stage {
    /// The tempo, in BPM.
    tempo: i32,
    /// Determines if video is on or off.
    video_state: VideoState,
    /// The video transparency  Defaults to 50.  This has no effect if `videoState`
    /// is off or the project does not use an exptension with video input.
    video_transparency: i32,
    /// The text to speech language.  Defaults to the editor language.
    text_to_speech_language: String,
    variables: HashMap<String, Value>,
    current_costume: usize,
}

/// This is a target.
///
/// A target is a combination of a sprite(or not) and the stage. If the sprite
/// is absent(None), the target is assumed to be the stage.
struct Target {
    /// The stage.
    ///
    /// This is an owned variable, meaning it is a copy of the real stage. The
    /// real stage should be set to this copy when the target is returned from a
    /// thread.
    stage: Stage,
    sprite: Option<Sprite>,
}

impl Target {
    /// Create a new sprite target.
    fn new(sprite: Sprite, stage: Stage) -> Self {
        Self {
            sprite: Some(sprite),
            stage,
        }
    }

    /// Create a new stage target.
    fn new_stage(stage: Stage) -> Self {
        Self {
            sprite: None,
            stage,
        }
    }
}

// enum Target {
//     Sprite {
//         /// Whether the sprite is visibile.  Defaults to true.
//         visible: bool,
//         /// The x-coordinate.  Defaults to 0.
//         x: f32,
//         /// The y-coordinate.  Defaults to 0.
//         y: f32,
//         /// The sprite's size, as a percentage.  Defaults to 100.
//         size: f32,
//         /// The direction of the sprite in degrees clockwise from up.  Defaults to 90.
//         direction: f32,
//         /// Whether the sprite is draggable.  Defaults to false.
//         draggable: bool,
//         /// The rotation style.
//         rotation_style: RotationStyle,
//         /// The name of the sprite.
//         name: String,
//         /// The blocks in the sprite.
//         /// This is currently only 1 stack of blocks,
//         /// but this should change soon.
//         blocks: Vec<Thread>,
//         /// A list of variables for the sprite
//         variables: HashMap<String, Value>,
//     },
//     Stage {
//         /// The tempo, in BPM.
//         tempo: i32,
//         /// Determines if video is on or off.
//         video_state: VideoState,
//         /// The video transparency  Defaults to 50.  This has no effect if `videoState`
//         /// is off or the project does not use an exptension with video input.
//         video_transparency: i32,
//         /// The text to speech language.  Defaults to the editor language.
//         text_to_speech_language: String,
//         variables: HashMap<String, Value>,
//     },
// }

// impl Target {
//     // fn new(&self) -> Self;
//     fn set_x(&mut self, xin: f32) {
//         match self {
//             Target::Sprite { ref mut x, .. } => *x = xin,
//             Target::Stage { .. } => {}
//         }
//     }
//     fn set_y(&mut self, yin: f32) {
//         match self {
//             Target::Sprite { ref mut y, .. } => *y = yin,
//             Target::Stage { .. } => {}
//         }
//     }
//     fn get_x(&self) -> f32 {
//         // TODO document case for stage.
//         match self {
//             Target::Sprite { x, .. } => *x,
//             _ => 0.0,
//         }
//     }
//     fn get_y(&self) -> f32 {
//         match self {
//             Target::Sprite { y, .. } => *y,
//             _ => 0.0,
//         }
//     }
//     fn go_to(&mut self, x: f32, y: f32) {
//         self.set_x(x);
//         self.set_y(y);
//     }
//     fn change_x_by(&mut self, x: f32) {
//         self.set_x(self.get_x() + x);
//     }
//     fn change_y_by(&mut self, y: f32) {
//         self.set_y(self.get_y() + y);
//     }
//     fn say(&self, text: String) {
//         println!("{}", text);
//     }
//     fn direction(&self) -> f32 {
//         match self {
//             Self::Sprite { direction, .. } => *direction,
//             _ => 0.0,
//         }
//     }
//     fn point_direction(&mut self, direction_in: f32) {
//         match self {
//             Self::Sprite { direction, .. } => *direction = direction_in,
//             Self::Stage { .. } => {}
//         }
//     }
//     fn set_rotation_style(&mut self, style: RotationStyle) {
//         match self {
//             Self::Sprite { rotation_style, .. } => *rotation_style = style,
//             Self::Stage { .. } => {}
//         }
//     }
//     fn move_steps(&mut self, steps: f32) {
//         let radians = self.direction() * PI / 180.0;
//         let dx = steps * radians.cos();
//         let dy = steps * radians.sin();
//         self.go_to(self.get_x() + dx, self.get_y() + dy);
//     }
//     fn turn_left(&mut self, degrees: f32) {
//         self.point_direction(self.direction() - degrees);
//     }
//     fn turn_right(&mut self, degrees: f32) {
//         self.point_direction(self.direction() + degrees);
//     }
//     fn get_variable(&self, id: String) -> Option<&Value> {
//         // TODO implement
//         unimplemented!();
//         //None
//     }
//     fn set_variable(&mut self, id: String, value: &mut Value) {}
//     fn change_variable(&mut self, id: String, amount: f32) {
//         // self.set_variable(id, self.get_variable(id).unwrap() + amount);
//     }
//     fn generate_random(&self, from: i32, to: i32) -> i32 {
//         let mut rng = rand::thread_rng();
//         return rng.gen_range(from..to);
//     }
// }
