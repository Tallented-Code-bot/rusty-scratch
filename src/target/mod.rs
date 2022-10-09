extern crate rand;
use genawaiter::stack;
use rand::Rng;
use std::{collections::HashMap, f32::consts::PI};

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

// struct Sprite {
//     /// Whether the sprite is visibile.  Defaults to true.
//     visible: bool,
//     /// The x-coordinate.  Defaults to 0.
//     x: f32,
//     /// The y-coordinate.  Defaults to 0.
//     y: f32,
//     /// The sprite's size, as a percentage.  Defaults to 100.
//     size: f32,
//     /// The direction of the sprite in degrees clockwise from up.  Defaults to 90.
//     direction: f32,
//     /// Whether the sprite is draggable.  Defaults to false.
//     draggable: bool,
//     /// The rotation style.
//     rotation_style: RotationStyle,
//     /// The name of the sprite.
//     name: String,
//     /// The blocks in the sprite.
//     /// This is currently only 1 stack of blocks,
//     /// but this should change soon.
//     blocks: Vec<Thread>,
//     /// A list of variables for the sprite
//     variables: HashMap<String, Value>,
// }

// impl Sprite {}

// /// A stack of blocks
// struct Stack {
//     stack_type: StackType,
//     reference: String,
// }

// enum StackType {
//     FlagClicked,
// }

/// A thread object.
struct Thread {
    function: fn(object: &mut Target),
    // object: &'a mut Target<'a>,
}

/// The main project class.  This is in charge of running threads and
/// redrawing the screen.
struct Program {
    threads: Vec<Thread>,
}

impl Program {
    /// Run 1 tick
    fn tick(&mut self) {
        for thread in &mut self.threads {
            // (thread.function)(&mut thread.object);
        }
    }
    fn new() -> Self {
        return Program {
            threads: Vec::new(),
        };
    }

    /// Add threads from a sprite to the program.
    /// This _moves_ the threads out of the sprite.
    fn add_threads(&mut self, mut threads: Vec<Thread>) {
        // self.threads.append(&mut sprite.blocks);
        self.threads.append(&mut threads)
    }
}

enum Target {
    Sprite {
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
        blocks: Vec<Thread>,
        /// A list of variables for the sprite
        variables: HashMap<String, Value>,
    },
    Stage {
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
    },
}

impl Target {
    // fn new(&self) -> Self;
    fn set_x(&mut self, xin: f32) {
        match self {
            Target::Sprite { ref mut x, .. } => *x = xin,
            Target::Stage { .. } => {}
        }
    }
    fn set_y(&mut self, yin: f32) {
        match self {
            Target::Sprite { ref mut y, .. } => *y = yin,
            Target::Stage { .. } => {}
        }
    }
    fn get_x(&self) -> f32 {
        // TODO document case for stage.
        match self {
            Target::Sprite { x, .. } => *x,
            _ => 0.0,
        }
    }
    fn get_y(&self) -> f32 {
        match self {
            Target::Sprite { y, .. } => *y,
            _ => 0.0,
        }
    }
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
    fn direction(&self) -> f32 {
        match self {
            Self::Sprite { direction, .. } => *direction,
            _ => 0.0,
        }
    }
    fn point_direction(&mut self, direction_in: f32) {
        match self {
            Self::Sprite { direction, .. } => *direction = direction_in,
            Self::Stage { .. } => {}
        }
    }
    fn set_rotation_style(&mut self, style: RotationStyle) {
        match self {
            Self::Sprite { rotation_style, .. } => *rotation_style = style,
            Self::Stage { .. } => {}
        }
    }
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
    fn get_variable(&self, id: String) -> Option<&Value> {
        // TODO implement
        unimplemented!();
        //None
    }
    fn set_variable(&mut self, id: String, value: &mut Value) {}
    fn change_variable(&mut self, id: String, amount: f32) {
        // self.set_variable(id, self.get_variable(id).unwrap() + amount);
    }
    fn generate_random(&self, from: i32, to: i32) -> i32 {
        let mut rng = rand::thread_rng();
        return rng.gen_range(from..to);
    }
}
