use crate::target::{RotationStyle, Value, VideoState};
use json::{self, JsonValue};
use opengl_graphics::CreateTexture;
use rand::Rng;
use resvg;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::{self, Cursor};
use std::process::{Command, Output};
use target::StartType;
use ureq;
use zip;

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
    blocks.insert("motion_setx", "set_x(&mut object,Xf32);");
    blocks.insert("motion_sety", "set_y(&mut object,Yf32)");
    blocks.insert("motion_changexby", "change_x_by(&mut object,DXf32)");
    blocks.insert("motion_changeyby", "change_y_by(&mut object,DYf32);");
    // blocks.insert("motion_movesteps", "object.move_steps(STEPSf32);");
    blocks.insert("motion_movesteps", "move_steps(sprite.clone(),STEPSf32);");
    blocks.insert("motion_turnleft", "turn_left(&mut object,DEGREESf32)");
    blocks.insert("motion_turnright", "turn_right(&mut object,DEGREESf32)");
    blocks.insert("motion_gotoxy", "go_to(&mut object,Xf32,Yf32)");
    blocks.insert(
        "motion_pointindirection",
        "point_in_direction(&mut object,DIRECTIONf32)",
    );
    blocks.insert("event_whenflagclicked", "flag_clicked();");
    blocks.insert(
        "control_repeat",
        "for z in 0..TIMES.into(){SUBSTACK\nYield::Start.await;}",
    ); //TODO add yielding
    blocks.insert("control_forever", "loop{SUBSTACK\nYield::Start.await;}");
    blocks.insert("control_if", "if CONDITION {SUBSTACK}");
    blocks.insert("control_if_else", "if CONDITION {SUBSTACK}else{SUBSTACK2}");
    blocks.insert("control_wait_until", "wait_until(CONDITION).await;");
    blocks.insert(
        "control_repeat_until",
        "while !CONDITION{SUBSTACK\nYield::Start.await;}",
    );

    // blocks.insert("looks_say", "object.say(String::from(\"MESSAGE\"));");
    blocks.insert("looks_say", "say(Value::from(MESSAGE));");
    blocks.insert("event_whenflagclicked", "");
    // blocks.insert(
    //     "data_variable",
    //     "get_variable(Some(sprite.clone()),stage.clone(),VARIABLE)",
    // );
    blocks.insert(
        "data_setvariableto",
        "object.set_variable(String::from(\"VARIABLE\"),&mut Value::from(VALUEf32));",
    );
    blocks.insert(
        "data_changevariableby",
        "object.change_variable(String::from(\"VARIABLE\"),VALUEf32);",
    );
    blocks.insert("operator_add", "NUM1+NUM2");
    blocks.insert("operator_subtract", "NUM1-NUM2");
    blocks.insert("operator_multiply", "NUM1*NUM2");
    blocks.insert("operator_divide", "NUM1/NUM2");
    blocks.insert("operator_random", "generate_random(FROM,TO)");
    blocks.insert("operator_lt", "OPERAND1<OPERAND2");
    blocks.insert("operator)_equals", "OPERAND1=OPERAND2");
    blocks.insert("operator_gt", "OPERAND1>OPERAND2");
    blocks.insert("operator_and", "OPERAND1&&OPERAND2");
    blocks.insert("operator_or", "OPERAND1||OPERAND2");
    blocks.insert("operator_not", "!OPERAND");
    blocks.insert("argument_reporter_string_number", "VALUE");

    return blocks;
}

/// This represents a custom block.
struct CustomBlock {
    code: String,
    /// A mapping of argument ids to names.
    argument_ids_names: HashMap<String, String>,
}

