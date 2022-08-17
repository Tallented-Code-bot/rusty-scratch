use crate::target::{RotationStyle, VideoState};
use json::{self, JsonValue};
use std::collections::HashMap;
use std::error::Error;
use std::process::{Command,Output};
use std::fs;
use std::io::Read;
use std::io;
use zip;
use std::fs::File;
use reqwest;

mod target;
pub mod thread;

/// Creates a block reference hashmap.
/// This translates scratch code to rust code.
///
/// The function that are used here are defined in
/// `src/target/mod.rs`, in the `Sprite` and `Stage` structs,
/// and the `Target` trait.
///
/// # Usage
/// ```rust
/// // Generate the reference
/// let block_reference=make_blocks_lookup();
///
/// // Now we can use the reference in other functions.
/// follow_stack(block,block_id,block_reference);
/// ```
fn make_blocks_lookup() -> HashMap<&'static str, &'static str> {
    let mut blocks: HashMap<&str, &str> = HashMap::new();
    blocks.insert("motion_setx", "object.set_x(Xf32);");
    blocks.insert("motion_sety", "object.set_y(Yf32)");
    blocks.insert("motion_changexby", "object.change_x_by(DXf32)");
    blocks.insert("motion_changeyby", "object.change_y_by(DYf32);");
    blocks.insert("motion_movesteps", "object.move_steps(STEPSf32);");
    blocks.insert("motion_turnleft", "object.turn_left(DEGREESf32)");
    blocks.insert("motion_turnright", "object.turn_right(DEGREESf32)");
    blocks.insert("motion_gotoxy", "object.go_to(Xf32,Yf32)");
    blocks.insert("event_whenflagclicked", "flag_clicked();");
    blocks.insert("control_repeat", "for x in 0..TIMES{SUBSTACK}"); //TODO add yielding
    blocks.insert("control_forever", "loop{SUBSTACK}"); //TODO add yielding
    blocks.insert("control_if", "if CONDITION {SUBSTACK}");
    blocks.insert("control_if_else", "if CONDITION {SUBSTACK}else{SUBSTACK2}");
    blocks.insert("control_repeat_until", "while !CONDITION{SUBSTACK}"); //TODO add yielding
    blocks.insert("looks_say", "object.say(String::from(\"MESSAGE\"));");
    blocks.insert("event_whenflagclicked", "");
    blocks.insert("data_variable", "object.get_variable(VARIABLE)");
    blocks.insert(
        "data_setvariableto",
        "object.set_variable(String::from(\"VARIABLE\"),&mut Value::from(VALUEf32));",
    );
    blocks.insert(
        "data_changevariableby",
        "object.change_variable(String::from(\"VARIABLE\"),VALUEf32);",
    );
    blocks.insert("operator_add","NUM1+NUM2");
    blocks.insert("operator_subtract","NUM1-NUM2");
    blocks.insert("operator_multiply","NUM1*NUM2");
    blocks.insert("operator_divide", "NUM1/NUM2");
    blocks.insert("operator_random","generate_random(FROM,TO)");
    blocks.insert("operator_lt","OPERAND1<OPERAND2");
    blocks.insert("operator)_equals","OPERAND1=OPERAND2");
    blocks.insert("operator_gt","OPERAND1>OPERAND2");
    blocks.insert("operator_and","OPERAND1&&OPERAND2");
    blocks.insert("operator_or","OPERAND1||OPERAND2");
    blocks.insert("operator_not","!OPERAND");

    return blocks;
}

fn main() {
    // let file = fs::read_to_string("./project.json").expect("Could not read file");
    // let project = get_project(String::from("./test_variables.sb3")).unwrap(); // TODO add proper error handling
    let project = get_project_online(720925925).unwrap(); // TODO add proper error handling
    std::fs::write("project.json",project.to_string());
    let block_reference = make_blocks_lookup();
    create_project(); // create a new cargo project
    //510186917
    let filename = "output/src/main.rs";


    // Get the library file to include
    let lib = include_str!("../target/target.rs");

    let mut targets: Vec<String> = Vec::new();

    for target in project["targets"].members() {
        targets.push(generate_target(target, &block_reference));
    }

    let output = format!(
        "
// This is the static Sprite, Stage, and block definitions
        {lib}
        //########################################
        // Below this is generated code.

        fn main(){{
            {targets}
            // (Sprite1.blocks.function)(&mut Sprite1);
            
            let mut program=Program::new();
            program.add_threads(&mut Sprite1);
        }}
        ",
        lib = lib,
        targets = targets.join("\n"),
        // sprite1 = generate_target(&project["targets"][1], &block_reference)
    );

    fs::write(filename, output).expect("Could not write file.");
    format_file(filename.to_string()).expect("Could not format file.");

    // ###############################################

    // write_to_file(
    //     (
    //         "l;hbH%/NGXC+[%R,/D9_",
    //         &project["targets"][1]["blocks"]["l;hbH%/NGXC+[%R,/D9_"],
    //     ),
    //     &project["targets"][1]["blocks"],
    //     &block_reference,
    // )
}

