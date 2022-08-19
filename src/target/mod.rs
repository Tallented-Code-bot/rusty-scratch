extern crate rand;
use std::{collections::HashMap, f32::consts::PI};
use rand::Rng;

#[derive(Clone,Debug)]
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

impl std::fmt::Display for RotationStyle{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RotationStyle::AllAround => write!(f,"RotationStyle::AllAround"),
            RotationStyle::LeftRight => write!(f,"RotationStyle::LeftRight"),
            RotationStyle::DontRotate => write!(f,"RotationStyle::DontRotate"),
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

/// A thread object.
#[derive(Clone)]
struct Thread<'a> {
    function: fn(object: &mut Target),
    object:&'a Target<'a>,
}

/// The main project class.  This is in charge of running threads and
/// redrawing the screen.
struct Program<'a>{
    threads:Vec<(Thread<'a>,usize)>,
    //targets:Vec<Box<dyn Target>>
    targets:Vec<Target<'a>>
}

impl Program<'_>{
    /// Run 1 tick
    fn tick(&mut self){
        for thread in &mut self.threads{
            //(thread.0.function)(&mut thread.1);
            (thread.0.function)(&mut self.targets[thread.1])
        }
    }
    fn new()->Self{
        return Program{
            threads:Vec::new(),
            targets:Vec::new()
        }
    }

    /// Add threads from a sprite to the program.
    /// This _moves_ the threads out of the sprite.
    fn add_threads<'a>(&mut self){
        //for thread in sprite.blocks{
        //    self.threads.push((thread,&sprite))
        //}
        for (index,target) in self.targets.iter().enumerate(){
            for thread in target.blocks.clone(){
                self.threads.push((thread,index));
            }
        }

        //self.threads.append(&mut sprite.blocks);
    }

}

/// A target(either a sprite or the stage).
#[derive(Clone)]
struct Target<'a>{
    /// Whether the target is the stage or not
    isStage:bool,
    /// The name of the target.
    /// It should always be "Stage" if `isStage` is true.
    name:String,
    /// A hashmap of variables.
    variables:HashMap<String,Value>,
    /// The lists in this target.
    /// UNIMPLEMENTED
    lists:(),

    /// A list of broadcasts.  This is normally only present in the stage.
    /// UNIMPLEMENTED
    broadcasts:(),
    /// The blocks in the target, grouped in threads(hats).
    blocks:Vec<Thread<'a>>,
    /// The current costume or backdrop number.
    currentCostume:u32,
    /// A list of costumes.
    /// UNIMPLEMENTED
    costumes:(),
    /// A list of sounds.
    /// UNIMPLEMENTED
    sounds:(),
    layerOrder:u32,
    volume:u32,
    // Stage variables ------------------------------------v------------------------------------
    /// The tempo, in BPM.
    tempo: i32,
    /// Determines if video is on or off.
    videoState: VideoState,
    /// The video transparency  Defaults to 50.  This has no effect if `videoState`
    /// is off or the project does not use an exptension with video input.
    videoTransparency: i32,
    /// The text to speech language.  Defaults to the editor language.
    textToSpeechLanguage: String,
    //Sprite variables ------------------------------------v------------------------------------
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
    /// A reference to the stage
    stage:Option<&'a Target<'a>>,
    //-------------------------------------------------------------------------------------------
}

impl Target<'_>{
    // fn set_x(&mut self, x: f32);
    // fn set_y(&mut self, y: f32);
    // fn get_x(&self) -> f32;
    // fn get_y(&self) -> f32;
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
    fn change_variable(&mut self, id: String, amount: f32) {
        // self.set_variable(id, self.get_variable(id).unwrap() + amount);
    }
    fn generate_random(&self,from:i32,to:i32)->i32{
        let mut rng=rand::thread_rng();
        return rng.gen_range(from..to);
    }







    fn set_x(&mut self, x: f32) {
        self.x = x;
    }
    fn set_y(&mut self, y: f32) {
        self.y = y;
    }
    fn get_x(&self) -> f32 {
        return self.x;
    }
    fn get_y(&self) -> f32 {
        return self.y;
    }
    fn direction(&self) -> f32 {
        return self.direction;
    }
    fn point_direction(&mut self, direction: f32) {
        self.direction = direction;
    }
    fn set_rotation_style(&mut self, style: RotationStyle) {
        self.rotation_style = style;
    }
    /// Get a variable
    fn get_variable(&self, id: String) -> Option<&Value> {
        let variable = self.variables.get(&id);

        match variable {
            Some(variable) => return Some(variable),
            None => {}
        }
        

        let variable = match self.stage{
            Some(stage)=>{stage.get_variable(id)},
            None=>{None}
        };


        match variable {
            Some(variable) => return Some(variable),
            None => return None,
        }
    }
    /// Set a variable
    fn set_variable(&mut self, id: String, value: &mut Value) {
        if self.variables.contains_key(&id) {
            let mut variable = self.variables.entry(id).or_default();
            variable = value;
        }
    }
}