fn main() {
    // let file = fs::read_to_string("./project.json").expect("Could not read file");
    // let project = get_project(String::from("./test_variables.sb3")).unwrap(); // TODO add proper error handling
    let project = get_project_online(759912461).expect("Could not fetch project"); // TODO add proper error handling
    std::fs::write("project.json", project.to_string()).expect("Could not write to project.json");
    let block_reference = make_blocks_lookup();
    create_project().expect("Could not create new rust project"); // create a new cargo project
                                                                  //510186917
    let filename = "output/src/main.rs";

    // Get the library file to include
    let lib = include_str!("../target/target.rs");

    let mut targets: Vec<String> = Vec::new();

    for target in project["targets"].members() {
        targets.push(generate_target(target, &block_reference));
        get_target_assets(target);
    }

    let output = format!(
        "
        // This is the static Sprite, Stage, and block definitions
        {lib}
        //########################################
        // Below this is generated code.

        fn main(){{
            let opengl = OpenGL::V3_2;

            // Create a glutin window
            let mut window: Window = WindowSettings::new(\"rusty-scratch\",[200,200])
                .graphics_api(opengl)
                .exit_on_esc(true)
                .build()
                .unwrap();

            let mut program=Program::new();
            let mut sprites: Vec<Rc<Mutex<Sprite>>> = Vec::new();

            {targets}
            // (Sprite1.blocks.function)(&mut Sprite1);
            
            //program.add_threads(Sprite1.blocks);
            //program.add_all_threads();


            let mut events = Events::new(EventSettings::new());
            events.set_max_fps(30);
            events.set_ups(30);
            program.click_flag();
            while let Some(e) = events.next(&mut window){{
                if let Some(args) = e.render_args(){{
                    program.render(&args,Stage.clone());
                }}
                if let Some(args) = e.update_args(){{
                    program.tick(/*Stage.clone()*/);
                }}
            }}

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
fn get_project_online(id: u32) -> Result<JsonValue, Box<dyn Error>> {
    // Create the project url.
    let token = fetch_project_token(id)?;
    let url = format!("https://projects.scratch.mit.edu/{id}?token={token}");

    // Get the project (`fetch_sb3_file` creates a file called "project.json".
    // I have not yet figured out how to get the project assets.)
    let project = json::parse(fetch_sb3_file(url).as_str())?;

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
    // println!("{}", block.1);
    let mut function;
    if opcode == "procedures_call" {
        let cblock = data["mutation"]["proccode"].to_string();
        function = format!(
            "stack_procedures_definition_{}(sprite.clone(),stage.clone()).await;",
            cblock
        );
    } else {
        function = match block_reference.get(&opcode as &str) {
            Some(x) => x.to_string(),
            None => {
                println!("Error: unknown block (opcode {})", opcode);
                panic!();
            }
        };
    }

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
                10 => function.replacen(
                    input.0,
                    &*format!("String::from(\"{}\")", input.1[1][1].as_str().unwrap()),
                    1,
                ), // String
                11 => todo!(),
                12 => function.replacen(
                    // Variable
                    input.0,
                    format!(
                        "get_variable(Some(sprite.clone()),stage.clone(),\"{}\")",
                        input.1[1][2].as_str().unwrap()
                    )
                    .as_str(),
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

    let mut current_block = block;

    loop {
        // push the current block onto the stack.
        stack.push(get_block(current_block, blocks, block_reference));

        // If there is no next block, break out of the loop.
        if current_block.1["next"].is_null() {
            break;
        }
        current_block = (
            current_block.1["next"].as_str().unwrap(),
            &blocks[current_block.1["next"].as_str().unwrap()],
        );
    }
    return stack;
}
fn handle_custom_block(
    block: (&str, &JsonValue),
    blocks: &JsonValue,
    block_reference: &HashMap<&str, &str>,
) -> Option<(String, String)> {
    if block.1["opcode"] != "procedures_definition" {
        return None;
    }

    // the "inner" block definition
    let prototype = (
        block.1["inputs"]["custom_block"][1].as_str().unwrap(),
        &blocks[block.1["inputs"]["custom_block"][1].as_str().unwrap()],
    );

    let input_names = json::parse(&prototype.1["mutation"]["argumentnames"].to_string()).unwrap();
    let input_ids = json::parse(&prototype.1["mutation"]["argumentids"].to_string()).unwrap();
    let mut inputs = HashMap::new();

    for (id, name) in input_ids.members().zip(input_names.members()) {
        inputs.insert(id.to_string(), name.to_string());
    }

    let mut function = follow_stack(
        (
            &block.1["next"].to_string(),
            &blocks[block.1["next"].to_string()],
        ),
        blocks,
        block_reference,
    )
    .join("\n");

    // If run without screen refresh is enabled, remove all
    // yields.
    //
    let warp = &prototype.1["mutation"]["warp"];
    if (warp.is_boolean() && warp.as_bool().expect("Must be bool"))
        || (warp.is_string() && warp.as_str().expect("Must be str") == "true")
    {
        function = function.replace("yield_!(Some(object));", "");
    }

    let proccode = prototype.1["mutation"]["test_block_1"].to_string();

    // format!("{}", function);

    /*
     * let input_1=XKCD;
     * move_steps(input_1);
     * change_size(input_1);
     */

    Some((proccode, function))
}

/// Create a hat block definition function.
fn create_hat(
    block: (&str, &JsonValue),
    blocks: &JsonValue,
    block_reference: &HashMap<&str, &str>,
    custom_blocks: &HashMap<String, String>,
) -> Result<(String, StartType, String, bool), String> {
    // Make sure the block is a top level block.
    if !block.1["topLevel"].as_bool().unwrap() {
        return Err(String::from("Not a top level block"));
    }
    // Make sure the block has no parent.
    if !block.1["parent"].is_null() {
        return Err(String::from("Block has a parent"));
    }

    let mut custom_block = false;
    let start_type = match block.1["opcode"].as_str().unwrap() {
        "procedures_call" => return Err(String::from("Custom block")),
        "event_whenflagclicked" => StartType::FlagClicked,
        "procedures_define" => {
            custom_block = true;
            StartType::NoStart
        }
        _ => StartType::NoStart,
    };

    // if let Some(x) = handle_custom_block(block, blocks, block_reference) {
    //     return Ok(x);
    // }

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
    let mut function = format!(
        //         "gen!({{
        // let mut object:Target =yield_!(None);
        // {}
        // yield_!(Some(object));
        // }})",
        "{}",
        contents.join("\n")
    );

    let mut rng = rand::thread_rng();
    let mut warp = false;
    let name = if block.1["opcode"] == "procedures_definition" {
        let prototype = &blocks[block.1["inputs"]["custom_block"][1].to_string()];
        if prototype["mutation"]["warp"] == "true" {
            warp = false; //true
        }
        function = function.replace("Yield::Start.await;", ""); // remove all yields
        format!(
            "procedures_definition_{}",
            prototype["mutation"]["proccode"]
        )
    } else {
        format!(
            "{}{}",
            block.1["opcode"].as_str().unwrap(),
            rng.gen_range(0..99999999999999999u64)
        )
    };

    // TODO Remove this
    return Ok((String::from(function), start_type, name, warp));
}

/// Returns all stacks of blocks.
fn create_all_hats(
    blocks: &JsonValue,
    block_reference: &HashMap<&str, &str>,
    name: String,
) -> Result<String, String> {
    let mut contents: String = String::new();

    // Get all the custom blocks
    let mut custom_blocks: HashMap<String, String> = HashMap::new();
    for block in blocks.entries() {
        let cblock = handle_custom_block(block, blocks, block_reference);
        match cblock {
            Some((procode, func)) => {
                custom_blocks.insert(procode, func);
            }
            None => {}
        };
    }

    let custom_block_reference = custom_blocks.clone();
    // Expand all custom blocks in the definitions of the custom blocks
    for (_name, definition) in &mut custom_blocks {
        expand_custom_blocks(definition, &custom_block_reference);
    }

    for block in blocks.entries() {
        let hat = create_hat(block, blocks, block_reference, &custom_blocks);
        match hat {
            Ok((function, start_type, function_name, warp)) => {
                match warp{
                    false => contents.push_str(format!(
                    // "program.add_thread(Thread{{function:{},obj_index:Some(program.objects.len()),complete:false}});\n",
                    "async fn stack_{function_name}(sprite: Rc<Mutex<Sprite>>, stage: Rc<Mutex<Stage>>){{{function}}}
program.add_thread(Thread::new(stack_{function_name}({sprite_name}.clone(),Stage.clone())/*,Some(program.objects.len())*/,{start_type}));\n",
                    sprite_name = name,
                ) .as_str()),
                    true => contents.push_str(format!("fn stack_{function_name}(sprite:Rc<Mutex<Sprite>>,stage:Rc<Mutex<Stage>>){{{function}}}").as_str())
                }
            }
            Err(x) => {
                continue;
            }
        }
    }

    //return Err(String::from("Bad"));
    return Ok(format!("{}", contents));
}

/// Expand all the custom blocks in a function.
fn expand_custom_blocks(function: &mut String, custom_blocks: &HashMap<String, String>) {
    for (name, definition) in custom_blocks {
        *function = function.replacen(name, definition, 1);
    }
}

/// Get the variables from a target.
///
/// The string constructs a new HashMap with the variables
/// in it.
fn get_variables(target: &JsonValue) -> Result<String, &str> {
    // let mut to_return = String::from("HashMap::from([");
    let mut to_return = String::new();
    for (key, value) in target["variables"].entries() {
        // cloud variables are not supported
        if let Some(true) = value[2].as_bool() {
            return Err("Does not support cloud variables");
        }

        // if the value is a string, include quotation marks                    v        v
        if value[1].is_string() {
            to_return.push_str(&*format!(
                ".add_variable(String::from(\"{key}\"),(String::from(\"{name}\"),Value::from(\"{value}\")))\n",
                name = value[0],
                value = value[1]
            ))
        } else {
            //otherwise don't include qotation marks.
            to_return.push_str(&*format!(
                ".add_variable(String::from(\"{key}\"),(String::from(\"{name}\"),Value::from({value})))\n",
                name = value[0],
                value = value[1]
            ));
        }
    }

    // to_return.push_str("])");

    // return Ok(to_return);
    return Ok(to_return);
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

    let function = create_hat(block, blocks, block_reference, &HashMap::new()).unwrap();

    fs::write(filename, format!("{}\n\n\n{}", lib, function.0)).expect("Could not write file.");
}

/// Generate a new target(sprite or stage) from json.
fn generate_target(target: &JsonValue, block_reference: &HashMap<&str, &str>) -> String {
    // If the target is the stage
    if target["isStage"].as_bool().unwrap() {
        let function = create_all_hats(
            &target["blocks"],
            block_reference,
            target["name"].to_string(),
        )
        .unwrap();
        return format!(
            "/*let mut {name}=Rc::new(Mutex::new(Stage{{
                tempo:{tempo},
                video_state:{videoState},
                video_transparency:{videoTransparency},
                text_to_speech_language:String::from(\"{textToSpeechLanguage}\"),
                variables:{variables},
                costume:0,
                costumes:Vec::new(),
            }}));*/
            //let mut tempo={tempo};
            //let mut video_state={videoState};
            //let mut video_transparency={videoTransparency};
            //let mut text_to_speech_language=String::from(\"{textToSpeechLanguage}\");
            //let mut global_variables:HashMap<String,Value> =HashMap::new();
            //let mut currentCostume:usize=0;

            let mut {name}=Rc::new(Mutex::new(
                StageBuilder::new()
                    .tempo({tempo})
                    .video_state({videoState})
                    .video_transparency({videoTransparency})
                    {costume}
                    {variables}
                    .build()
            ));
            {function}
",
            name = target["name"],
            tempo = target["tempo"],
            videoState = VideoState::from_str(target["videoState"].as_str().unwrap())
                .unwrap()
                .to_str(),
            textToSpeechLanguage = target["textToSpeechLanguage"],
            videoTransparency = target["videoTransparency"],
            variables = get_variables(target).expect("There are no cloud variables"),
            costume = target_costumes(target),
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
        let function = create_all_hats(
            &target["blocks"],
            &block_reference,
            target["name"].to_string(),
        )
        .unwrap();

        return format!(
            "/*let mut {name}=Rc::new(Mutex::new(Sprite{{
                visible:{visible},
                x:{x}f32,
                y:{y}f32,
                size:{size}f32,
                direction:{direction}f32,
                draggable:{draggable},
                rotation_style:{rotationStyle},
                name:\"{name}\".to_string(),
                variables:{variables},
                costume:0,
                costumes:Vec::new(),
            }}));*/

            let mut {name} = Rc::new(Mutex::new(
                SpriteBuilder::new(\"{name}\".to_string())
                    .visible({visible})
                    .position({x}f32,{y}f32)
                    .direction({direction}f32)
                    .draggable({draggable})
                    .rotation_style({rotationStyle})
                    {costumes}
                    {variables}
                    .build()
            ));

            {function}
            program.add_object({name}.clone());
            //sprites.push({name}.clone());",
            name = target["name"],
            visible = target["visible"],
            x = target["x"],
            y = target["y"],
            size = target["size"],
            direction = target["direction"],
            variables = get_variables(target).expect("There should be no cloud variables"),
            draggable = target["draggable"],
            rotationStyle = RotationStyle::from_str(target["rotationStyle"].as_str().unwrap())
                .unwrap()
                .to_str(),
            costumes = target_costumes(target),
        );
    }
}

// struct blockstack
//
// type of hat

/// Fetch a scratch sb3 file.
fn fetch_sb3_file(url: String) -> String {
    // fetch the file,
    let response = ureq::get(&url).call().expect("Could not get file"); // TODO better error handling

    // create a new file,
    //let mut out=File::create("project.sv3").expect("Could not create file");
    // and copy the contents of the response to the new file.
    //response.copy_to(&mut out);
    // io::copy(&mut response,&mut out)?;
    return response.into_string().unwrap();
}

/// Get the project token for a scratch project.
fn fetch_project_token(id: u32) -> Result<String, &'static str> {
    let url = format!("https://api.scratch.mit.edu/projects/{id}");
    let text = match ureq::get(&url).call() {
        Ok(res) => res
            .into_string()
            .expect("Response can be converted to string"),
        Err(_) => return Err("Could not get from api"),
    };

    let json = json::parse(&text).or(Err("Cannot parse json"))?;

    let token = &json["project_token"];

    if token.is_null() {
        return Err("Token does not exist");
    } else {
        return Ok(token.to_string());
    }
}