/// Get the `project.json` file from a scratch `sb3` file.
fn get_project(filename: String) -> Result<JsonValue, Box<dyn Error>> {
    let file = std::path::Path::new(&filename);
    let zipfile = std::fs::File::open(&file)?;

    let mut archive = zip::ZipArchive::new(zipfile)?;

    let mut file = match archive.by_name("project.json") {
        Ok(file) => file,
        Err(..) => {
            // return Err(String::from("Could not find project.json"));
            return Err(From::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Could not find project.json",
            )));
        }
    };

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let project = json::parse(&contents as &str)?;
    return Ok(project);
}


/// Get an online scratch project.
fn get_project_online(id:u32)->Result<JsonValue,Box<dyn Error>>{
    // Create the project url.
    let url=format!("https://projects.scratch.mit.edu/{}",id);

    // Get the project (`fetch_sb3_file` creates a file called "project.json".
    // I have not yet figured out how to get the project assets.)
    let project=json::parse(fetch_sb3_file(url).as_str())?;

    return Ok(project);
}

/// Turn 1 block into a rust function.  If the block
/// has a substack(a block such as a loop, or an if-statement),
/// then the substack will also be returned inside the main block.
fn get_block(
    block: (&str, &JsonValue),
    blocks: &JsonValue,
    block_reference: &HashMap<&str, &str>,
) -> String {
    let (id, data) = block;
    let opcode = data["opcode"].to_string();
    println!("{}", block.1);
    let mut function = block_reference.get(&opcode as &str).unwrap().to_string(); // TODO better error handling

    // iterate over each input
    for input in data["inputs"].entries() {
        if input.1[1].is_array() {
            // If the input is an array, it must be a single value.
            println!("{}", input.1[1]);

            function = match input.1[1][0].as_u32().unwrap() {
                4 => function.replacen(input.0, &input.1[1][1].as_str().unwrap().to_string(), 1), // Number
                5 => function.replacen(input.0, &input.1[1][1].as_str().unwrap().to_string(), 1), // Positive number
                6 => function.replacen(input.0, &input.1[1][1].as_str().unwrap().to_string(), 1), // Positive integer
                7 => function.replacen(input.0, &input.1[1][1].as_str().unwrap().to_string(), 1), // Integer
                8 => function.replacen(input.0, &input.1[1][1].as_str().unwrap().to_string(), 1), // Angle
                9 => function.replacen(input.0, input.1[1][1].as_str().unwrap(), 1), // Color
                10 => function.replacen(input.0, input.1[1][1].as_str().unwrap(), 1), // String
                11 => todo!(),
                12 => function.replacen( // Variable
                    input.0,
                    format!(
                        "object.get_variable({})",
                        input.1[1][2].as_str().unwrap()
                    ).as_str(),
                    1,
                ),
                13 => todo!(),
                _ => {
                    unreachable!()
                }
            };
            function = function.replacen(input.0, input.1[1][1].as_str().unwrap(), 1);
        } else if input.1[1].is_string() {
            // otherwise, it must be a substack.
            // TODO get more than the first block

            // Recursively follow the substack...
            let subfunc = get_block(
                (
                    input.1[1].as_str().unwrap(), // The id of the block we want to go to
                    &blocks[input.1[1].as_str().unwrap()], //The object corresponding to the id
                ),
                blocks,           // list of blocks
                &block_reference, //block reference
            );
            // ... and insert the subfunction in the main function variable.
            function = function.replacen(input.0, &subfunc, 1);
        }
    }

    for field in data["fields"].entries() {
        function = function.replacen(field.0, field.1[1].as_str().unwrap(), 1);
    }

    // Return the completed function
    return function;
}

/// Follow a stack of scratch blocks.
fn follow_stack(
    block: (&str, &JsonValue),
    blocks: &JsonValue,
    block_reference: &HashMap<&str, &str>,
) -> Vec<String> {
    // Create a list of functions
    let mut stack: Vec<String> = Vec::new();

    let mut currentBlock = block;

    loop {
        // push the current block onto the stack.
        stack.push(get_block(currentBlock, blocks, block_reference));

        // If there is no next block, break out of the loop.
        if currentBlock.1["next"].is_null() {
            break;
        }
        currentBlock = (
            currentBlock.1["next"].as_str().unwrap(),
            &blocks[currentBlock.1["next"].as_str().unwrap()],
        );
    }
    return stack;
}

