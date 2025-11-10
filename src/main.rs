use std::{
    fs,
    error::Error,
    collections::HashMap,
    path::{ Path, PathBuf },
    process::{ Command, exit },
    io::{ self, Write },
    env::{ self, var },
};

//TODO: implement command history logging w/ timestamps
const PROMPT: &str = "#~ ";

fn main() {
    if let Err(e) = process_command("clr", Vec::new()) {
        eprintln!("failed to execute command 'clr': {e}");
    }
    io::stdout().flush().unwrap();

    loop {
        let raw_line = get_command();
        if raw_line.is_empty() { continue; }
        //TODO: potentially change to split_once(' ') and collect params more gracefully
        let line = raw_line.trim().split(' ').collect::<Vec<&str>>();

        let cmd = line.get(0).unwrap().to_owned();
        let params = if line.len() > 1 { line[1..].to_vec() } else { Vec::new() };

        match process_command(cmd, params) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("failed to execute command '{cmd}': {e}");
            }
        };

        io::stdout().flush().unwrap();
    }
}

fn get_command() -> String {
    print!("{}", PROMPT);
    io::stdout().flush().unwrap();

    let input = io::stdin();
    let mut command = String::new();
    input.read_line(&mut command).unwrap();

    io::stdout().flush().unwrap();

    command.trim().to_string()
}

fn process_command(cmd: &str, params: Vec<&str>) -> Result<(), Box<dyn Error>> {
    let raw_path = var("PATH")?;
    let mut path = env::split_paths(&raw_path)
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<String>>();

    let curr_dir = env::current_dir()?.to_string_lossy().into_owned();
    let home = if env::consts::OS == "windows" { "USERPROFILE" } else { "HOME" };
    path.push(curr_dir);

    let builtins: HashMap<&str, &str> = HashMap::from_iter(vec![
        ("echo", "[argument(s): message] print message to stdout"),
        ("this", "print the current location in the file tree"),
        ("move", "[argument(s): target] change location in file tree"),
        ("type", "[argument(s): file] print the location of an executable"),
        ("exec", "[argument(s): program, parameters (optional)] run an executable"),
        ("exit", "[argument(s): exit code (optional)] exit the shell"),
        ("PATH", "print every directory in the PATH environment variable"),
        ("clr", "clear the screen"),
        ("help", "[argument(s): shell command (optional)] show this screen or information about a command"),
    ]);

    match cmd {
        "move" => {
            if params.is_empty() {
                eprintln!("command failed 'move': no target directory provided");

                return Ok(())
            }

            let dir = params.get(0).unwrap().to_owned();

            if dir == "~" {
                env::set_current_dir(var(home)?)?;

                return Ok(())
            }

            let curr = env::current_dir()?;
            let target = Path::new(&dir);
            let mut target_buf = PathBuf::from(target);

            if target.is_relative() {
                target_buf = curr.join(target);
            }
            env::set_current_dir(target_buf)?;

            Ok(())
        },
        "this" => {
            let mut loc = env::current_dir()?;
            if params.get(0).is_some() {
                loc = loc.join(params.get(0).unwrap().to_owned());
            }

            let dirs = fs::read_dir(loc)?;

            for entry in dirs {
                let entry = entry?;
                let file_name = entry.file_name();
                let res = file_name.to_str().unwrap();

                if entry.file_type()?.is_dir() { print!("(D)-") } else { print!("(F)-") }
                println!("[{res}]")
            }

            Ok(())
        },
        "exit" => {
            io::stdout().flush()?;
            process_command("clr", Vec::new())?;

            if params.is_empty() {
                exit(0);
            }
            exit(params[0].parse::<i32>()?);
        },
        "help" => {
            if params.is_empty() {
                for (k, v) in builtins {
                    println!(" - {k:<5} : {v}");
                }
            } else {
                let target = params
                    .get(0)
                    .unwrap()
                    .to_owned();
                let text = builtins
                    .get(&target)
                    .unwrap_or_else(|| &"this shell command doesn't exist")
                    .to_owned();

                println!(" - {target:<5} : {text}");
            }
            Ok(())
        },
        "echo" => {
            let arg = params.join(" ");

            println!("{arg}");
            Ok(())
        },
        "clr" => {
            if env::consts::OS == "windows" {
                Command::new("cmd")
                    .args(&["/C", "cls"])
                    .status()?;
            } else {
                Command::new("clear")
                    .status()?;
            }

            Ok(())
        },
        "PATH" => {
            println!("{path:?}");
            Ok(())
        },
        "type" => {
            if let None = params.get(0) {
                eprintln!("command failed 'type': please provide an argument");
                return Ok(())
            };
            let arg = params.get(0).unwrap().to_owned();

            match arg.to_owned() {
                _ if builtins.contains_key(arg) => {
                    println!("{arg} is a shell builtin");
                },
                _ => {
                    let exec = search_for_exec(arg, path);

                    if let Some(file) = exec {
                        println!("{arg} is {file}");
                    } else {
                        eprintln!("{arg}: not found");
                    }
                }
            }

            Ok(())
        },
        "exec" => {
            if let None = params.get(0) {
                eprintln!("command failed 'exec': please provide an argument");

                return Ok(())
            }

            let exec_arg = params.get(0).unwrap().to_owned();
            let exec = search_for_exec(exec_arg, path);

            match exec {
                Some(val) => {
                    let mut command = Command::new(val);

                    for param in params[1..].iter() {
                        command.arg(param);
                    }
                    command.status()?;
                    println!();
                },
                None => {
                    eprintln!("{exec_arg}: command not found");
                }
            }

            Ok(())
        },
        _ => {
            eprintln!("{cmd}: not a shell command");
            Ok(())
        },
    }
}

fn search_for_exec(cmd: &str, paths: Vec<String>) -> Option<String> {
    for folder in paths {
        let arg_path;

        if env::consts::OS == "windows" {
            arg_path = if cmd.ends_with(".exe") {
                format!("{folder}\\{cmd}")
            } else {
                format!("{folder}\\{cmd}.exe")
            };
        } else {
            arg_path = format!("{folder}/{cmd}");
        }

        let res_path = Path::new(&arg_path);

        if res_path.exists() &&
            !res_path.metadata().unwrap().permissions().readonly() {
            return Some(arg_path);
        }
    }

    None
}