/// Formats the given filename with rustfmt.
fn format_file(filename: String) -> io::Result<Output> {
    return Command::new("rustfmt").arg(filename).output();
    //.expect("Could not execute rustfmt");
}

fn create_project() -> Result<(), io::Error> {
    Command::new("cargo").arg("new").arg("output").output()?;

    let toml = "
    [package]
    name=\"output\"
    version=\"1.0.0\"
    edition=\"2021\"
    
    [dependencies]
    rand=\"0.8.5\"
    genawaiter=\"0.99.1\"
    piston = \"0.53.0\"
    piston2d-graphics = \"0.42.0\"
    pistoncore-glutin_window = \"0.69.0\"
    piston2d-opengl_graphics = \"0.81.0\"
    resvg = \"0.25.0\"
    ";

    fs::write("output/Cargo.toml", toml)?;

    return Ok(());
}

/// Download the assets for a target.
fn get_target_assets(target: &JsonValue) {
    // create the asset directory
    fs::create_dir_all(format!("output/assets/{}", target["name"])).unwrap();

    // iterate through all costumes
    for costume in target["costumes"].members() {
        let response = ureq::get(&format!(
            "https://assets.scratch.mit.edu/{}",
            costume["md5ext"]
        ))
        .call()
        .expect("Could not download asset file");

        let mut file = std::fs::File::create(format!(
            "output/assets/{}/{}.{}",
            target["name"], costume["name"], costume["dataFormat"]
        ))
        .unwrap();

        let mut content = response.into_reader();
        std::io::copy(&mut content, &mut file).unwrap();
    }
}