/// Create a hat block definition function.
fn create_hat(
    block: (&str, &JsonValue),
    blocks: &JsonValue,
    block_reference: &HashMap<&str, &str>,
) -> Result<String, String> {
    // Make sure the block is a top level block.
    if !block.1["topLevel"].as_bool().unwrap() {
        return Err(String::from("Not a top level block"));
    }
    // Make sure the block has no parent.
    if !block.1["parent"].is_null() {
        return Err(String::from("Block has a parent"));
    }

    // Get the contents of the stack
    let contents = follow_stack(
        (
            &block.1["next"].to_string(),         // the ID of the next block
            &blocks[block.1["next"].to_string()], // the next block
        ),
        blocks,
        block_reference,
    );

    // let function = "fn NAME (){CONTENTS}";
    // let name=format!{}
    let function = format!("|object: &mut dyn Target|{{{}}}", contents.join("\n"));

    // TODO Remove this
    return Ok(String::from(function));
}

/// Returns all stacks of blocks.
fn create_all_hats(
    blocks: &JsonValue,
    block_reference: &HashMap<&str, &str>,
)->Result<String,String>{
    let mut contents:String=String::new();

    for block in blocks.entries(){
        let hat=create_hat(block,blocks,block_reference);
        match hat{
            Ok(x)=>{contents.push_str(format!("Thread{{function:{},object:Self}}",x.as_str()).as_str())},
            Err(x)=>{continue;}
        }
    } 
    //return Err(String::from("Bad"));
    return Ok(format!("{}",contents));
}

/// Writes the output rust file.
fn write_to_file(
    block: (&str, &JsonValue),
    blocks: &JsonValue,
    block_reference: &HashMap<&str, &str>,
) {
    let filename = "output.rs";

    // Get the library file to include
    let lib = include_str!("../target/target.rs");

    let function = create_hat(block, blocks, block_reference).unwrap();

    fs::write(filename, format!("{}\n\n\n{}", lib, function)).expect("Could not write file.");
}

/// Generate a new target(sprite or stage) from json.
fn generate_target(target: &JsonValue, block_reference: &HashMap<&str, &str>) -> String {
    // If the target is the stage
    if target["isStage"].as_bool().unwrap() {
        return format!(
            "let mut {name}=Stage{{
                tempo:{tempo},
                videoState:{videoState},
                videoTransparency:{videoTransparency},
                textToSpeechLanguage:String::from(\"{textToSpeechLanguage}\"),
                variables:HashMap::new(),
            }};",
            name = target["name"],
            tempo = target["tempo"],
            videoState = VideoState::from_str(target["videoState"].as_str().unwrap())
                .unwrap()
                .to_str(),
            textToSpeechLanguage = target["textToSpeechLanguage"],
            videoTransparency = target["videoTransparency"]
        );
    } else {
        /* let function = create_hat(
            (
                ";R9G|C|f#(g@5F[3Im)I",
                &target["blocks"][";R9G|C|f#(g@5F[3Im)I"],
            ),
            &target["blocks"],
            &block_reference,
        )
        .unwrap(); */
        let function=create_all_hats(&target["blocks"], &block_reference).unwrap();

        return format!(
            "let mut {name}=Sprite{{
                visible:{visible},
                x:{x}f32,
                y:{y}f32,
                size:{size}f32,
                direction:{direction}f32,
                draggable:{draggable},
                rotation_style:{rotationStyle},
                name:\"{name}\".to_string(),
                blocks:vec![ {function} ],
                stage:Stage,
                variables:HashMap::new(),
            }};",
            name = target["name"],
            visible = target["visible"],
            x = target["x"],
            y = target["y"],
            size = target["size"],
            direction = target["direction"],
            draggable = target["draggable"],
            rotationStyle = RotationStyle::from_str(target["rotationStyle"].as_str().unwrap())
                .unwrap()
                .to_str(),
        );
    }
}

// struct blockstack
//
// type of hat



/// Fetch a scratch sb3 file.
fn fetch_sb3_file(url:String)->String{
    // fetch the file,
    let mut response=reqwest::blocking::get(url).expect("Could not get file.");
    // create a new file,
    //let mut out=File::create("project.sv3").expect("Could not create file");
    // and copy the contents of the response to the new file.
    //response.copy_to(&mut out);
    // io::copy(&mut response,&mut out)?;
    return response.text().unwrap();
}

/// Formats the given filename with rustfmt.
fn format_file(filename: String)->io::Result<Output>{
    return Command::new("rustfmt")
        .arg(filename)
        .output();
        //.expect("Could not execute rustfmt"); 
}

fn create_project()->Result<(),io::Error>{
    Command::new("cargo")
        .arg("new")
        .arg("output")
        .output()?;
    
    let toml="
    [package]
    name=\"output\"
    version=\"1.0.0\"
    edition=\"2021\"
    
    [dependencies]
    rand=\"0.8.5\"
    genawaiter=\"0.99.1\"
    ";

    fs::write("output/Cargo.toml",toml)?;

    return Ok(());
}