/// Get all the target costumes
fn target_costumes(target: &JsonValue) -> String {
    let mut to_return = String::new();
    let name = &target["name"];
    for costume in target["costumes"].members() {
        let costumename = &costume["name"];
        let format = &costume["dataFormat"];
        let stage_or_sprite;
        if target["isStage"].as_bool().unwrap() {
            stage_or_sprite = "stage";
        } else {
            stage_or_sprite = "sprite";
        }

        // TODO handle png files properly
        if format != "svg" {
            continue;
        }
        // to_return.push_str(&format!("program.add_costume_{stage_or_sprite}(
        //                                 Costume::new(PathBuf::from(\"assets/{name}/{costumename}.{format}\"),1.0).unwrap(),
        //                                 &mut {name}
        //                             );\n"));
        to_return.push_str(&format!(".add_costume(Costume::new(PathBuf::from(\"assets/{name}/{costumename}.{format}\"),1.0).unwrap())\n"))
    }

    return to_return;
}

/// Convert all the svg assets in a target to pngs.
fn convert_svg_png(target: &JsonValue) {
    use opengl_graphics::{CreateTexture, Format, Texture, TextureSettings};
    use resvg::tiny_skia::{Pixmap, Transform};
    use resvg::usvg::{FitTo, Options, Tree};

    let paths = fs::read_dir(format!("output/assets/{}", target["name"])).unwrap();

    for path in paths {
        let tree = Tree::from_str(
            &fs::read_to_string(path.unwrap().path()).unwrap(),
            &Options::default().to_ref(),
        )
        .unwrap();
        let fit_to = FitTo::Original;
        let transform = Transform::default();
        let mut pixmap = Pixmap::new(1, 1).unwrap();
        let pixmapmut = pixmap.as_mut();

        resvg::render(&tree, fit_to, transform, pixmapmut);

        let texture = Texture::create(
            &mut (),
            Format::Rgba8,
            pixmap.data(),
            [pixmap.width(), pixmap.height()],
            &TextureSettings::new(),
        );
    }
}
